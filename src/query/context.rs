//! Read-only context assembly for Query mode.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::context::analyzer;
use crate::knowledge::store::KnowledgeStore;
use crate::knowledge::types::KnowledgeNode;
use crate::query::sources::SourceRef;
use crate::session::PersistentTurn;

/// Context assembled for a query request.
#[derive(Debug, Clone)]
pub struct QueryContext {
    /// Text passed to the LLM.
    pub text: String,
    /// Sources used to assemble the context.
    pub sources: Vec<SourceRef>,
}

/// Options for read-only query context assembly.
#[derive(Debug, Clone)]
pub struct QueryContextOptions {
    /// Currently visible module, if known.
    pub visible_module: Option<String>,
    /// Current C4 level, if known.
    pub c4_level: Option<String>,
    /// Recent session history.
    pub session_turns: Vec<PersistentTurn>,
}

impl QueryContextOptions {
    /// Creates empty query context options.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            visible_module: None,
            c4_level: None,
            session_turns: Vec::new(),
        }
    }
}

/// Assembles read-only workspace context for Query mode.
pub fn assemble_query_context(
    workspace: &Path,
    options: &QueryContextOptions,
) -> crate::query::engine::Result<QueryContext> {
    let mut parts = Vec::new();
    let mut sources = Vec::new();

    let project_map = analyzer::analyze_workspace(workspace)
        .map_err(crate::query::engine::QueryError::Context)?;
    let module_files = collect_module_file_index(workspace);
    if !project_map.modules.is_empty() {
        parts.push(format!(
            "Workspace modules:\n{}",
            analyzer::format_module_summary(&project_map)
        ));
        sources.push(SourceRef::WorkspaceSummary);

        for module in &project_map.modules {
            sources.push(SourceRef::GraphModule {
                module: module.name.clone(),
                path: module_files
                    .get(&module.name)
                    .cloned()
                    .or_else(|| module_path(workspace, &module.name))
                    .unwrap_or_else(|| workspace.join(".duumbi/graph/__invalid__")),
            });
        }
    }

    if let Some(module) = options.visible_module.as_deref().filter(|m| !m.is_empty()) {
        parts.push(format!("Visible module: {module}"));
        if !is_safe_module_name(module) {
            parts.push("Visible module was ignored because its name is invalid.".to_string());
        } else {
            let path = module_files
                .get(module)
                .cloned()
                .or_else(|| module_path(workspace, module));
            if let Some(path) = path
                && let Ok(source) = fs::read_to_string(&path)
            {
                if let Ok(parsed) = crate::parser::parse_jsonld(&source)
                    && let Ok(graph) = crate::graph::builder::build_graph_no_call_check(&parsed)
                {
                    parts.push(format!(
                        "Visible module graph description:\n{}",
                        crate::graph::describe::describe_to_string(&graph)
                    ));
                }
                parts.push(format!(
                    "Visible module JSON-LD excerpt:\n{}",
                    truncate_chars(&source, 6000)
                ));
            }
        }
    }

    if let Some(c4_level) = options.c4_level.as_deref().filter(|l| !l.is_empty()) {
        parts.push(format!("Visible C4 level: {c4_level}"));
    }

    let intent_lines = collect_intent_summaries(workspace, &mut sources);
    if !intent_lines.is_empty() {
        parts.push(format!("Active intents:\n{}", intent_lines.join("\n")));
    }

    if !options.session_turns.is_empty() {
        let session_lines = options
            .session_turns
            .iter()
            .rev()
            .take(5)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .enumerate()
            .map(|(offset, turn)| {
                let index = options.session_turns.len().saturating_sub(5) + offset;
                sources.push(SourceRef::SessionTurn { index });
                format!(
                    "{}. [{}] {} -> {}",
                    index + 1,
                    turn.task_type,
                    turn.request,
                    turn.summary
                )
            })
            .collect::<Vec<_>>();
        parts.push(format!(
            "Recent session turns:\n{}",
            session_lines.join("\n")
        ));
    }

    let store = KnowledgeStore::open_existing(workspace);
    let knowledge = store.load_all();
    if !knowledge.is_empty() {
        let lines = knowledge
            .iter()
            .take(8)
            .map(format_knowledge_node)
            .collect::<Vec<_>>();
        for node in knowledge.iter().take(8) {
            sources.push(SourceRef::KnowledgeNode {
                id: node.id().to_string(),
                node_type: node_type(node).to_string(),
            });
        }
        parts.push(format!("Knowledge highlights:\n{}", lines.join("\n")));
    }

    if parts.is_empty() {
        parts.push("No DUUMBI workspace context was found.".to_string());
    }

    Ok(QueryContext {
        text: truncate_chars(&parts.join("\n\n"), 24000),
        sources,
    })
}

