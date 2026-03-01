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

    let unique_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let tmp_dir = std::env::temp_dir().join(format!(
        "duumbi_phase4_{}_{}",
        std::process::id(),
        unique_id
    ));
    fs::create_dir_all(&tmp_dir).expect("invariant: must create tmp dir");

    // Write object files
    let mut obj_paths: Vec<std::path::PathBuf> = Vec::new();
    // Link main module first so its `main` symbol is found first
    for name in &["main", "math"] {
        if let Some(bytes) = objects.get(*name) {
            let obj_path = tmp_dir.join(format!("{name}.o"));
            fs::write(&obj_path, bytes).expect("invariant: must write object file");
            obj_paths.push(obj_path);
        }
    }

    // Compile runtime and link
    let runtime_o = compile_runtime_to(&tmp_dir);
    let binary = tmp_dir.join("output");

    let obj_refs: Vec<&Path> = obj_paths.iter().map(|p| p.as_path()).collect();
    linker::link_multi(&obj_refs, &runtime_o, &binary).expect("multi-module link must succeed");

    assert!(binary.exists(), "linked binary must exist");

    // Run the binary: double(21) = 42
    let output = Command::new(&binary)
        .output()
        .expect("invariant: compiled binary must be runnable");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        stdout.trim(),
        "42",
        "expected double(21)=42 printed, got '{}'",
        stdout.trim()
    );

    let exit_code = output
        .status
        .code()
        .expect("invariant: binary must have exit code");
    assert_eq!(exit_code, 42, "expected exit code 42, got {exit_code}");

    let _ = fs::remove_dir_all(&tmp_dir);
}
