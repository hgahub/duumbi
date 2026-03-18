//! Project analyzer that builds a [`ProjectMap`] from workspace files.
//!
//! Scans `.duumbi/graph/` and dependency directories to build a complete
//! picture of all modules, their functions, and exports.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::context::ContextError;

/// Summary of the entire project structure.
#[derive(Debug, Clone)]
pub struct ProjectMap {
    /// All modules found in the workspace.
    pub modules: Vec<ModuleInfo>,

    /// Function name → module name mapping for quick lookup.
    pub exports: HashMap<String, String>,
}

/// Information about a single module.
#[derive(Debug, Clone)]
pub struct ModuleInfo {
    /// Module name (e.g. "main", "calculator/ops").
    pub name: String,

    /// Functions defined in this module.
    pub functions: Vec<FunctionSummary>,

    /// Whether this is the main module.
    pub is_main: bool,
}

/// Summary of a function's signature (no body details).
#[derive(Debug, Clone)]
pub struct FunctionSummary {
    /// Function name.
    pub name: String,

    /// Parameter names and types.
    pub params: Vec<(String, String)>,

    /// Return type.
    pub return_type: String,
}

/// Scans the workspace and builds a [`ProjectMap`].
///
/// Reads `.jsonld` files from `.duumbi/graph/` and extracts module/function
/// information. Also scans vendor and cache directories.
///
/// # Errors
///
/// Returns an error if the graph directory cannot be read.
pub fn analyze_workspace(workspace: &Path) -> Result<ProjectMap, ContextError> {
    let graph_dir = workspace.join(".duumbi/graph");
    let mut modules = Vec::new();
    let mut exports = HashMap::new();

    // Scan workspace modules
    if graph_dir.exists() {
        scan_graph_directory(&graph_dir, &mut modules, &mut exports)?;
    }

    // Scan vendor modules
    let vendor_dir = workspace.join(".duumbi/vendor");
    if vendor_dir.exists() {
        scan_dependency_modules(&vendor_dir, &mut modules, &mut exports)?;
    }

    // Scan cache modules
    let cache_dir = workspace.join(".duumbi/cache");
    if cache_dir.exists() {
        scan_dependency_modules(&cache_dir, &mut modules, &mut exports)?;
    }

    Ok(ProjectMap { modules, exports })
}

/// Formats a human-readable summary of the project map.
///
/// Used for prompt enrichment — shows available modules and their function
/// signatures so the LLM can use existing functions via `Call`.
#[must_use]
pub fn format_module_summary(project_map: &ProjectMap) -> String {
    let mut lines = Vec::new();
    for module in &project_map.modules {
        let funcs: Vec<String> = module
            .functions
            .iter()
            .map(|f| {
                let params: Vec<String> = f
                    .params
                    .iter()
                    .map(|(name, typ)| format!("{name}: {typ}"))
                    .collect();
                format!("  {}({}) -> {}", f.name, params.join(", "), f.return_type)
            })
            .collect();
        let marker = if module.is_main { " (main)" } else { "" };
        lines.push(format!("[{}{}]", module.name, marker));
        for func_line in funcs {
            lines.push(func_line);
        }
    }
    lines.join("\n")
}

/// Scans a graph directory for `.jsonld` files and extracts module info.
///
/// Recursively scans subdirectories for multi-level module names.
fn scan_graph_directory(
    graph_dir: &Path,
    modules: &mut Vec<ModuleInfo>,
    exports: &mut HashMap<String, String>,
) -> Result<(), ContextError> {
    let entries =
        fs::read_dir(graph_dir).map_err(|e| ContextError::Io(format!("reading graph dir: {e}")))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            // Recurse into module subdirectories
            scan_graph_directory(&path, modules, exports)?;
        } else if path.extension().is_some_and(|ext| ext == "jsonld")
            && let Ok(content) = fs::read_to_string(&path)
            && let Ok(value) = serde_json::from_str::<serde_json::Value>(&content)
            && let Some(info) = extract_module_info(&value)
        {
            for func in &info.functions {
                exports.insert(func.name.clone(), info.name.clone());
            }
            modules.push(info);
        }
    }

    Ok(())
}

