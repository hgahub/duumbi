//! Multi-module program loading and cross-module linking.
//!
//! Discovers all `.jsonld` module files in a workspace, builds per-module
//! semantic graphs, and validates cross-module `Call` references against
//! the combined export table.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use thiserror::Error;

use crate::errors::codes;
use crate::graph::{SemanticGraph, builder};
use crate::parser;
use crate::types::{FunctionName, ModuleName, Op};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// A compiled multi-module program.
///
/// Holds a per-module semantic graph for every `.jsonld` file discovered in
/// the workspace, plus the combined cross-module export table.
#[allow(dead_code)] // Fields consumed by compiler in upcoming phases
#[derive(Debug)]
pub struct Program {
    /// Per-module semantic graphs, keyed by module name.
    pub modules: HashMap<ModuleName, SemanticGraph>,
    /// Cross-module export table: exported function name → owning module.
    pub exports: HashMap<FunctionName, ModuleName>,
}

/// Errors produced while loading a multi-module program.
#[allow(dead_code)] // Consumed by the compiler CLI in upcoming phase (#59)
#[derive(Debug, Error)]
pub enum ProgramError {
    /// A `.jsonld` file could not be read or parsed.
    #[error("Failed to load '{path}': {reason}")]
    LoadFailed {
        /// Path to the failing file or directory.
        path: String,
        /// Reason for the failure.
        reason: String,
    },

    /// Graph construction errors from a single module.
    #[error("Graph error in module '{module}': {error}")]
    GraphError {
        /// Name of the module that failed.
        module: String,
        /// The underlying graph error.
        error: crate::graph::GraphError,
    },

    /// A `Call` op references a function that is not exported by any loaded module.
    #[error(
        "[{code}] Unresolved cross-module reference: \
         '{function}' called from module '{from_module}' is not exported by any loaded module"
    )]
    UnresolvedCrossModuleRef {
        /// Error code (E010).
        code: &'static str,
        /// The callee function name.
        function: String,
        /// The module containing the unresolved call.
        from_module: String,
    },
}

// ---------------------------------------------------------------------------
// Program implementation
// ---------------------------------------------------------------------------

#[allow(dead_code)] // Called by compiler CLI in upcoming phase (#59)
impl Program {
    /// Discovers and loads all `.jsonld` modules from `<workspace>/.duumbi/graph/`.
    ///
    /// **Loading steps:**
    /// 1. Scan `<workspace>/.duumbi/graph/` for `*.jsonld` files.
    /// 2. Parse each file into a `ModuleAst` and collect `duumbi:exports`.
    /// 3. Build per-module semantic graphs, passing other modules' exports as
    ///    known externals so cross-module calls are not flagged as orphan refs.
    /// 4. Validate that all cross-module `Call` ops resolve to an entry in the
    ///    combined export table; report [`ProgramError::UnresolvedCrossModuleRef`]
    ///    (E010) for any that do not.
    ///
    /// Returns all accumulated errors rather than stopping at the first failure.
    ///
    /// # Errors
    ///
    /// Returns a non-empty `Vec<ProgramError>` if any module fails to load,
    /// parse, build, or validate.
    pub fn load(workspace: &Path) -> Result<Self, Vec<ProgramError>> {
        let graph_dir = workspace.join(".duumbi").join("graph");
        let mut errors: Vec<ProgramError> = Vec::new();

        // Step 1: discover and parse all .jsonld files
        let entries = match fs::read_dir(&graph_dir) {
            Ok(e) => e,
            Err(e) => {
                return Err(vec![ProgramError::LoadFailed {
                    path: graph_dir.display().to_string(),
                    reason: e.to_string(),
                }]);
            }
        };

        let mut asts = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("jsonld") {
                continue;
            }

            let source = match fs::read_to_string(&path) {
                Ok(s) => s,
                Err(e) => {
                    errors.push(ProgramError::LoadFailed {
                        path: path.display().to_string(),
                        reason: e.to_string(),
                    });
                    continue;
                }
            };

            match parser::parse_jsonld(&source) {
                Ok(ast) => asts.push(ast),
                Err(e) => errors.push(ProgramError::LoadFailed {
                    path: path.display().to_string(),
                    reason: e.to_string(),
                }),
            }
        }

        if !errors.is_empty() {
            return Err(errors);
        }

        // Step 2: collect the combined export table (fn_name → module_name)
        let all_exports: HashMap<String, String> = asts
            .iter()
            .flat_map(|ast| {
                ast.exports
                    .iter()
                    .map(|fn_name| (fn_name.clone(), ast.name.0.clone()))
            })
            .collect();

        // Step 3: build per-module graphs without intra-module Call validation.
        // Cross-module calls are validated in step 4 using the export table.
        let mut modules: HashMap<ModuleName, SemanticGraph> = HashMap::new();

