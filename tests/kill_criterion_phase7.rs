//! Phase 7 Kill Criterion Validation
//!
//! Four criteria that must ALL pass for Phase 7 to be considered complete:
//!
//! 1. **Deterministic semantic hash** — same module in two temp dirs → identical hash
//! 2. **Publish + Install** — publish to registry, install in clean workspace, verify
//! 3. **Vendor + Offline** — vendor deps, build offline succeeds
//! 4. **Lockfile deterministic** — generate lockfile twice → byte-identical output
//!
//! Uses an embedded `duumbi-registry` test server (in-memory SQLite, random port).

use std::collections::HashMap;
use std::fs;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;

use duumbi::config::{
    DependencyConfig, DuumbiConfig, VendorSection, VendorStrategy, WorkspaceSection,
};
use duumbi::deps;
use duumbi::hash;
use duumbi::registry::client::{RegistryClient, RegistryCredential};

use duumbi_registry::{AppState, build_app, db::Database, storage::Storage};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Valid JSON-LD module: main function returns 42.
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
                 "duumbi:value": 42, "duumbi:resultType": "i64"},
                {"@type": "duumbi:Return", "@id": "duumbi:main/main/entry/1",
                 "duumbi:operand": {"@id": "duumbi:main/main/entry/0"}}
            ]
        }]
    }]
}"#;

/// Library module: helper function returns 7.
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
                 "duumbi:value": 7, "duumbi:resultType": "i64"}},
                {{"@type": "duumbi:Return", "@id": "duumbi:{name}/helper/entry/1",
                 "duumbi:operand": {{"@id": "duumbi:{name}/helper/entry/0"}}}}
            ]
        }}]
    }}]
}}"#
    )
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_workspace(dir: &Path) {
    let graph = dir.join(".duumbi").join("graph");
    fs::create_dir_all(&graph).expect("create graph dir");
    fs::write(graph.join("main.jsonld"), MAIN_MODULE).expect("write main");
}

fn make_publishable_module(dir: &Path, name: &str, version: &str) {
    let duumbi_dir = dir.join(".duumbi");
    let graph = duumbi_dir.join("graph");
    fs::create_dir_all(&graph).expect("create graph dir");
    fs::write(graph.join(format!("{name}.jsonld")), lib_module(name)).expect("write module");

    let manifest = format!(
        r#"[module]
name = "@test/{name}"
version = "{version}"
description = "Test module for kill criterion"

[exports]
functions = ["helper"]
"#
    );
    fs::write(duumbi_dir.join("manifest.toml"), manifest).expect("write manifest");
}

fn save_cfg(workspace: &Path, cfg: &DuumbiConfig) {
    duumbi::config::save_config(workspace, cfg).expect("save config");
}

/// Starts an embedded test server, returns (base_url, token).
async fn start_test_server() -> (String, String, tempfile::TempDir) {
    let tmp = tempfile::TempDir::new().expect("temp dir");

    let database = Database::open(":memory:").expect("in-memory db");
    database.migrate().expect("migration");

    let token = "duu_kill_criterion_token";
    database
        .create_token("testuser", token)
        .expect("create token");

    let storage = Storage::new(tmp.path().join("modules").to_str().unwrap()).expect("storage");

    let state = Arc::new(AppState {
        db: database,
        storage,
    });

    let app = build_app(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr: SocketAddr = listener.local_addr().expect("local addr");

    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve");
    });

    (format!("http://{addr}"), token.to_string(), tmp)
}

fn build_client(registry_url: &str, token: &str) -> RegistryClient {
    let mut registries = HashMap::new();
    registries.insert("test".to_string(), registry_url.to_string());

    let mut credentials = HashMap::new();
    credentials.insert(
        "test".to_string(),
        RegistryCredential {
            token: token.to_string(),
        },
    );

    RegistryClient::new(registries, credentials, None).expect("build client")
}

// ===========================================================================
// KILL CRITERION 1: Deterministic semantic hash
// ===========================================================================

