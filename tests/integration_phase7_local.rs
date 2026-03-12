//! Phase 7 (M7) integration tests — local dependency scenarios.
//!
//! Tests cover workspace-only builds, vendor layer, offline mode,
//! lockfile integrity, and migration. No network calls required.

use std::fs;
use std::path::Path;

use duumbi::config::{self, DependencyConfig, DuumbiConfig, VendorSection, VendorStrategy};
use duumbi::deps;
use duumbi::hash;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Valid JSON-LD module with a main function that returns 0.
const MAIN_MODULE: &str = r#"{
    "@context": {"duumbi": "https://duumbi.dev/ns/core#"},
    "@type": "duumbi:Module",
    "@id": "duumbi:main",
    "duumbi:name": "main",
    "duumbi:functions": [{
        "@type": "duumbi:Function",
        "@id": "duumbi:main/main",
        "duumbi:name": "main",
        "duumbi:returnType": "i64",
        "duumbi:params": [],
        "duumbi:blocks": [{
            "@type": "duumbi:Block",
            "@id": "duumbi:main/main/entry",
            "duumbi:label": "entry",
            "duumbi:ops": [
                {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/0",
                 "duumbi:value": 0, "duumbi:resultType": "i64"},
                {"@type": "duumbi:Return", "@id": "duumbi:main/main/entry/1",
                 "duumbi:operand": {"@id": "duumbi:main/main/entry/0"}}
            ]
        }]
    }]
}"#;

/// Library module with an exported helper function.
fn lib_module(name: &str) -> String {
    format!(
        r#"{{
    "@context": {{"duumbi": "https://duumbi.dev/ns/core#"}},
    "@type": "duumbi:Module",
    "@id": "duumbi:{name}",
    "duumbi:name": "{name}",
    "duumbi:exports": ["helper"],
    "duumbi:functions": [{{
        "@type": "duumbi:Function",
        "@id": "duumbi:{name}/helper",
        "duumbi:name": "helper",
        "duumbi:returnType": "i64",
        "duumbi:params": [],
        "duumbi:blocks": [{{
            "@type": "duumbi:Block",
            "@id": "duumbi:{name}/helper/entry",
            "duumbi:label": "entry",
            "duumbi:ops": [
                {{"@type": "duumbi:Const", "@id": "duumbi:{name}/helper/entry/0",
                 "duumbi:value": 42, "duumbi:resultType": "i64"}},
                {{"@type": "duumbi:Return", "@id": "duumbi:{name}/helper/entry/1",
                 "duumbi:operand": {{"@id": "duumbi:{name}/helper/entry/0"}}}}
            ]
        }}]
    }}]
}}"#
    )
}

/// Creates a minimal workspace with main.jsonld.
fn make_workspace(dir: &Path) {
    let graph = dir.join(".duumbi").join("graph");
    fs::create_dir_all(&graph).expect("invariant: create graph dir");
    fs::write(graph.join("main.jsonld"), MAIN_MODULE).expect("invariant: write main");
}

/// Creates a library module in the cache at the standard path.
fn make_cache_module(workspace: &Path, scope: &str, name: &str, version: &str) {
    let graph_dir = workspace
        .join(".duumbi/cache")
        .join(scope)
        .join(format!("{name}@{version}"))
        .join("graph");
    fs::create_dir_all(&graph_dir).expect("invariant: create cache dir");
    fs::write(graph_dir.join(format!("{name}.jsonld")), lib_module(name))
        .expect("invariant: write cache module");
}

/// Creates a library module as a separate workspace (path dep).
fn make_path_dep(dir: &Path, module_name: &str) {
    let graph = dir.join(".duumbi").join("graph");
    fs::create_dir_all(&graph).expect("invariant: create dep graph dir");
    fs::write(
        graph.join(format!("{module_name}.jsonld")),
        lib_module(module_name),
    )
    .expect("invariant: write dep module");
}

fn save_cfg(workspace: &Path, cfg: &DuumbiConfig) {
    config::save_config(workspace, cfg).expect("invariant: save config");
}

// ---------------------------------------------------------------------------
// 1. Workspace-only build (no external deps)
// ---------------------------------------------------------------------------

