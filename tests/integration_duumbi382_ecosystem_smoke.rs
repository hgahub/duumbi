//! DUUMBI-382 ecosystem smoke harness evidence.
//!
//! This file starts with the reusable embedded-registry harness and one
//! representative module install. Later Ralph cycles can extend the same
//! helpers across the full required module matrix.

use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::io::Read as _;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use duumbi::config::{self, WorkspaceSection};
use duumbi::manifest::ModuleManifest;
use duumbi::registry::client::{RegistryClient, RegistryCredential};
use duumbi_registry::{
    AppState, AuthMode,
    auth::rate_limit::RateLimiter,
    build_app,
    db::{CreateUser, Database},
    storage::Storage,
};

const STDLIB_VERSION: &str = "1.0.0";
const STDLIB_STRING_GRAPH: &str = include_str!("../stdlib/string.jsonld");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SmokeStage {
    Search,
    Install,
    Manifest,
    Integrity,
}

impl SmokeStage {
    fn as_str(self) -> &'static str {
        match self {
            Self::Search => "search",
            Self::Install => "install",
            Self::Manifest => "manifest",
            Self::Integrity => "integrity",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StageFailure {
    module: &'static str,
    stage: SmokeStage,
    detail: String,
}

impl StageFailure {
    fn new(module: &'static str, stage: SmokeStage, detail: impl Into<String>) -> Self {
        Self {
            module,
            stage,
            detail: detail.into(),
        }
    }
}

impl fmt::Display for StageFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "module={} stage={} status=failed detail={}",
            self.module,
            self.stage.as_str(),
            self.detail
        )
    }
}

struct DuumbiOutput {
    args: Vec<String>,
    status: ExitStatus,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    timed_out: bool,
}

struct EmbeddedRegistry {
    url: String,
    token: String,
    _tmp: tempfile::TempDir,
}

fn duumbi_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_duumbi"))
}

fn output_text(output: &DuumbiOutput) -> String {
    format!(
        "command: duumbi {}\ntimed_out: {}\nstdout:\n{}\nstderr:\n{}",
        output.args.join(" "),
        output.timed_out,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn run_duumbi(workspace: &Path, args: &[&str]) -> DuumbiOutput {
    const COMMAND_TIMEOUT: Duration = Duration::from_secs(30);

    let mut child = Command::new(duumbi_binary())
        .args(args)
        .current_dir(workspace)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|error| panic!("duumbi command {args:?} should start: {error}"));

    let started = Instant::now();
    let mut timed_out = false;
    loop {
        if child.try_wait().expect("poll duumbi command").is_some() {
            break;
        }
        if started.elapsed() >= COMMAND_TIMEOUT {
            timed_out = true;
            child.kill().expect("kill timed-out duumbi command");
            break;
        }
        thread::sleep(Duration::from_millis(25));
    }

    let mut stdout = Vec::new();
    if let Some(mut reader) = child.stdout.take() {
        reader.read_to_end(&mut stdout).expect("read stdout");
    }
    let mut stderr = Vec::new();
    if let Some(mut reader) = child.stderr.take() {
        reader.read_to_end(&mut stderr).expect("read stderr");
    }
    let status = child.wait().expect("wait for duumbi command");

    DuumbiOutput {
        args: args.iter().map(|arg| (*arg).to_string()).collect(),
        status,
        stdout,
        stderr,
        timed_out,
    }
}

fn assert_command_success(
    module: &'static str,
    stage: SmokeStage,
    output: DuumbiOutput,
) -> DuumbiOutput {
    assert!(
        output.status.success(),
        "{}",
        StageFailure::new(module, stage, output_text(&output))
    );
    output
}

async fn start_embedded_registry() -> EmbeddedRegistry {
    let tmp = tempfile::TempDir::new().expect("temp dir");

    let database = Database::open(":memory:").expect("in-memory db");
    database.migrate().expect("migration");

    let token = "duu_duumbi_382_ecosystem_token";
    let user_id = database
        .create_user(&CreateUser {
            username: "duumbi382",
            display_name: None,
            avatar_url: None,
            email: None,
            password_hash: None,
        })
        .expect("create test user");
    database
        .create_token(user_id, "duumbi-382-ecosystem", token)
        .expect("create token");

    let storage = Storage::new(tmp.path().join("modules").to_str().unwrap()).expect("storage");

    let state = Arc::new(AppState {
        db: database,
        storage,
        auth_mode: AuthMode::LocalPassword,
        jwt_secret: "test-jwt-secret".to_string(),
        base_url: "http://localhost".to_string(),
        github_client_id: None,
        github_client_secret: None,
        rate_limiter: RateLimiter::new(),
    });

    let app = build_app(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr: SocketAddr = listener.local_addr().expect("local addr");

    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve");
    });

    EmbeddedRegistry {
        url: format!("http://{addr}"),
        token: token.to_string(),
        _tmp: tmp,
    }
}

fn registry_client(registry_url: &str, token: Option<&str>) -> RegistryClient {
    let mut registries = HashMap::new();
    registries.insert("local".to_string(), registry_url.to_string());

    let mut credentials = HashMap::new();
    if let Some(token) = token {
        credentials.insert(
            "local".to_string(),
            RegistryCredential {
                token: token.to_string(),
            },
        );
    }

    RegistryClient::new(registries, credentials, None).expect("registry client")
}