#[test]
fn kc1_deterministic_semantic_hash() {
    // Create the same module in two separate temp directories
    let dir_a = tempfile::TempDir::new().expect("dir a");
    let dir_b = tempfile::TempDir::new().expect("dir b");

    let graph_a = dir_a.path().join("graph");
    let graph_b = dir_b.path().join("graph");
    fs::create_dir_all(&graph_a).unwrap();
    fs::create_dir_all(&graph_b).unwrap();

    // Write identical module content
    let content = lib_module("mymod");
    fs::write(graph_a.join("mymod.jsonld"), &content).unwrap();
    fs::write(graph_b.join("mymod.jsonld"), &content).unwrap();

    let hash_a = hash::semantic_hash(&graph_a).expect("hash a");
    let hash_b = hash::semantic_hash(&graph_b).expect("hash b");

    assert_eq!(
        hash_a, hash_b,
        "KC1: same module in different dirs must produce identical semantic hash"
    );

    // Also verify: different @id but same logic → same hash
    let content_alt_ids = lib_module("mymod").replace("duumbi:mymod", "duumbi:alternate");
    let dir_c = tempfile::TempDir::new().expect("dir c");
    let graph_c = dir_c.path().join("graph");
    fs::create_dir_all(&graph_c).unwrap();
    fs::write(graph_c.join("mymod.jsonld"), &content_alt_ids).unwrap();

    let hash_c = hash::semantic_hash(&graph_c).expect("hash c");
    assert_eq!(
        hash_a, hash_c,
        "KC1: @id-independent — different @id, same logic must produce same hash"
    );
}

// ===========================================================================
// KILL CRITERION 2: Publish + Install round-trip
// ===========================================================================

#[tokio::test]
async fn kc2_publish_install_roundtrip() {
    let (base_url, token, _server_tmp) = start_test_server().await;
    let client = build_client(&base_url, &token);

    // Create a publishable module
    let pub_dir = tempfile::TempDir::new().expect("pub dir");
    make_publishable_module(pub_dir.path(), "mathlib", "1.0.0");

    // Pack it
    let tarball = duumbi::registry::package::pack_module(pub_dir.path())
        .expect("KC2: pack_module must succeed");

    // Publish
    let resp = client
        .publish("test", "@test/mathlib", &tarball)
        .await
        .expect("KC2: publish must succeed");
    assert_eq!(resp.name, "@test/mathlib");
    assert_eq!(resp.version, "1.0.0");

    // Install into a clean workspace
    let ws = tempfile::TempDir::new().expect("workspace");
    make_workspace(ws.path());

    let cache_dir = ws.path().join(".duumbi/cache");
    fs::create_dir_all(&cache_dir).unwrap();

    let manifest = client
        .download_module("test", "@test/mathlib", "1.0.0", &cache_dir)
        .await
        .expect("KC2: download must succeed");

    assert_eq!(manifest.module.name, "@test/mathlib");
    assert_eq!(manifest.module.version, "1.0.0");

    // Verify the module files exist in cache
    let cached_graph = cache_dir.join("@test/mathlib@1.0.0/graph");
    assert!(cached_graph.exists(), "KC2: cached graph dir must exist");

    // Verify we can load the workspace with this dependency
    let mut cfg = DuumbiConfig::default();
    cfg.dependencies.insert(
        "@test/mathlib".to_string(),
        DependencyConfig::Version("1.0.0".to_string()),
    );
    cfg.registries.insert("test".to_string(), base_url.clone());
    cfg.workspace = Some(WorkspaceSection {
        default_registry: Some("test".to_string()),
        ..Default::default()
    });
    save_cfg(ws.path(), &cfg);

    let program = deps::load_program_with_deps(ws.path())
        .expect("KC2: load_program_with_deps must succeed with installed dep");
    assert!(
        program.modules.len() >= 2,
        "KC2: must load main + installed dependency"
    );
}

// ===========================================================================
// KILL CRITERION 3: Vendor + Offline build
// ===========================================================================