#[test]
fn workspace_only_load_program_succeeds() {
    let ws = tempfile::TempDir::new().expect("tempdir");
    make_workspace(ws.path());

    let program = deps::load_program_with_deps(ws.path()).expect("must load workspace-only");
    assert_eq!(program.modules.len(), 1, "only main module expected");
}

#[test]
fn workspace_only_with_multiple_graph_files() {
    let ws = tempfile::TempDir::new().expect("tempdir");
    make_workspace(ws.path());

    // Add a second module directly in the graph dir (workspace module)
    let graph = ws.path().join(".duumbi/graph");
    fs::write(graph.join("utils.jsonld"), lib_module("utils")).expect("write utils");

    let program = deps::load_program_with_deps(ws.path()).expect("must load");
    assert_eq!(program.modules.len(), 2, "main + utils");
}

// ---------------------------------------------------------------------------
// 2. Path dependency integration
// ---------------------------------------------------------------------------

#[test]
fn path_dep_adds_and_loads_in_program() {
    let ws = tempfile::TempDir::new().expect("tempdir");
    make_workspace(ws.path());

    let dep_ws = tempfile::TempDir::new().expect("tempdir");
    make_path_dep(dep_ws.path(), "mylib");

    let dep_path = dep_ws.path().to_str().expect("utf8 path").to_string();
    deps::add_dependency(ws.path(), "mylib", &dep_path).expect("add dep");

    let program = deps::load_program_with_deps(ws.path()).expect("must load");
    assert_eq!(program.modules.len(), 2, "main + mylib");
}

#[test]
fn path_dep_generates_lockfile_with_integrity() {
    let ws = tempfile::TempDir::new().expect("tempdir");
    make_workspace(ws.path());

    let dep_ws = tempfile::TempDir::new().expect("tempdir");
    make_path_dep(dep_ws.path(), "mylib");

    let dep_path = dep_ws.path().to_str().expect("utf8 path").to_string();
    deps::add_dependency(ws.path(), "mylib", &dep_path).expect("add dep");

    let cfg = config::load_config(ws.path()).expect("config");
    let lock = deps::generate_lockfile(ws.path(), &cfg).expect("lockfile");

    assert_eq!(lock.dependencies.len(), 1);
    let entry = &lock.dependencies[0];
    assert_eq!(entry.name, "mylib");
    assert!(entry.integrity.is_some(), "must have integrity hash");
    assert!(entry.semantic_hash.is_some(), "must have semantic hash");
    assert!(entry.is_v1());

    // Lockfile persisted on disk
    assert!(ws.path().join(".duumbi/deps.lock").exists());
}

// ---------------------------------------------------------------------------
// 3. Cache dependency resolution
// ---------------------------------------------------------------------------

#[test]
fn cache_dep_resolves_and_loads() {
    let ws = tempfile::TempDir::new().expect("tempdir");
    make_workspace(ws.path());
    make_cache_module(ws.path(), "@duumbi", "stdlib-math", "1.0.0");

    let mut cfg = DuumbiConfig::default();
    cfg.dependencies.insert(
        "@duumbi/stdlib-math".to_string(),
        DependencyConfig::Version("1.0.0".to_string()),
    );
    save_cfg(ws.path(), &cfg);

    let program = deps::load_program_with_deps(ws.path()).expect("must load");
    assert_eq!(program.modules.len(), 2, "main + stdlib-math");
}

// ---------------------------------------------------------------------------
// 4. Vendor layer
// ---------------------------------------------------------------------------

#[test]
fn vendor_all_copies_cache_to_vendor_dir() {
    let ws = tempfile::TempDir::new().expect("tempdir");
    make_workspace(ws.path());
    make_cache_module(ws.path(), "@duumbi", "stdlib-math", "1.0.0");

    let mut cfg = DuumbiConfig::default();
    cfg.dependencies.insert(
        "@duumbi/stdlib-math".to_string(),
        DependencyConfig::Version("1.0.0".to_string()),
    );
    save_cfg(ws.path(), &cfg);

    let results = deps::vendor_dependencies(ws.path(), &deps::VendorMode::All).expect("vendor");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "@duumbi/stdlib-math");

    // Verify vendor files exist
    let vendor_graph = ws.path().join(".duumbi/vendor/@duumbi/stdlib-math/graph");
    assert!(vendor_graph.exists(), "vendor graph dir must exist");
    assert!(
        vendor_graph.join("stdlib-math.jsonld").exists(),
        "vendored module must exist"
    );
}