fn collect_intent_summaries(workspace: &Path, sources: &mut Vec<SourceRef>) -> Vec<String> {
    let intents_dir = crate::intent::intents_dir(workspace);
    let entries = match fs::read_dir(&intents_dir) {
        Ok(entries) => entries,
        Err(_) => return Vec::new(),
    };

    entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            if !path
                .extension()
                .is_some_and(|ext| ext == "yaml" || ext == "yml")
            {
                return None;
            }
            let slug = path.file_stem()?.to_string_lossy().to_string();
            let content = fs::read_to_string(&path).ok()?;
            let spec = serde_yaml::from_str::<crate::intent::spec::IntentSpec>(&content).ok()?;
            sources.push(SourceRef::Intent {
                slug: slug.clone(),
                path,
            });
            Some(format!("- {slug}: {} [{:?}]", spec.intent, spec.status))
        })
        .collect()
}

fn module_path(workspace: &Path, module: &str) -> Option<PathBuf> {
    if !is_safe_module_name(module) {
        return None;
    }
    if module == "main" || module == "app/main" {
        Some(workspace.join(".duumbi/graph/main.jsonld"))
    } else {
        Some(
            workspace
                .join(".duumbi/graph")
                .join(format!("{module}.jsonld")),
        )
    }
}

fn is_safe_module_name(module: &str) -> bool {
    !module.is_empty()
        && !module.contains("..")
        && !module.contains('\\')
        && !module.starts_with('/')
        && !module.ends_with('/')
        && !module.split('/').any(str::is_empty)
        && module
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '/' || c == '-' || c == '_')
}

fn collect_module_file_index(workspace: &Path) -> HashMap<String, PathBuf> {
    let mut modules = HashMap::new();
    collect_modules_from_dir(&workspace.join(".duumbi/graph"), &mut modules, 0, true);
    collect_modules_from_dir(&workspace.join(".duumbi/vendor"), &mut modules, 0, false);
    collect_modules_from_dir(&workspace.join(".duumbi/cache"), &mut modules, 0, false);
    modules
}

fn collect_modules_from_dir(
    dir: &Path,
    modules: &mut HashMap<String, PathBuf>,
    depth: u32,
    direct_graph_dir: bool,
) {
    if depth > 6 {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if !direct_graph_dir && path.file_name().is_some_and(|name| name == "graph") {
                collect_modules_from_dir(&path, modules, depth + 1, true);
            }
            collect_modules_from_dir(&path, modules, depth + 1, direct_graph_dir);
        } else if direct_graph_dir
            && path.extension().is_some_and(|ext| ext == "jsonld")
            && let Some(module) = module_name_from_jsonld(&path)
        {
            modules.entry(module).or_insert(path);
        }
    }
}

fn module_name_from_jsonld(path: &Path) -> Option<String> {
    let content = fs::read_to_string(path).ok()?;
    let value = serde_json::from_str::<serde_json::Value>(&content).ok()?;
    (value.get("@type")?.as_str()? == "duumbi:Module")
        .then(|| value.get("duumbi:name")?.as_str().map(str::to_string))?
}

