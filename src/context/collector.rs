//! Node collector — executes traversal plans on workspace `.jsonld` files.
//!
//! Produces deduplicated JSON-LD fragments with priority ranking,
//! ready for budget fitting.

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use crate::context::ContextError;
use crate::context::analyzer::ProjectMap;
use crate::context::traversal::{StepKind, TraversalPlan};

/// A collected context fragment with priority.
#[derive(Debug, Clone)]
pub struct ContextFragment {
    /// The JSON-LD text fragment.
    pub text: String,

    /// Priority (lower = more important).
    pub priority: u32,

    /// Source module name.
    pub source_module: String,
}

/// Collected context nodes from traversal.
#[derive(Debug, Clone)]
pub struct ContextNodes {
    /// Collected fragments in priority order.
    pub fragments: Vec<ContextFragment>,
}

/// Executes a traversal plan against the workspace and collects context fragments.
///
/// Returns deduplicated fragments sorted by priority.
///
/// # Errors
///
/// Returns an error if workspace files cannot be read.
pub fn collect(
    workspace: &Path,
    plan: &TraversalPlan,
    project_map: &ProjectMap,
) -> Result<ContextNodes, ContextError> {
    let mut fragments = Vec::new();
    let mut seen_modules = HashSet::new();

    for step in &plan.steps {
        match &step.kind {
            StepKind::AllModuleSignatures => {
                for module in &project_map.modules {
                    if seen_modules.insert(format!("sig:{}", module.name)) {
                        let text = format_module_signatures(module);
                        fragments.push(ContextFragment {
                            text,
                            priority: step.priority,
                            source_module: module.name.clone(),
                        });
                    }
                }
            }

            StepKind::FullModule(name) => {
                if seen_modules.insert(format!("full:{name}"))
                    && let Some(text) = load_module_content(workspace, name)
                {
                    fragments.push(ContextFragment {
                        text,
                        priority: step.priority,
                        source_module: name.clone(),
                    });
                }
            }

            StepKind::MainModule => {
                if seen_modules.insert("full:main".to_string())
                    && let Some(text) = load_module_content(workspace, "main")
                {
                    fragments.push(ContextFragment {
                        text,
                        priority: step.priority,
                        source_module: "main".to_string(),
                    });
                }
            }

            StepKind::DependencySignatures => {
                // Scan cache/vendor for dependency modules
                for dir_name in &["cache", "vendor"] {
                    let dep_dir = workspace.join(format!(".duumbi/{dir_name}"));
                    if dep_dir.exists()
                        && let Ok(entries) = fs::read_dir(&dep_dir)
                    {
                        for entry in entries.flatten() {
                            let module_name = entry.file_name().to_string_lossy().to_string();
                            if seen_modules.insert(format!("dep:{module_name}")) {
                                // Find the module in project_map or extract from file
                                if let Some(module) =
                                    project_map.modules.iter().find(|m| m.name == module_name)
                                {
                                    let text = format_module_signatures(module);
                                    fragments.push(ContextFragment {
                                        text,
                                        priority: step.priority,
                                        source_module: module_name,
                                    });
                                }
                            }
                        }
                    }
                }
            }

            StepKind::Neighborhood { target, hops } => {
                // Include signatures of other modules as neighborhood context.
                // A full call-graph hop traversal would require parsing all edges;
                // for now we include all non-target module signatures, limited by
                // the hops parameter (hops=1: only modules sharing exported names,
                // hops>=2: all other modules).
                if seen_modules.insert(format!("neigh:{target}")) {
                    for module in &project_map.modules {
                        if module.name == *target {
                            continue;
                        }
                        // hops=1: only modules that share function names with the target
                        if *hops <= 1 {
                            let target_funcs: std::collections::HashSet<&str> = project_map
                                .modules
                                .iter()
                                .filter(|m| m.name == *target)
                                .flat_map(|m| m.functions.iter().map(|f| f.name.as_str()))
                                .collect();
                            let shares_func = module
                                .functions
                                .iter()
                                .any(|f| target_funcs.contains(f.name.as_str()));
                            if !shares_func {
                                continue;
                            }
                        }
                        if seen_modules.insert(format!("sig:{}", module.name)) {
                            let text = format_module_signatures(module);
                            fragments.push(ContextFragment {
                                text,
                                priority: step.priority,
                                source_module: module.name.clone(),
                            });
                        }
                    }
                }
            }

            StepKind::ErrorContext => {
                // Load recent errors from learning history
                let successes = crate::knowledge::learning::query_combined_successes(workspace, 5);
                for success in successes {
                    if !success.error_codes.is_empty() {
                        let text = format!(
                            "Past fix: '{}' resolved errors [{}] in module '{}' ({} ops, {} retries)",
                            success.request,
                            success.error_codes.join(", "),
                            success.module,
                            success.ops_count,
                            success.retry_count,
                        );
                        fragments.push(ContextFragment {
                            text,
                            priority: step.priority,
                            source_module: success.module.clone(),
                        });
                    }
                }
            }
        }
    }

    // Sort by priority
    fragments.sort_by_key(|f| f.priority);

    Ok(ContextNodes { fragments })
}

/// Formats a module's function signatures for context inclusion.
fn format_module_signatures(module: &crate::context::analyzer::ModuleInfo) -> String {
    let funcs: Vec<String> = module
        .functions
        .iter()
        .map(|f| {
            let params: Vec<String> = f.params.iter().map(|(n, t)| format!("{n}: {t}")).collect();
            format!("  {}({}) -> {}", f.name, params.join(", "), f.return_type)
        })
        .collect();
    format!("Module '{}':\n{}", module.name, funcs.join("\n"))
}