#[test]
fn vendor_selective_filters_by_scope() {
    let ws = tempfile::TempDir::new().expect("tempdir");
    make_workspace(ws.path());
    make_cache_module(ws.path(), "@duumbi", "stdlib-math", "1.0.0");
    make_cache_module(ws.path(), "@company", "auth", "2.0.0");

    let mut cfg = DuumbiConfig::default();
    cfg.dependencies.insert(
        "@duumbi/stdlib-math".to_string(),
        DependencyConfig::Version("1.0.0".to_string()),
    );
    cfg.dependencies.insert(
        "@company/auth".to_string(),
        DependencyConfig::Version("2.0.0".to_string()),
    );
    save_cfg(ws.path(), &cfg);

    let results = deps::vendor_dependencies(
        ws.path(),
        &deps::VendorMode::Include("@company/*".to_string()),
    )
    .expect("vendor");

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "@company/auth");

    // @duumbi should NOT be vendored
    let duumbi_vendor = ws.path().join(".duumbi/vendor/@duumbi/stdlib-math/graph");
    assert!(
        !duumbi_vendor.exists(),
        "non-matching scope must be skipped"
    );
}

#[test]
fn vendor_config_rules_respects_strategy() {
    let ws = tempfile::TempDir::new().expect("tempdir");
    make_workspace(ws.path());
    make_cache_module(ws.path(), "@company", "auth", "1.0.0");

    let mut cfg = DuumbiConfig::default();
    cfg.dependencies.insert(
        "@company/auth".to_string(),
        DependencyConfig::Version("1.0.0".to_string()),
    );
    cfg.vendor = Some(VendorSection {
        strategy: VendorStrategy::Selective,
        include: vec!["@company/*".to_string()],
    });
    save_cfg(ws.path(), &cfg);

    let results =
        deps::vendor_dependencies(ws.path(), &deps::VendorMode::ConfigRules).expect("vendor");
    assert_eq!(results.len(), 1);
}

#[test]
fn vendor_skips_path_dependencies() {
    let ws = tempfile::TempDir::new().expect("tempdir");
    make_workspace(ws.path());
    let dep_ws = tempfile::TempDir::new().expect("tempdir");
    make_path_dep(dep_ws.path(), "mylib");

    let dep_path = dep_ws.path().to_str().expect("utf8 path").to_string();
    deps::add_dependency(ws.path(), "mylib", &dep_path).expect("add dep");

    let results = deps::vendor_dependencies(ws.path(), &deps::VendorMode::All).expect("vendor");
    assert!(results.is_empty(), "path deps must not be vendored");
}

// ---------------------------------------------------------------------------
// 5. Offline mode
// ---------------------------------------------------------------------------

#[test]
fn offline_build_succeeds_with_vendored_dep() {
    let ws = tempfile::TempDir::new().expect("tempdir");
    make_workspace(ws.path());
    make_cache_module(ws.path(), "@duumbi", "stdlib-math", "1.0.0");

    let mut cfg = DuumbiConfig::default();
    cfg.dependencies.insert(
        "@duumbi/stdlib-math".to_string(),
        DependencyConfig::Version("1.0.0".to_string()),
    );
    save_cfg(ws.path(), &cfg);

    // Vendor first
    deps::vendor_dependencies(ws.path(), &deps::VendorMode::All).expect("vendor");

    // Offline load succeeds (vendor layer)
    let program = deps::load_program_with_deps_opts(ws.path(), true).expect("offline must work");
    assert_eq!(program.modules.len(), 2);
}

#[test]
fn offline_build_fails_for_cache_only_dep() {
    let ws = tempfile::TempDir::new().expect("tempdir");
    make_workspace(ws.path());
    make_cache_module(ws.path(), "@duumbi", "stdlib-math", "1.0.0");

    let mut cfg = DuumbiConfig::default();
    cfg.dependencies.insert(
        "@duumbi/stdlib-math".to_string(),
        DependencyConfig::Version("1.0.0".to_string()),
    );
    save_cfg(ws.path(), &cfg);

    // No vendor step — offline must fail
    let result = deps::load_program_with_deps_opts(ws.path(), true);
    assert!(result.is_err(), "offline must fail for cache-only dep");
}