fn format_knowledge_node(node: &KnowledgeNode) -> String {
    match node {
        KnowledgeNode::Success(record) => {
            format!("- success: {} ({})", record.request, record.task_type)
        }
        KnowledgeNode::Decision(record) => format!("- decision: {}", record.decision),
        KnowledgeNode::Pattern(record) => {
            format!("- pattern: {} - {}", record.name, record.description)
        }
    }
}

fn node_type(node: &KnowledgeNode) -> &'static str {
    match node {
        KnowledgeNode::Success(_) => "success",
        KnowledgeNode::Decision(_) => "decision",
        KnowledgeNode::Pattern(_) => "pattern",
    }
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    let mut truncated = value.chars().take(max_chars).collect::<String>();
    truncated.push_str("\n[truncated]");
    truncated
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::SourceRef;
    use chrono::Utc;
    use serde_json::json;

    fn write_module(path: &Path, module: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("module parent");
        }
        let value = json!({
            "@context": { "duumbi": "https://duumbi.dev/ns/core#" },
            "@type": "duumbi:Module",
            "@id": format!("duumbi:{module}"),
            "duumbi:name": module,
            "duumbi:functions": [{
                "@type": "duumbi:Function",
                "@id": format!("duumbi:{module}/main"),
                "duumbi:name": "main",
                "duumbi:returnType": "i64",
                "duumbi:blocks": []
            }]
        });
        fs::write(path, serde_json::to_string_pretty(&value).expect("json")).expect("write module");
    }

    #[test]
    fn query_context_does_not_create_knowledge_directories() {
        let temp = tempfile::tempdir().expect("test tempdir");

        assemble_query_context(temp.path(), &QueryContextOptions::empty())
            .expect("query context should assemble for empty workspace");

        assert!(!temp.path().join(".duumbi/knowledge").exists());
    }

    #[test]
    fn truncate_chars_adds_marker_when_over_budget() {
        assert_eq!(truncate_chars("abcdef", 3), "abc\n[truncated]");
    }

    #[test]
    fn visible_module_rejects_path_traversal_names() {
        let temp = tempfile::tempdir().expect("test tempdir");
        fs::create_dir_all(temp.path().join(".duumbi")).expect("duumbi dir");
        fs::write(temp.path().join(".duumbi/secret.jsonld"), "secret").expect("secret");
        let options = QueryContextOptions {
            visible_module: Some("../secret".to_string()),
            c4_level: None,
            session_turns: Vec::new(),
        };

        let context =
            assemble_query_context(temp.path(), &options).expect("query context should assemble");

        assert!(context.text.contains("Visible module: ../secret"));
        assert!(context.text.contains("ignored because its name is invalid"));
        assert!(!context.text.contains("Visible module JSON-LD excerpt"));
    }

    #[test]
    fn graph_module_sources_use_dependency_file_paths() {
        let temp = tempfile::tempdir().expect("test tempdir");
        let dep_path = temp
            .path()
            .join(".duumbi/vendor/@scope/pkg/graph/dep.jsonld");
        write_module(&dep_path, "dep");

        let context = assemble_query_context(temp.path(), &QueryContextOptions::empty())
            .expect("query context should assemble");

        assert!(context.sources.iter().any(|source| {
            matches!(
                source,
                SourceRef::GraphModule { module, path }
                    if module == "dep" && path == &dep_path
            )
        }));
    }

    #[test]
    fn session_turn_sources_keep_original_indices() {
        let temp = tempfile::tempdir().expect("test tempdir");
        let session_turns = (0..7)
            .map(|index| crate::session::PersistentTurn {
                request: format!("request {index}"),
                summary: format!("summary {index}"),
                timestamp: Utc::now(),
                task_type: "Query".to_string(),
            })
            .collect::<Vec<_>>();
        let options = QueryContextOptions {
            visible_module: None,
            c4_level: None,
            session_turns,
        };

        let context =
            assemble_query_context(temp.path(), &options).expect("query context should assemble");
        let indices = context
            .sources
            .iter()
            .filter_map(|source| match source {
                SourceRef::SessionTurn { index } => Some(*index),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(indices, vec![2, 3, 4, 5, 6]);
    }
}