/// Loads the raw content of a module's `.jsonld` file.
fn load_module_content(workspace: &Path, module_name: &str) -> Option<String> {
    let graph_dir = workspace.join(".duumbi/graph");

    // Try direct file match
    let path = graph_dir.join(format!("{module_name}.jsonld"));
    if path.exists() {
        return fs::read_to_string(&path).ok();
    }

    // Try subdirectory (for multi-level module names like "calculator/ops")
    if module_name.contains('/') {
        let parts: Vec<&str> = module_name.split('/').collect();
        let last = parts.last()?;
        let dir_parts: Vec<&str> = parts[..parts.len() - 1].to_vec();
        let mut sub_path = graph_dir.clone();
        for part in dir_parts {
            sub_path = sub_path.join(part);
        }
        let file_path = sub_path.join(format!("{last}.jsonld"));
        if file_path.exists() {
            return fs::read_to_string(&file_path).ok();
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::analyzer::{FunctionSummary, ModuleInfo};
    use crate::context::traversal::{TraversalPlan, TraversalStep};
    use std::collections::HashMap;
    use tempfile::TempDir;

    fn setup_workspace(tmp: &TempDir) {
        let graph_dir = tmp.path().join(".duumbi/graph");
        fs::create_dir_all(&graph_dir).expect("mkdir");
        let main_json = serde_json::json!({
            "@context": { "duumbi": "https://duumbi.dev/ns/core#" },
            "@type": "duumbi:Module",
            "@id": "duumbi:main",
            "duumbi:name": "main",
            "duumbi:functions": [{
                "@type": "duumbi:Function",
                "@id": "duumbi:main/main",
                "duumbi:name": "main",
                "duumbi:returnType": "i64",
                "duumbi:blocks": []
            }]
        });
        fs::write(
            graph_dir.join("main.jsonld"),
            serde_json::to_string_pretty(&main_json).expect("serialize"),
        )
        .expect("write");
    }

    fn sample_map() -> ProjectMap {
        ProjectMap {
            modules: vec![ModuleInfo {
                name: "main".to_string(),
                functions: vec![FunctionSummary {
                    name: "main".to_string(),
                    params: Vec::new(),
                    return_type: "i64".to_string(),
                }],
                is_main: true,
            }],
            exports: HashMap::new(),
        }
    }

    #[test]
    fn collect_all_signatures() {
        let tmp = TempDir::new().expect("temp dir");
        setup_workspace(&tmp);
        let map = sample_map();
        let plan = TraversalPlan {
            steps: vec![TraversalStep {
                kind: StepKind::AllModuleSignatures,
                priority: 1,
                signatures_only: true,
            }],
        };
        let nodes = collect(tmp.path(), &plan, &map).expect("collect");
        assert_eq!(nodes.fragments.len(), 1);
        assert!(nodes.fragments[0].text.contains("main"));
    }

    #[test]
    fn collect_full_module() {
        let tmp = TempDir::new().expect("temp dir");
        setup_workspace(&tmp);
        let map = sample_map();
        let plan = TraversalPlan {
            steps: vec![TraversalStep {
                kind: StepKind::FullModule("main".to_string()),
                priority: 1,
                signatures_only: false,
            }],
        };
        let nodes = collect(tmp.path(), &plan, &map).expect("collect");
        assert_eq!(nodes.fragments.len(), 1);
        assert!(nodes.fragments[0].text.contains("duumbi:Module"));
    }

    #[test]
    fn collect_deduplicates() {
        let tmp = TempDir::new().expect("temp dir");
        setup_workspace(&tmp);
        let map = sample_map();
        let plan = TraversalPlan {
            steps: vec![
                TraversalStep {
                    kind: StepKind::AllModuleSignatures,
                    priority: 1,
                    signatures_only: true,
                },
                TraversalStep {
                    kind: StepKind::AllModuleSignatures,
                    priority: 2,
                    signatures_only: true,
                },
            ],
        };
        let nodes = collect(tmp.path(), &plan, &map).expect("collect");
        // Should not duplicate
        assert_eq!(nodes.fragments.len(), 1);
    }

    #[test]
    fn collect_empty_plan() {
        let tmp = TempDir::new().expect("temp dir");
        setup_workspace(&tmp);
        let map = sample_map();
        let plan = TraversalPlan { steps: Vec::new() };
        let nodes = collect(tmp.path(), &plan, &map).expect("collect");
        assert!(nodes.fragments.is_empty());
    }

    #[test]
    fn collect_sorted_by_priority() {
        let tmp = TempDir::new().expect("temp dir");
        setup_workspace(&tmp);
        let map = ProjectMap {
            modules: vec![
                ModuleInfo {
                    name: "main".to_string(),
                    functions: vec![FunctionSummary {
                        name: "main".to_string(),
                        params: Vec::new(),
                        return_type: "i64".to_string(),
                    }],
                    is_main: true,
                },
                ModuleInfo {
                    name: "ops".to_string(),
                    functions: vec![FunctionSummary {
                        name: "add".to_string(),
                        params: Vec::new(),
                        return_type: "i64".to_string(),
                    }],
                    is_main: false,
                },
            ],
            exports: HashMap::new(),
        };
        let plan = TraversalPlan {
            steps: vec![
                TraversalStep {
                    kind: StepKind::AllModuleSignatures,
                    priority: 3,
                    signatures_only: true,
                },
                TraversalStep {
                    kind: StepKind::MainModule,
                    priority: 1,
                    signatures_only: false,
                },
            ],
        };
        let nodes = collect(tmp.path(), &plan, &map).expect("collect");
        // MainModule (priority 1) should come first
        assert!(nodes.fragments[0].priority <= nodes.fragments.last().map_or(0, |f| f.priority));
    }
}