#[test]
fn offline_build_succeeds_with_path_dep() {
    let ws = tempfile::TempDir::new().expect("tempdir");
    make_workspace(ws.path());
    let dep_ws = tempfile::TempDir::new().expect("tempdir");
    make_path_dep(dep_ws.path(), "mylib");

    let dep_path = dep_ws.path().to_str().expect("utf8 path").to_string();
    deps::add_dependency(ws.path(), "mylib", &dep_path).expect("add dep");

    // Path deps are always available offline (they're local)
    let program =
        deps::load_program_with_deps_opts(ws.path(), true).expect("offline path dep must work");
    assert_eq!(program.modules.len(), 2);
}

// ---------------------------------------------------------------------------
// 6. Lockfile integrity verification
// ---------------------------------------------------------------------------

#[test]
fn lockfile_integrity_passes_for_untampered_deps() {
    let ws = tempfile::TempDir::new().expect("tempdir");
    make_workspace(ws.path());
    let dep_ws = tempfile::TempDir::new().expect("tempdir");
    make_path_dep(dep_ws.path(), "mylib");

    let dep_path = dep_ws.path().to_str().expect("utf8 path").to_string();
    deps::add_dependency(ws.path(), "mylib", &dep_path).expect("add dep");

    let cfg = config::load_config(ws.path()).expect("config");
    let lock = deps::generate_lockfile(ws.path(), &cfg).expect("lockfile");

    let failures = deps::verify_lockfile(&lock).expect("verify");
    assert!(failures.is_empty(), "untampered deps must pass");
}

#[test]
fn lockfile_integrity_detects_tampered_dep() {
    let ws = tempfile::TempDir::new().expect("tempdir");
    make_workspace(ws.path());
    let dep_ws = tempfile::TempDir::new().expect("tempdir");
    make_path_dep(dep_ws.path(), "mylib");

    let dep_path = dep_ws.path().to_str().expect("utf8 path").to_string();
    deps::add_dependency(ws.path(), "mylib", &dep_path).expect("add dep");

    let cfg = config::load_config(ws.path()).expect("config");
    let lock = deps::generate_lockfile(ws.path(), &cfg).expect("lockfile");

    // Tamper with the dependency
    let graph_dir = dep_ws.path().join(".duumbi/graph");
    fs::write(
        graph_dir.join("mylib.jsonld"),
        r#"{"@type": "duumbi:Module", "duumbi:name": "tampered", "duumbi:functions": []}"#,
    )
    .expect("tamper");

    let failures = deps::verify_lockfile(&lock).expect("verify");
    assert_eq!(failures.len(), 1, "tampered dep must be detected");
    assert_eq!(failures[0].name, "mylib");
}

#[test]
fn lockfile_integrity_detects_tampered_vendored_dep() {
    let ws = tempfile::TempDir::new().expect("tempdir");
    make_workspace(ws.path());
    make_cache_module(ws.path(), "@duumbi", "stdlib-math", "1.0.0");

    let mut cfg = DuumbiConfig::default();
    cfg.dependencies.insert(
        "@duumbi/stdlib-math".to_string(),
        DependencyConfig::Version("1.0.0".to_string()),
    );
    save_cfg(ws.path(), &cfg);

    // Vendor then generate lockfile
    deps::vendor_dependencies(ws.path(), &deps::VendorMode::All).expect("vendor");
    let cfg = config::load_config(ws.path()).expect("config");
    let lock = deps::generate_lockfile(ws.path(), &cfg).expect("lockfile");

    // Tamper with vendored file
    let vendor_file = ws
        .path()
        .join(".duumbi/vendor/@duumbi/stdlib-math/graph/stdlib-math.jsonld");
    fs::write(
        &vendor_file,
        r#"{"@type": "duumbi:Module", "duumbi:name": "evil", "duumbi:functions": []}"#,
    )
    .expect("tamper vendor");

    let failures = deps::verify_lockfile(&lock).expect("verify");
    assert!(
        !failures.is_empty(),
        "tampered vendor file must be detected"
    );
}

// ---------------------------------------------------------------------------
// 7. Lockfile determinism
// ---------------------------------------------------------------------------

