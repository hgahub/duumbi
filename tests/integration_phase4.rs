//! Phase 4 integration tests.
//!
//! Tests multi-module compilation: two modules compile to two `.o` files
//! that link into a working binary.

use std::fs;
use std::path::Path;
use std::process::Command;

use duumbi::compiler::{linker, lowering};
use duumbi::graph::program::Program;

const RUNTIME_C_SOURCE: &str = include_str!("../runtime/duumbi_runtime.c");

/// Writes the embedded runtime C source to a temp file and compiles it.
fn compile_runtime_to(tmp_dir: &Path) -> std::path::PathBuf {
    let runtime_c = tmp_dir.join("duumbi_runtime.c");
    fs::write(&runtime_c, RUNTIME_C_SOURCE).expect("invariant: must write runtime C");
    let runtime_o = tmp_dir.join("duumbi_runtime.o");
    linker::compile_runtime(&runtime_c, &runtime_o).expect("invariant: runtime must compile");
    runtime_o
}

/// Sets up a two-module workspace in a temp dir using the fixtures.
///
/// Returns a `TempDir` that contains `.duumbi/graph/main.jsonld` and
/// `.duumbi/graph/math.jsonld`.
fn setup_two_module_workspace() -> tempfile::TempDir {
    let ws = tempfile::TempDir::new().expect("invariant: tempdir must be creatable");
    let graph_dir = ws.path().join(".duumbi").join("graph");
    fs::create_dir_all(&graph_dir).expect("invariant: must create graph dir");

    let fixture_dir = Path::new("tests/fixtures/multi_module");
    for name in &["main.jsonld", "math.jsonld"] {
        let src = fixture_dir.join(name);
        let dst = graph_dir.join(name);
        fs::copy(&src, &dst).expect("invariant: fixture file must be copyable");
    }
    ws
}

/// Builds a program from object bytes and runs it, returning stdout and exit code.
///
/// Uses a `TempDir` for guaranteed unique, isolated temp space even under
/// parallel test execution.
fn link_and_run(objects: &std::collections::HashMap<String, Vec<u8>>) -> (String, i32) {
    let tmp = tempfile::TempDir::new().expect("invariant: must create tmp dir");
    let tmp_dir = tmp.path();

    // Write all objects — main first
    let mut all_names: Vec<&str> = objects.keys().map(|s| s.as_str()).collect();
    // Ensure 'main' is linked first for symbol visibility
    all_names.sort_by_key(|n| if *n == "main" { 0 } else { 1 });

    let mut obj_paths = Vec::new();
    for name in &all_names {
        if let Some(bytes) = objects.get(*name) {
            let path = tmp_dir.join(format!("{name}.o"));
            fs::write(&path, bytes).expect("invariant: write obj");
            obj_paths.push(path);
        }
    }

    let runtime_o = compile_runtime_to(tmp_dir);
    let binary = tmp_dir.join("output");
    let obj_refs: Vec<&Path> = obj_paths.iter().map(|p| p.as_path()).collect();
    linker::link_multi(&obj_refs, &runtime_o, &binary).expect("link must succeed");

    let output = Command::new(&binary)
        .output()
        .expect("invariant: binary must run");

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let exit_code = output.status.code().unwrap_or(-1);
    // tmp dir automatically cleaned up when dropped
    (stdout, exit_code)
}

// ---------------------------------------------------------------------------
// Existing multi-module fixture tests
// ---------------------------------------------------------------------------

#[test]
fn phase4_two_module_program_compiles_to_two_objects() {
    let ws = setup_two_module_workspace();
    let program = Program::load(ws.path()).expect("must load two-module program");

    let objects =
        lowering::compile_program(&program).expect("multi-module compilation must succeed");

    assert_eq!(objects.len(), 2, "expected 2 object files (main, math)");
    assert!(objects.contains_key("main"), "must have 'main' object");
    assert!(objects.contains_key("math"), "must have 'math' object");

    // Both objects must be valid native object files
    for (mod_name, bytes) in &objects {
        assert!(
            !bytes.is_empty(),
            "object for '{mod_name}' must not be empty"
        );
        let is_macho = bytes.len() >= 4
            && (bytes[0..4] == [0xCF, 0xFA, 0xED, 0xFE] || bytes[0..4] == [0xFE, 0xED, 0xFA, 0xCF]);
        let is_elf = bytes.len() >= 4 && bytes[0..4] == [0x7F, 0x45, 0x4C, 0x46];
        assert!(
            is_macho || is_elf,
            "object for '{mod_name}' must be valid Mach-O or ELF"
        );
    }
}