#[tokio::test]
async fn kc3_vendor_offline_build() {
    let (base_url, token, _server_tmp) = start_test_server().await;
    let client = build_client(&base_url, &token);

    // Publish a module
    let pub_dir = tempfile::TempDir::new().expect("pub dir");
    make_publishable_module(pub_dir.path(), "vendorlib", "1.0.0");
    let tarball = duumbi::registry::package::pack_module(pub_dir.path()).expect("pack");
    client
        .publish("test", "@test/vendorlib", &tarball)
        .await
        .expect("publish");

    // Set up workspace and download to cache
    let ws = tempfile::TempDir::new().expect("workspace");
    make_workspace(ws.path());

    let cache_dir = ws.path().join(".duumbi/cache");
    fs::create_dir_all(&cache_dir).unwrap();
    client
        .download_module("test", "@test/vendorlib", "1.0.0", &cache_dir)
        .await
        .expect("download");

    // Configure dependency
    let mut cfg = DuumbiConfig::default();
    cfg.dependencies.insert(
        "@test/vendorlib".to_string(),
        DependencyConfig::Version("1.0.0".to_string()),
    );
    cfg.registries.insert("test".to_string(), base_url.clone());
    cfg.workspace = Some(WorkspaceSection {
        default_registry: Some("test".to_string()),
        ..Default::default()
    });
    cfg.vendor = Some(VendorSection {
        strategy: VendorStrategy::All,
        include: vec![],
    });
    save_cfg(ws.path(), &cfg);

    // Vendor — copy from cache to vendor dir
    let vendor_dir = ws.path().join(".duumbi/vendor");
    let cached = cache_dir.join("@test/vendorlib@1.0.0");
    let vendor_dest = vendor_dir.join("@test").join("vendorlib");
    fs::create_dir_all(&vendor_dest).unwrap();

    // Copy graph files
    let src_graph = cached.join("graph");
    let dst_graph = vendor_dest.join("graph");
    fs::create_dir_all(&dst_graph).unwrap();
    for entry in fs::read_dir(&src_graph).expect("read cache graph") {
        let entry = entry.unwrap();
        fs::copy(entry.path(), dst_graph.join(entry.file_name())).unwrap();
    }
    // Copy manifest
    if cached.join("manifest.toml").exists() {
        fs::copy(
            cached.join("manifest.toml"),
            vendor_dest.join("manifest.toml"),
        )
        .unwrap();
    }

    // Verify: load program with vendored dependency (no network needed)
    let program = deps::load_program_with_deps(ws.path())
        .expect("KC3: load_program_with_deps must succeed with vendored dep (offline)");
    assert!(
        program.modules.len() >= 2,
        "KC3: must load main + vendored dependency"
    );
}

// ===========================================================================
// KILL CRITERION 4: Lockfile deterministic
// ===========================================================================

#[test]
fn kc4_lockfile_deterministic() {
    // Build a workspace with a cache dependency
    let ws = tempfile::TempDir::new().expect("workspace");
    make_workspace(ws.path());

    // Add a cached dependency
    let cache_graph = ws.path().join(".duumbi/cache/@test/detmod@1.0.0/graph");
    fs::create_dir_all(&cache_graph).unwrap();
    fs::write(cache_graph.join("detmod.jsonld"), lib_module("detmod")).unwrap();

    let mut cfg = DuumbiConfig::default();
    cfg.dependencies.insert(
        "@test/detmod".to_string(),
        DependencyConfig::Version("1.0.0".to_string()),
    );
    save_cfg(ws.path(), &cfg);

    // Generate lockfile twice
    let lock_path = ws.path().join(".duumbi/deps.lock");

    let (_, content_a) =
        deps::build_lockfile(ws.path(), &cfg).expect("KC4: first lockfile generation must succeed");

    let (_, content_b) = deps::build_lockfile(ws.path(), &cfg)
        .expect("KC4: second lockfile generation must succeed");

    assert_eq!(
        content_a, content_b,
        "KC4: lockfile must be byte-identical across two generations from same state"
    );

    // Also write and re-read to verify round-trip
    fs::write(&lock_path, &content_a).unwrap();
    let content_disk = fs::read_to_string(&lock_path).unwrap();
    assert_eq!(
        content_a, content_disk,
        "KC4: lockfile must survive write/read round-trip"
    );
}