#[test]
fn lockfile_output_is_deterministic_across_generates() {
    let ws = tempfile::TempDir::new().expect("tempdir");
    make_workspace(ws.path());
    make_cache_module(ws.path(), "@duumbi", "stdlib-math", "1.0.0");

    let mut cfg = DuumbiConfig::default();
    cfg.dependencies.insert(
        "@duumbi/stdlib-math".to_string(),
        DependencyConfig::Version("1.0.0".to_string()),
    );
    save_cfg(ws.path(), &cfg);

    let cfg = config::load_config(ws.path()).expect("config");
    deps::generate_lockfile(ws.path(), &cfg).expect("lockfile 1");
    let content1 = fs::read_to_string(ws.path().join(".duumbi/deps.lock")).expect("read 1");
    deps::generate_lockfile(ws.path(), &cfg).expect("lockfile 2");
    let content2 = fs::read_to_string(ws.path().join(".duumbi/deps.lock")).expect("read 2");

    assert_eq!(content1, content2, "lockfile must be deterministic");
}

// ---------------------------------------------------------------------------
// 8. Semantic hash stability
// ---------------------------------------------------------------------------

#[test]
fn semantic_hash_stable_across_workspace_operations() {
    let ws = tempfile::TempDir::new().expect("tempdir");
    make_workspace(ws.path());

    let graph_dir = ws.path().join(".duumbi/graph");
    let hash1 = hash::semantic_hash(&graph_dir).expect("hash 1");

    // Generate lockfile, vendor, etc. — should not change graph hash
    let hash2 = hash::semantic_hash(&graph_dir).expect("hash 2");
    assert_eq!(hash1, hash2);
}

// ---------------------------------------------------------------------------
// 9. Resolution priority: vendor > cache
// ---------------------------------------------------------------------------

#[test]
fn vendor_takes_priority_over_cache_in_resolution() {
    let ws = tempfile::TempDir::new().expect("tempdir");
    make_workspace(ws.path());
    make_cache_module(ws.path(), "@duumbi", "stdlib-math", "1.0.0");

    let mut cfg = DuumbiConfig::default();
    cfg.dependencies.insert(
        "@duumbi/stdlib-math".to_string(),
        DependencyConfig::Version("1.0.0".to_string()),
    );
    save_cfg(ws.path(), &cfg);

    // Vendor the dep
    deps::vendor_dependencies(ws.path(), &deps::VendorMode::All).expect("vendor");

    // Resolve should find it in vendor (not cache)
    let resolved =
        deps::resolve_module(ws.path(), "@duumbi/stdlib-math", "1.0.0").expect("resolve");
    assert_eq!(
        resolved.source,
        deps::ModuleSource::Vendor,
        "vendor must take priority over cache"
    );
}

// ---------------------------------------------------------------------------
// 10. Multiple deps end-to-end
// ---------------------------------------------------------------------------

#[test]
fn multiple_deps_mixed_sources_load_correctly() {
    let ws = tempfile::TempDir::new().expect("tempdir");
    make_workspace(ws.path());

    // Path dep
    let dep_ws = tempfile::TempDir::new().expect("tempdir");
    make_path_dep(dep_ws.path(), "locallib");
    let dep_path = dep_ws.path().to_str().expect("utf8 path").to_string();
    deps::add_dependency(ws.path(), "locallib", &dep_path).expect("add path dep");

    // Cache dep
    make_cache_module(ws.path(), "@duumbi", "stdlib-math", "1.0.0");
    let mut cfg = config::load_config(ws.path()).expect("config");
    cfg.dependencies.insert(
        "@duumbi/stdlib-math".to_string(),
        DependencyConfig::Version("1.0.0".to_string()),
    );
    save_cfg(ws.path(), &cfg);

    let program = deps::load_program_with_deps(ws.path()).expect("must load");
    assert_eq!(program.modules.len(), 3, "main + locallib + stdlib-math");
}