#[test]
fn phase4_two_module_program_links_and_runs() {
    let ws = setup_two_module_workspace();
    let program = Program::load(ws.path()).expect("must load two-module program");
    let objects =
        lowering::compile_program(&program).expect("multi-module compilation must succeed");
    let (stdout, exit_code) = link_and_run(&objects);

    assert_eq!(stdout, "42", "expected double(21)=42 printed");
    assert_eq!(exit_code, 42, "expected exit code 42");
}

// ---------------------------------------------------------------------------
// Stdlib math import: math.abs(-5) = 5
// ---------------------------------------------------------------------------

const STDLIB_MATH_JSONLD: &str = include_str!("../stdlib/math.jsonld");

/// Main module that calls math.abs(-5), prints the result, and returns it.
const ABS_MAIN_JSONLD: &str = r#"{
  "@context": {"duumbi": "https://duumbi.dev/ns/core#"},
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
        {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/0",
          "duumbi:value": -5, "duumbi:resultType": "i64"},
        {"@type": "duumbi:Call", "@id": "duumbi:main/main/entry/1",
          "duumbi:function": "abs",
          "duumbi:args": [{"@id": "duumbi:main/main/entry/0"}],
          "duumbi:resultType": "i64"},
        {"@type": "duumbi:Print", "@id": "duumbi:main/main/entry/2",
          "duumbi:operand": {"@id": "duumbi:main/main/entry/1"}},
        {"@type": "duumbi:Return", "@id": "duumbi:main/main/entry/3",
          "duumbi:operand": {"@id": "duumbi:main/main/entry/1"}}
      ]
    }]
  }]
}"#;

#[test]
fn phase4_stdlib_math_abs_produces_correct_output() {
    let ws = tempfile::TempDir::new().expect("tempdir");
    let graph_dir = ws.path().join(".duumbi").join("graph");
    fs::create_dir_all(&graph_dir).expect("create graph dir");
    fs::write(graph_dir.join("main.jsonld"), ABS_MAIN_JSONLD).expect("write main");
    fs::write(graph_dir.join("math.jsonld"), STDLIB_MATH_JSONLD).expect("write math");

    let program = Program::load(ws.path()).expect("program with math stdlib must load");
    let objects = lowering::compile_program(&program).expect("must compile");

    let (stdout, exit_code) = link_and_run(&objects);
    assert_eq!(stdout, "5", "abs(-5) must print 5");
    assert_eq!(exit_code, 5, "abs(-5) must exit with code 5");
}

// ---------------------------------------------------------------------------
// Lockfile determinism
// ---------------------------------------------------------------------------

#[test]
fn phase4_lockfile_is_deterministic() {
    use duumbi::config::DuumbiConfig;
    use duumbi::deps::generate_lockfile;

    let ws = tempfile::TempDir::new().expect("tempdir");
    // Set up math as a dep workspace
    let dep_ws = tempfile::TempDir::new().expect("dep tempdir");
    let dep_graph = dep_ws.path().join(".duumbi").join("graph");
    fs::create_dir_all(&dep_graph).expect("create dep graph dir");
    fs::write(dep_graph.join("math.jsonld"), STDLIB_MATH_JSONLD).expect("write math");

    // Create workspace .duumbi/ for the lockfile
    fs::create_dir_all(ws.path().join(".duumbi")).expect("create .duumbi");

    let mut config = DuumbiConfig::default();
    config.dependencies.insert(
        "math".to_string(),
        duumbi::config::DependencyConfig {
            path: dep_ws.path().to_str().expect("utf8").to_string(),
        },
    );

    let lock1 = generate_lockfile(ws.path(), &config).expect("lockfile 1");
    let lock2 = generate_lockfile(ws.path(), &config).expect("lockfile 2");

    assert_eq!(lock1.dependencies.len(), 1);
    assert_eq!(
        lock1.dependencies[0].hash, lock2.dependencies[0].hash,
        "lockfile hash must be deterministic"
    );
}

// ---------------------------------------------------------------------------
// Error case: unresolved cross-module reference (E010)
// ---------------------------------------------------------------------------