/// Scans vendor/cache dependency directories.
fn scan_dependency_modules(
    dep_dir: &Path,
    modules: &mut Vec<ModuleInfo>,
    exports: &mut HashMap<String, String>,
) -> Result<(), ContextError> {
    if let Ok(entries) = fs::read_dir(dep_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                // Look for graph/ subdirectory
                let graph_sub = path.join("graph");
                if graph_sub.exists()
                    && let Ok(jsonld_entries) = fs::read_dir(&graph_sub)
                {
                    for jsonld_entry in jsonld_entries.flatten() {
                        let jsonld_path = jsonld_entry.path();
                        if jsonld_path.extension().is_some_and(|ext| ext == "jsonld")
                            && let Ok(content) = fs::read_to_string(&jsonld_path)
                            && let Ok(value) = serde_json::from_str::<serde_json::Value>(&content)
                            && let Some(info) = extract_module_info(&value)
                        {
                            for func in &info.functions {
                                exports.insert(func.name.clone(), info.name.clone());
                            }
                            modules.push(info);
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

/// Extracts module info from a JSON-LD module value.
fn extract_module_info(value: &serde_json::Value) -> Option<ModuleInfo> {
    let module_type = value.get("@type")?.as_str()?;
    if module_type != "duumbi:Module" {
        return None;
    }

    let name = value.get("duumbi:name")?.as_str()?.to_string();
    let is_main = name == "main";

    let functions = value
        .get("duumbi:functions")?
        .as_array()?
        .iter()
        .filter_map(extract_function_summary)
        .collect();

    Some(ModuleInfo {
        name,
        functions,
        is_main,
    })
}

/// Extracts a function summary from a JSON-LD function value.
fn extract_function_summary(func: &serde_json::Value) -> Option<FunctionSummary> {
    let name = func.get("duumbi:name")?.as_str()?.to_string();
    let return_type = func
        .get("duumbi:returnType")
        .and_then(|v| v.as_str())
        .unwrap_or("void")
        .to_string();

    let params = func
        .get("duumbi:params")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|p| {
                    let pname = p.get("duumbi:name")?.as_str()?.to_string();
                    let ptype = p.get("duumbi:paramType")?.as_str()?.to_string();
                    Some((pname, ptype))
                })
                .collect()
        })
        .unwrap_or_default();

    Some(FunctionSummary {
        name,
        params,
        return_type,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::TempDir;

    fn create_module_jsonld(workspace: &Path, filename: &str, module_json: &serde_json::Value) {
        let graph_dir = workspace.join(".duumbi/graph");
        fs::create_dir_all(&graph_dir).expect("mkdir");
        let path = graph_dir.join(filename);
        fs::write(
            path,
            serde_json::to_string_pretty(module_json).expect("serialize"),
        )
        .expect("write");
    }

    fn minimal_module() -> serde_json::Value {
        json!({
            "@context": { "duumbi": "https://duumbi.dev/ns/core#" },
            "@type": "duumbi:Module",
            "@id": "duumbi:main",
            "duumbi:name": "main",
            "duumbi:functions": [{
                "@type": "duumbi:Function",
                "@id": "duumbi:main/main",
                "duumbi:name": "main",
                "duumbi:returnType": "i64",
                "duumbi:blocks": [{
                    "@type": "duumbi:Block",
                    "@id": "duumbi:main/main/entry",
                    "duumbi:label": "entry",
                    "duumbi:ops": [
                        { "@type": "duumbi:Const", "@id": "duumbi:main/main/entry/0", "duumbi:value": 0, "duumbi:resultType": "i64" },
                        { "@type": "duumbi:Return", "@id": "duumbi:main/main/entry/1", "duumbi:operand": { "@id": "duumbi:main/main/entry/0" } }
                    ]
                }]
            }]
        })
    }

    #[test]
    fn analyze_empty_workspace() {
        let tmp = TempDir::new().expect("temp dir");
        fs::create_dir_all(tmp.path().join(".duumbi/graph")).expect("mkdir");
        let map = analyze_workspace(tmp.path()).expect("analyze");
        assert!(map.modules.is_empty());
    }

    #[test]
    fn analyze_single_module() {
        let tmp = TempDir::new().expect("temp dir");
        create_module_jsonld(tmp.path(), "main.jsonld", &minimal_module());

        let map = analyze_workspace(tmp.path()).expect("analyze");
        assert_eq!(map.modules.len(), 1);
        assert_eq!(map.modules[0].name, "main");
        assert!(map.modules[0].is_main);
        assert_eq!(map.modules[0].functions.len(), 1);
        assert_eq!(map.modules[0].functions[0].name, "main");
    }

    #[test]
    fn analyze_multi_module() {
        let tmp = TempDir::new().expect("temp dir");
        create_module_jsonld(tmp.path(), "main.jsonld", &minimal_module());

        let ops_module = json!({
            "@context": { "duumbi": "https://duumbi.dev/ns/core#" },
            "@type": "duumbi:Module",
            "@id": "duumbi:ops",
            "duumbi:name": "ops",
            "duumbi:functions": [{
                "@type": "duumbi:Function",
                "@id": "duumbi:ops/add",
                "duumbi:name": "add",
                "duumbi:returnType": "i64",
                "duumbi:params": [
                    { "duumbi:name": "a", "duumbi:paramType": "i64" },
                    { "duumbi:name": "b", "duumbi:paramType": "i64" }
                ],
                "duumbi:blocks": []
            }]
        });
        create_module_jsonld(tmp.path(), "ops.jsonld", &ops_module);

        let map = analyze_workspace(tmp.path()).expect("analyze");
        assert_eq!(map.modules.len(), 2);
        assert!(map.exports.contains_key("add"));
        assert_eq!(map.exports["add"], "ops");
    }

    #[test]
    fn format_module_summary_output() {
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
                        params: vec![
                            ("a".to_string(), "i64".to_string()),
                            ("b".to_string(), "i64".to_string()),
                        ],
                        return_type: "i64".to_string(),
                    }],
                    is_main: false,
                },
            ],
            exports: HashMap::new(),
        };

        let summary = format_module_summary(&map);
        assert!(summary.contains("[main (main)]"));
        assert!(summary.contains("[ops]"));
        assert!(summary.contains("add(a: i64, b: i64) -> i64"));
    }

    #[test]
    fn extract_function_with_params() {
        let func_json = json!({
            "@type": "duumbi:Function",
            "@id": "duumbi:main/multiply",
            "duumbi:name": "multiply",
            "duumbi:returnType": "i64",
            "duumbi:params": [
                { "duumbi:name": "x", "duumbi:paramType": "i64" },
                { "duumbi:name": "y", "duumbi:paramType": "i64" }
            ]
        });

        let summary = extract_function_summary(&func_json).expect("extract");
        assert_eq!(summary.name, "multiply");
        assert_eq!(summary.params.len(), 2);
        assert_eq!(summary.return_type, "i64");
    }

    #[test]
    fn analyze_no_duumbi_dir() {
        let tmp = TempDir::new().expect("temp dir");
        let map = analyze_workspace(tmp.path()).expect("analyze");
        assert!(map.modules.is_empty());
    }
}