fn make_stdlib_string_package(workspace: &Path) {
    let graph_dir = workspace.join(".duumbi/graph");
    fs::create_dir_all(&graph_dir).expect("create package graph dir");
    fs::write(graph_dir.join("string.jsonld"), STDLIB_STRING_GRAPH).expect("write string graph");

    let manifest = ModuleManifest::new(
        "@duumbi/stdlib-string",
        STDLIB_VERSION,
        "String utility functions",
        vec![
            "length".to_string(),
            "concat".to_string(),
            "contains".to_string(),
            "substring".to_string(),
            "trim".to_string(),
            "replace".to_string(),
        ],
    );
    fs::write(workspace.join(".duumbi/manifest.toml"), manifest.to_toml()).expect("write manifest");
}

async fn publish_stdlib_string(registry: &EmbeddedRegistry) {
    let package_workspace = tempfile::TempDir::new().expect("package workspace");
    make_stdlib_string_package(package_workspace.path());

    let tarball = duumbi::registry::package::pack_module(package_workspace.path())
        .expect("pack stdlib-string");
    let client = registry_client(&registry.url, Some(&registry.token));
    let response = client
        .publish("local", "@duumbi/stdlib-string", &tarball)
        .await
        .expect("publish stdlib-string");
    assert_eq!(response.name, "@duumbi/stdlib-string");
    assert_eq!(response.version, STDLIB_VERSION);
}

fn configure_embedded_registry(workspace: &Path, registry_url: &str) {
    let mut cfg = config::load_config(workspace).expect("config should parse");
    cfg.registries
        .insert("local".to_string(), registry_url.to_string());
    let workspace_section = cfg.workspace.get_or_insert_with(WorkspaceSection::default);
    workspace_section.default_registry = Some("local".to_string());
    config::save_config(workspace, &cfg).expect("save registry config");
}

fn assert_installed_string_metadata(workspace: &Path) {
    let module = "@duumbi/stdlib-string";
    let cache_entry = workspace
        .join(".duumbi/cache")
        .join("@duumbi/stdlib-string@1.0.0");

    assert!(
        cache_entry.join("graph/string.jsonld").exists(),
        "{}",
        StageFailure::new(module, SmokeStage::Install, "missing graph/string.jsonld")
    );
    assert!(
        cache_entry.join("CHECKSUM").exists(),
        "{}",
        StageFailure::new(module, SmokeStage::Integrity, "missing package CHECKSUM")
    );
    assert!(
        cache_entry.join(".integrity").exists(),
        "{}",
        StageFailure::new(
            module,
            SmokeStage::Integrity,
            "missing downloaded .integrity"
        )
    );

    let manifest_path = cache_entry.join("manifest.toml");
    let manifest = duumbi::manifest::parse_manifest(&manifest_path).unwrap_or_else(|error| {
        panic!(
            "{}",
            StageFailure::new(module, SmokeStage::Manifest, error.to_string())
        )
    });
    assert_eq!(manifest.module.name, module);
    assert_eq!(manifest.module.version, STDLIB_VERSION);
    assert!(
        manifest.exports.functions.contains(&"length".to_string()),
        "{}",
        StageFailure::new(module, SmokeStage::Manifest, "missing length export")
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn embedded_registry_harness_searches_installs_and_verifies_string_metadata() {
    let module = "@duumbi/stdlib-string";
    let registry = start_embedded_registry().await;
    publish_stdlib_string(&registry).await;

    let workspace = tempfile::TempDir::new().expect("workspace");
    let init = run_duumbi(
        workspace.path(),
        &["init", workspace.path().to_str().unwrap()],
    );
    assert_command_success(module, SmokeStage::Install, init);
    configure_embedded_registry(workspace.path(), &registry.url);

    let search = assert_command_success(
        module,
        SmokeStage::Search,
        run_duumbi(
            workspace.path(),
            &["search", "stdlib", "--registry", "local"],
        ),
    );
    assert!(
        output_text(&search).contains(module),
        "{}",
        StageFailure::new(
            module,
            SmokeStage::Search,
            "search output did not list module"
        )
    );

    let add = run_duumbi(
        workspace.path(),
        &[
            "deps",
            "add",
            "@duumbi/stdlib-string@1.0.0",
            "--registry",
            "local",
        ],
    );
    assert_command_success(module, SmokeStage::Install, add);

    let cfg = config::load_config(workspace.path()).expect("config should parse");
    assert!(
        cfg.dependencies.contains_key(module),
        "{}",
        StageFailure::new(
            module,
            SmokeStage::Install,
            "config dependency was not recorded"
        )
    );
    assert_installed_string_metadata(workspace.path());
}

#[test]
fn stage_failure_messages_include_module_and_stage() {
    let failure = StageFailure::new("@duumbi/stdlib-json", SmokeStage::Install, "missing cache");
    assert_eq!(
        failure.to_string(),
        "module=@duumbi/stdlib-json stage=install status=failed detail=missing cache"
    );
}