        for ast in &asts {
            match builder::build_graph_no_call_check(ast) {
                Ok(sg) => {
                    modules.insert(ast.name.clone(), sg);
                }
                Err(graph_errors) => {
                    for err in graph_errors {
                        errors.push(ProgramError::GraphError {
                            module: ast.name.0.clone(),
                            error: err,
                        });
                    }
                }
            }
        }

        if !errors.is_empty() {
            return Err(errors);
        }

        // Step 4: cross-module call validation (E010)
        // For each Call op, if the callee is not local and not exported → E010
        for (module_name, sg) in &modules {
            let local_fns: HashSet<&FunctionName> = sg.functions.iter().map(|f| &f.name).collect();

            for node in sg.graph.node_weights() {
                if let Op::Call { function } = &node.op {
                    let callee = FunctionName(function.clone());
                    if !local_fns.contains(&callee) && !all_exports.contains_key(function.as_str())
                    {
                        errors.push(ProgramError::UnresolvedCrossModuleRef {
                            code: codes::E010_UNRESOLVED_CROSS_MODULE,
                            function: function.clone(),
                            from_module: module_name.0.clone(),
                        });
                    }
                }
            }
        }

        if !errors.is_empty() {
            return Err(errors);
        }

        // Build the typed export table from all_exports
        let exports: HashMap<FunctionName, ModuleName> = all_exports
            .into_iter()
            .map(|(fn_name, mod_name)| (FunctionName(fn_name), ModuleName(mod_name)))
            .collect();

        Ok(Program { modules, exports })
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;