#[test]
fn phase4_unresolved_cross_module_ref_produces_e010() {
    use duumbi::errors::codes;
    use duumbi::graph::program::{Program, ProgramError};

    // main.jsonld calls "missing_fn" which doesn't exist in any module
    let main_with_bad_call = r#"{
  "@context": {"duumbi": "https://duumbi.dev/ns/core#"},
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
        {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/0",
          "duumbi:value": 1, "duumbi:resultType": "i64"},
        {"@type": "duumbi:Call", "@id": "duumbi:main/main/entry/1",
          "duumbi:function": "missing_fn",
          "duumbi:args": [{"@id": "duumbi:main/main/entry/0"}],
          "duumbi:resultType": "i64"},
        {"@type": "duumbi:Return", "@id": "duumbi:main/main/entry/2",
          "duumbi:operand": {"@id": "duumbi:main/main/entry/1"}}
      ]
    }]
  }]
}"#;

    let ws = tempfile::TempDir::new().expect("tempdir");
    let graph_dir = ws.path().join(".duumbi").join("graph");
    fs::create_dir_all(&graph_dir).expect("create graph dir");
    fs::write(graph_dir.join("main.jsonld"), main_with_bad_call).expect("write main");

    let errors = Program::load(ws.path()).expect_err("must fail on unresolved ref");
    assert!(
        errors.iter().any(|e| matches!(
            e,
            ProgramError::UnresolvedCrossModuleRef { function, code, .. }
                if function == "missing_fn" && *code == codes::E010_UNRESOLVED_CROSS_MODULE
        )),
        "expected E010 for 'missing_fn', got: {errors:?}"
    );
}

// ---------------------------------------------------------------------------
// Error case: missing dependency workspace
// ---------------------------------------------------------------------------

#[test]
fn phase4_missing_dep_workspace_returns_error() {
    use duumbi::deps::add_dependency;

    let ws = tempfile::TempDir::new().expect("tempdir");
    fs::create_dir_all(ws.path().join(".duumbi").join("graph")).expect("create graph dir");

    let result = add_dependency(ws.path(), "ghost", "/nonexistent/path/xyz");
    assert!(result.is_err(), "adding a non-existent dep path must error");
}

// ---------------------------------------------------------------------------
// M4 Kill criterion: init → add stdlib math → compile abs(-7) → binary prints 7
// ---------------------------------------------------------------------------

/// M4 Kill criterion:
/// Simulates: `duumbi init` → user writes a 2-module program (main + math) →
/// `duumbi build && duumbi run` produces correct output.
///
/// Concretely: a user program calls `math.abs(-7)`, the binary prints `7`
/// and exits with code 7.
#[test]
fn phase4_m4_kill_criterion_init_to_binary() {
    // Simulate `duumbi init`: create .duumbi/graph/ structure + math stdlib
    let ws = tempfile::TempDir::new().expect("tempdir");
    let graph_dir = ws.path().join(".duumbi").join("graph");
    fs::create_dir_all(&graph_dir).expect("create graph dir");

    // stdlib math module (same as what `duumbi init` writes to .duumbi/stdlib/math/)
    fs::write(graph_dir.join("math.jsonld"), STDLIB_MATH_JSONLD).expect("write math");

    // User-written program: abs(-7) = 7
    let user_program = r#"{
  "@context": {"duumbi": "https://duumbi.dev/ns/core#"},
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
        {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/0",
          "duumbi:value": -7, "duumbi:resultType": "i64"},
        {"@type": "duumbi:Call", "@id": "duumbi:main/main/entry/1",
          "duumbi:function": "abs",
          "duumbi:args": [{"@id": "duumbi:main/main/entry/0"}],
          "duumbi:resultType": "i64"},
        {"@type": "duumbi:Print", "@id": "duumbi:main/main/entry/2",
          "duumbi:operand": {"@id": "duumbi:main/main/entry/1"}},
        {"@type": "duumbi:Return", "@id": "duumbi:main/main/entry/3",
          "duumbi:operand": {"@id": "duumbi:main/main/entry/1"}}
      ]
    }]
  }]
}"#;
    fs::write(graph_dir.join("main.jsonld"), user_program).expect("write user main");

    let program = Program::load(ws.path()).expect("program must load");
    assert_eq!(program.modules.len(), 2, "main + math");

    let objects = lowering::compile_program(&program).expect("must compile");
    let (stdout, exit_code) = link_and_run(&objects);

    assert_eq!(stdout, "7", "abs(-7) must print 7");
    assert_eq!(exit_code, 7, "abs(-7) must exit with code 7");
}