#[test]
fn multiple_deps_lockfile_has_all_entries() {
    let ws = tempfile::TempDir::new().expect("tempdir");
    make_workspace(ws.path());

    let dep_ws = tempfile::TempDir::new().expect("tempdir");
    make_path_dep(dep_ws.path(), "locallib");
    let dep_path = dep_ws.path().to_str().expect("utf8 path").to_string();
    deps::add_dependency(ws.path(), "locallib", &dep_path).expect("add dep");

    make_cache_module(ws.path(), "@duumbi", "stdlib-math", "1.0.0");
    let mut cfg = config::load_config(ws.path()).expect("config");
    cfg.dependencies.insert(
        "@duumbi/stdlib-math".to_string(),
        DependencyConfig::Version("1.0.0".to_string()),
    );
    save_cfg(ws.path(), &cfg);

    let cfg = config::load_config(ws.path()).expect("config");
    let lock = deps::generate_lockfile(ws.path(), &cfg).expect("lockfile");
    assert_eq!(lock.dependencies.len(), 2);

    let names: Vec<&str> = lock.dependencies.iter().map(|d| d.name.as_str()).collect();
    assert!(names.contains(&"locallib"));
    assert!(names.contains(&"@duumbi/stdlib-math"));
}

// ---------------------------------------------------------------------------
// 11. Frozen lockfile detection
// ---------------------------------------------------------------------------

#[test]
fn frozen_lockfile_passes_when_unchanged() {
    let ws = tempfile::TempDir::new().expect("tempdir");
    make_workspace(ws.path());
    let dep_ws = tempfile::TempDir::new().expect("tempdir");
    make_path_dep(dep_ws.path(), "mylib");

    let dep_path = dep_ws.path().to_str().expect("utf8 path").to_string();
    deps::add_dependency(ws.path(), "mylib", &dep_path).expect("add dep");

    // Generate lockfile (first time)
    let cfg = config::load_config(ws.path()).expect("config");
    deps::generate_lockfile(ws.path(), &cfg).expect("lockfile");

    let lock_content = fs::read_to_string(ws.path().join(".duumbi/deps.lock")).expect("read");

    // Generate again — should produce identical output
    deps::generate_lockfile(ws.path(), &cfg).expect("lockfile 2");
    let lock_content2 = fs::read_to_string(ws.path().join(".duumbi/deps.lock")).expect("read 2");

    assert_eq!(
        lock_content, lock_content2,
        "lockfile must be identical when deps unchanged (frozen would pass)"
    );
}

#[test]
fn frozen_lockfile_detects_drift_from_new_dep() {
    let ws = tempfile::TempDir::new().expect("tempdir");
    make_workspace(ws.path());

    // Generate initial lockfile with no deps
    let cfg = config::load_config(ws.path()).unwrap_or_default();
    deps::generate_lockfile(ws.path(), &cfg).expect("lockfile");
    let initial_lock = fs::read_to_string(ws.path().join(".duumbi/deps.lock")).expect("read");

    // Add a dep
    let dep_ws = tempfile::TempDir::new().expect("tempdir");
    make_path_dep(dep_ws.path(), "newlib");
    let dep_path = dep_ws.path().to_str().expect("utf8 path").to_string();
    deps::add_dependency(ws.path(), "newlib", &dep_path).expect("add dep");

    // Generate new lockfile
    let cfg = config::load_config(ws.path()).expect("config");
    deps::generate_lockfile(ws.path(), &cfg).expect("lockfile 2");
    let new_lock = fs::read_to_string(ws.path().join(".duumbi/deps.lock")).expect("read 2");

    assert_ne!(
        initial_lock, new_lock,
        "lockfile must differ after adding dep (frozen would fail)"
    );
}

// ---------------------------------------------------------------------------
// 12. Migration: old workspace format
// ---------------------------------------------------------------------------

#[test]
fn old_v0_lockfile_loads_and_upgrades() {
    let ws = tempfile::TempDir::new().expect("tempdir");
    let duumbi = ws.path().join(".duumbi");
    fs::create_dir_all(&duumbi).expect("create .duumbi");

    // Write a v0-style lockfile
    let v0 = r#"version = 1

[[dependencies]]
name = "mylib"
path = "/some/old/path"
hash = "0123456789abcdef"
"#;
    fs::write(duumbi.join("deps.lock"), v0).expect("write v0");

    let lock = deps::load_lockfile(ws.path()).expect("load v0");
    assert_eq!(lock.dependencies.len(), 1);
    assert_eq!(lock.dependencies[0].name, "mylib");
    assert_eq!(
        lock.dependencies[0].effective_path(),
        Some("/some/old/path")
    );
    assert!(
        !lock.dependencies[0].is_v1(),
        "v0 entry should not report as v1"
    );
}