    /// Creates a minimal valid module JSON-LD string with the given name and
    /// optional list of exported function names.
    fn make_module(name: &str, exports: &[&str]) -> String {
        let exports_json = if exports.is_empty() {
            String::new()
        } else {
            let items: Vec<String> = exports.iter().map(|e| format!("\"{e}\"")).collect();
            format!(",\n    \"duumbi:exports\": [{}]", items.join(", "))
        };
        format!(
            r#"{{
    "@context": {{"duumbi": "https://duumbi.dev/ns/core#"}},
    "@type": "duumbi:Module",
    "@id": "duumbi:{name}",
    "duumbi:name": "{name}"{exports_json},
    "duumbi:functions": [{{
        "@type": "duumbi:Function",
        "@id": "duumbi:{name}/main",
        "duumbi:name": "main",
        "duumbi:returnType": "i64",
        "duumbi:blocks": [{{
            "@type": "duumbi:Block",
            "@id": "duumbi:{name}/main/entry",
            "duumbi:label": "entry",
            "duumbi:ops": [
                {{"@type": "duumbi:Const", "@id": "duumbi:{name}/main/entry/0",
                  "duumbi:value": 0, "duumbi:resultType": "i64"}},
                {{"@type": "duumbi:Return", "@id": "duumbi:{name}/main/entry/1",
                  "duumbi:operand": {{"@id": "duumbi:{name}/main/entry/0"}}}}
            ]
        }}]
    }}]
}}"#
        )
    }

    /// Creates a module that calls an external function `callee`.
    fn make_module_with_call(name: &str, callee: &str, exports: &[&str]) -> String {
        let exports_json = if exports.is_empty() {
            String::new()
        } else {
            let items: Vec<String> = exports.iter().map(|e| format!("\"{e}\"")).collect();
            format!(",\n    \"duumbi:exports\": [{}]", items.join(", "))
        };
        format!(
            r#"{{
    "@context": {{"duumbi": "https://duumbi.dev/ns/core#"}},
    "@type": "duumbi:Module",
    "@id": "duumbi:{name}",
    "duumbi:name": "{name}"{exports_json},
    "duumbi:functions": [{{
        "@type": "duumbi:Function",
        "@id": "duumbi:{name}/main",
        "duumbi:name": "main",
        "duumbi:returnType": "i64",
        "duumbi:blocks": [{{
            "@type": "duumbi:Block",
            "@id": "duumbi:{name}/main/entry",
            "duumbi:label": "entry",
            "duumbi:ops": [
                {{"@type": "duumbi:Const", "@id": "duumbi:{name}/main/entry/0",
                  "duumbi:value": 1, "duumbi:resultType": "i64"}},
                {{
                    "@type": "duumbi:Call",
                    "@id": "duumbi:{name}/main/entry/1",
                    "duumbi:function": "{callee}",
                    "duumbi:args": [{{"@id": "duumbi:{name}/main/entry/0"}}],
                    "duumbi:resultType": "i64"
                }},
                {{"@type": "duumbi:Return", "@id": "duumbi:{name}/main/entry/2",
                  "duumbi:operand": {{"@id": "duumbi:{name}/main/entry/1"}}}}
            ]
        }}]
    }}]
}}"#
        )
    }

    fn write_workspace(files: &[(&str, &str)]) -> tempfile::TempDir {
        let dir = tempfile::TempDir::new().expect("tempdir");
        let graph_dir = dir.path().join(".duumbi").join("graph");
        fs::create_dir_all(&graph_dir).expect("create graph dir");
        for (filename, content) in files {
            let path = graph_dir.join(filename);
            let mut f = fs::File::create(&path).expect("create file");
            f.write_all(content.as_bytes()).expect("write");
        }
        dir
    }

    #[test]
    fn single_module_program_loads_successfully() {
        let module = make_module("main", &[]);
        let ws = write_workspace(&[("main.jsonld", &module)]);
        let program = Program::load(ws.path()).expect("must load");
        assert_eq!(program.modules.len(), 1);
        assert!(
            program
                .modules
                .contains_key(&ModuleName("main".to_string()))
        );
    }

    #[test]
    fn two_module_program_with_valid_cross_call() {
        // app calls "helper" which is exported by math
        let app = make_module_with_call("app", "helper", &[]);

        // Wait — "math" doesn't actually define "helper", it just calls it.
        // A proper math module that defines and exports "helper".
        let math_with_helper = r#"{
    "@context": {"duumbi": "https://duumbi.dev/ns/core#"},
    "@type": "duumbi:Module",
    "@id": "duumbi:math",
    "duumbi:name": "math",
    "duumbi:exports": ["helper", "main"],
    "duumbi:functions": [
        {
            "@type": "duumbi:Function",
            "@id": "duumbi:math/main",
            "duumbi:name": "main",
            "duumbi:returnType": "i64",
            "duumbi:blocks": [{
                "@type": "duumbi:Block",
                "@id": "duumbi:math/main/entry",
                "duumbi:label": "entry",
                "duumbi:ops": [
                    {"@type": "duumbi:Const", "@id": "duumbi:math/main/entry/0",
                      "duumbi:value": 0, "duumbi:resultType": "i64"},
                    {"@type": "duumbi:Return", "@id": "duumbi:math/main/entry/1",
                      "duumbi:operand": {"@id": "duumbi:math/main/entry/0"}}
                ]
            }]
        },
        {
            "@type": "duumbi:Function",
            "@id": "duumbi:math/helper",
            "duumbi:name": "helper",
            "duumbi:returnType": "i64",
            "duumbi:blocks": [{
                "@type": "duumbi:Block",
                "@id": "duumbi:math/helper/entry",
                "duumbi:label": "entry",
                "duumbi:ops": [
                    {"@type": "duumbi:Const", "@id": "duumbi:math/helper/entry/0",
                      "duumbi:value": 42, "duumbi:resultType": "i64"},
                    {"@type": "duumbi:Return", "@id": "duumbi:math/helper/entry/1",
                      "duumbi:operand": {"@id": "duumbi:math/helper/entry/0"}}
                ]
            }]
        }
    ]
}"#;

        let ws = write_workspace(&[("math.jsonld", math_with_helper), ("app.jsonld", &app)]);

        let program = Program::load(ws.path()).expect("must load two-module program");
        assert_eq!(program.modules.len(), 2);
        assert!(
            program
                .exports
                .contains_key(&FunctionName("helper".to_string()))
        );
        assert_eq!(
            program.exports[&FunctionName("helper".to_string())],
            ModuleName("math".to_string())
        );
    }

    #[test]
    fn cross_module_call_to_unexported_function_produces_e010() {
        // app calls "secret" which exists in math but is NOT exported
        let math = make_module("math", &[]); // exports nothing
        let app = make_module_with_call("app", "secret", &[]);

        let ws = write_workspace(&[("math.jsonld", &math), ("app.jsonld", &app)]);

        let errors = Program::load(ws.path()).expect_err("must fail on unresolved ref");
        assert!(
            errors.iter().any(|e| matches!(
                e,
                ProgramError::UnresolvedCrossModuleRef { function, code, .. }
                    if function == "secret" && *code == codes::E010_UNRESOLVED_CROSS_MODULE
            )),
            "expected E010 for unresolved 'secret', got: {errors:?}"
        );
    }

    #[test]
    fn missing_graph_directory_returns_load_error() {
        let dir = tempfile::TempDir::new().expect("tempdir");
        // No .duumbi/graph/ directory created
        let errors = Program::load(dir.path()).expect_err("must fail on missing dir");
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, ProgramError::LoadFailed { .. })),
            "expected LoadFailed error"
        );
    }

    #[test]
    fn non_jsonld_files_in_graph_dir_are_ignored() {
        let module = make_module("main", &[]);
        let ws = write_workspace(&[
            ("main.jsonld", &module),
            ("README.md", "# docs"),
            ("notes.txt", "some notes"),
        ]);
        let program = Program::load(ws.path()).expect("must load, ignoring non-jsonld files");
        assert_eq!(program.modules.len(), 1);
    }
}
