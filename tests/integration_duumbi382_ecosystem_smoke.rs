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
use duumbi::workspace::{build_workspace, run_workspace_binary, workspace_output_path};
use duumbi_registry::{
    AppState, AuthMode,
    auth::rate_limit::RateLimiter,
    build_app,
    db::{CreateUser, Database},
    storage::Storage,
};
use serde_json::{Value, json};

const STDLIB_VERSION: &str = "1.0.0";
const STDLIB_MATH_GRAPH: &str = include_str!("../stdlib/math.jsonld");
const STDLIB_IO_GRAPH: &str = include_str!("../stdlib/io.jsonld");
const STDLIB_LANG_GRAPH: &str = include_str!("../stdlib/lang.jsonld");
const STDLIB_STRING_GRAPH: &str = include_str!("../stdlib/string.jsonld");
const STDLIB_FILE_GRAPH: &str = include_str!("../stdlib/file.jsonld");
const STDLIB_JSON_GRAPH: &str = include_str!("../stdlib/json.jsonld");
const STDLIB_NET_GRAPH: &str = include_str!("../stdlib/net.jsonld");
const STDLIB_HTTP_GRAPH: &str = include_str!("../stdlib/http.jsonld");
const STDLIB_DB_GRAPH: &str = include_str!("../stdlib/db.jsonld");
const STDLIB_SERVER_GRAPH: &str = include_str!("../stdlib/server.jsonld");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SmokeStage {
    Search,
    Install,
    Manifest,
    Integrity,
    Import,
    Build,
    Run,
    ProductionGuard,
}

impl SmokeStage {
    fn as_str(self) -> &'static str {
        match self {
            Self::Search => "search",
            Self::Install => "install",
            Self::Manifest => "manifest",
            Self::Integrity => "integrity",
            Self::Import => "import",
            Self::Build => "build",
            Self::Run => "run",
            Self::ProductionGuard => "production-guard",
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProductionSmokeDecision {
    Skipped,
    Ready,
}

#[derive(Debug, Clone, Copy)]
struct ModuleSpec {
    module: &'static str,
    graph_file: &'static str,
    graph: &'static str,
    description: &'static str,
    exports: &'static [&'static str],
}

impl ModuleSpec {
    fn manifest(self) -> ModuleManifest {
        ModuleManifest::new(
            self.module,
            STDLIB_VERSION,
            self.description,
            self.exports
                .iter()
                .map(|export| (*export).to_string())
                .collect(),
        )
    }
}

const REQUIRED_MODULES: &[ModuleSpec] = &[
    ModuleSpec {
        module: "@duumbi/stdlib-math",
        graph_file: "math.jsonld",
        graph: STDLIB_MATH_GRAPH,
        description: "Mathematical utility functions (abs, max, min, sqrt, pow, mod, clamp, sign)",
        exports: &["abs", "max", "min", "sqrt", "pow", "mod", "clamp", "sign"],
    },
    ModuleSpec {
        module: "@duumbi/stdlib-io",
        graph_file: "io.jsonld",
        graph: STDLIB_IO_GRAPH,
        description: "I/O utility functions (print wrappers, read_line, print_ln)",
        exports: &[
            "print_i64",
            "print_f64",
            "print_bool",
            "print_string",
            "read_line",
            "print_ln",
        ],
    },
    ModuleSpec {
        module: "@duumbi/stdlib-lang",
        graph_file: "lang.jsonld",
        graph: STDLIB_LANG_GRAPH,
        description: "Language utility functions (assert_true, i64_to_f64, f64_to_i64)",
        exports: &["assert_true", "i64_to_f64", "f64_to_i64"],
    },
    ModuleSpec {
        module: "@duumbi/stdlib-string",
        graph_file: "string.jsonld",
        graph: STDLIB_STRING_GRAPH,
        description: "String utility functions (length, contains, find, trim, to_upper, to_lower, replace)",
        exports: &[
            "length", "contains", "find", "trim", "to_upper", "to_lower", "replace",
        ],
    },
    ModuleSpec {
        module: "@duumbi/stdlib-file",
        graph_file: "file.jsonld",
        graph: STDLIB_FILE_GRAPH,
        description: "Workspace-confined UTF-8 file and path utility functions",
        exports: &[
            "read_file",
            "write_file",
            "file_exists",
            "list_dir",
            "path_join",
        ],
    },
    ModuleSpec {
        module: "@duumbi/stdlib-json",
        graph_file: "json.jsonld",
        graph: STDLIB_JSON_GRAPH,
        description: "JSON utility functions (parse, stringify, get_field, array_len, array_get)",
        exports: &["parse", "stringify", "get_field", "array_len", "array_get"],
    },
    ModuleSpec {
        module: "@duumbi/stdlib-net",
        graph_file: "net.jsonld",
        graph: STDLIB_NET_GRAPH,
        description: "TCP utility functions (connect, listen, accept, read, write, close)",
        exports: &[
            "tcp_connect",
            "tcp_listen",
            "tcp_accept",
            "tcp_read",
            "tcp_write",
            "tcp_close",
            "tcp_listener_close",
        ],
    },
    ModuleSpec {
        module: "@duumbi/stdlib-http",
        graph_file: "http.jsonld",
        graph: STDLIB_HTTP_GRAPH,
        description: "HTTP/HTTPS utility functions (GET, POST, PUT, DELETE, response accessors)",
        exports: &[
            "http_get",
            "http_post",
            "http_put",
            "http_delete",
            "http_status",
            "http_body",
            "http_headers",
            "http_response_free",
        ],
    },
    ModuleSpec {
        module: "@duumbi/stdlib-db",
        graph_file: "db.jsonld",
        graph: STDLIB_DB_GRAPH,
        description: "Local SQLite utility functions (open, execute, query, row access, cleanup)",
        exports: &[
            "db_open",
            "db_execute",
            "db_query",
            "db_rows_len",
            "db_row_get",
            "db_close",
            "db_rows_free",
        ],
    },
    ModuleSpec {
        module: "@duumbi/stdlib-server",
        graph_file: "server.jsonld",
        graph: STDLIB_SERVER_GRAPH,
        description: "Bounded local static-route HTTP server functions",
        exports: &[
            "server_new",
            "route_add_static",
            "server_start",
            "server_close",
        ],
    },
];

const BUILD_RUN_MODULES: &[ModuleSpec] = &[
    REQUIRED_MODULES[0],
    REQUIRED_MODULES[1],
    REQUIRED_MODULES[2],
    REQUIRED_MODULES[3],
];

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

fn node_ref(id: &str) -> Value {
    json!({ "@id": id })
}

fn main_id(name: &str) -> String {
    format!("duumbi:main/main/entry/{name}")
}

fn write_main_graph(workspace: &Path, graph: Value) {
    let graph_dir = workspace.join(".duumbi/graph");
    fs::create_dir_all(&graph_dir).expect("create graph dir");
    fs::write(
        graph_dir.join("main.jsonld"),
        serde_json::to_string_pretty(&graph).expect("serialize graph"),
    )
    .expect("write main graph");
}

fn core_import_build_run_graph() -> Value {
    json!({
        "@context": {"duumbi": "https://duumbi.dev/ns/core#"},
        "@type": "duumbi:Module",
        "@id": "duumbi:main",
        "duumbi:name": "main",
        "duumbi:imports": [
            {
                "duumbi:module": "math",
                "duumbi:path": "@duumbi/stdlib-math",
                "duumbi:functions": ["abs"]
            },
            {
                "duumbi:module": "io",
                "duumbi:path": "@duumbi/stdlib-io",
                "duumbi:functions": ["print_i64"]
            },
            {
                "duumbi:module": "lang",
                "duumbi:path": "@duumbi/stdlib-lang",
                "duumbi:functions": ["assert_true"]
            },
            {
                "duumbi:module": "string",
                "duumbi:path": "@duumbi/stdlib-string",
                "duumbi:functions": ["length"]
            }
        ],
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
                    {"@type": "duumbi:Const", "@id": main_id("negative"),
                     "duumbi:value": -7, "duumbi:resultType": "i64"},
                    {"@type": "duumbi:Call", "@id": main_id("abs"),
                     "duumbi:module": "math", "duumbi:function": "abs",
                     "duumbi:args": [node_ref(&main_id("negative"))],
                     "duumbi:resultType": "i64"},
                    {"@type": "duumbi:Call", "@id": main_id("print_abs"),
                     "duumbi:module": "io", "duumbi:function": "print_i64",
                     "duumbi:args": [node_ref(&main_id("abs"))],
                     "duumbi:resultType": "i64"},
                    {"@type": "duumbi:Const", "@id": main_id("text"),
                     "duumbi:value": "duumbi", "duumbi:resultType": "string"},
                    {"@type": "duumbi:Call", "@id": main_id("length"),
                     "duumbi:module": "string", "duumbi:function": "length",
                     "duumbi:args": [node_ref(&main_id("text"))],
                     "duumbi:resultType": "i64"},
                    {"@type": "duumbi:Call", "@id": main_id("print_length"),
                     "duumbi:module": "io", "duumbi:function": "print_i64",
                     "duumbi:args": [node_ref(&main_id("length"))],
                     "duumbi:resultType": "i64"},
                    {"@type": "duumbi:Const", "@id": main_id("truth"),
                     "duumbi:value": true, "duumbi:resultType": "bool"},
                    {"@type": "duumbi:Call", "@id": main_id("assert"),
                     "duumbi:module": "lang", "duumbi:function": "assert_true",
                     "duumbi:args": [node_ref(&main_id("truth"))],
                     "duumbi:resultType": "i64"},
                    {"@type": "duumbi:Const", "@id": main_id("zero"),
                     "duumbi:value": 0, "duumbi:resultType": "i64"},
                    {"@type": "duumbi:Return", "@id": main_id("return"),
                     "duumbi:operand": node_ref(&main_id("zero"))}
                ]
            }]
        }]
    })
}

fn production_smoke_decision(opt_in: Option<&str>, token: Option<&str>) -> ProductionSmokeDecision {
    if opt_in == Some("1") && token.is_some_and(|value| !value.trim().is_empty()) {
        ProductionSmokeDecision::Ready
    } else {
        ProductionSmokeDecision::Skipped
    }
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

fn make_stdlib_package(workspace: &Path, module: ModuleSpec) {
    let graph_dir = workspace.join(".duumbi/graph");
    fs::create_dir_all(&graph_dir).expect("create package graph dir");
    fs::write(graph_dir.join(module.graph_file), module.graph).expect("write graph");

    let manifest = module.manifest();
    fs::write(workspace.join(".duumbi/manifest.toml"), manifest.to_toml()).expect("write manifest");
}

async fn publish_module(registry: &EmbeddedRegistry, module: ModuleSpec) {
    let package_workspace = tempfile::TempDir::new().expect("package workspace");
    make_stdlib_package(package_workspace.path(), module);

    let tarball = duumbi::registry::package::pack_module(package_workspace.path())
        .unwrap_or_else(|error| panic!("pack {}: {error}", module.module));
    let client = registry_client(&registry.url, Some(&registry.token));
    let response = client
        .publish("local", module.module, &tarball)
        .await
        .unwrap_or_else(|error| panic!("publish {}: {error}", module.module));
    assert_eq!(response.name, module.module);
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

fn assert_installed_metadata(workspace: &Path, module: ModuleSpec) {
    let cache_leaf = module
        .module
        .strip_prefix("@duumbi/")
        .expect("stdlib scope");
    let cache_entry = workspace
        .join(".duumbi/cache")
        .join(format!("@duumbi/{cache_leaf}@{STDLIB_VERSION}"));

    assert!(
        cache_entry.join("graph").join(module.graph_file).exists(),
        "{}",
        StageFailure::new(
            module.module,
            SmokeStage::Install,
            format!("missing graph/{}", module.graph_file)
        )
    );
    assert!(
        cache_entry.join("CHECKSUM").exists(),
        "{}",
        StageFailure::new(
            module.module,
            SmokeStage::Integrity,
            "missing package CHECKSUM"
        )
    );
    assert!(
        cache_entry.join(".integrity").exists(),
        "{}",
        StageFailure::new(
            module.module,
            SmokeStage::Integrity,
            "missing downloaded .integrity"
        )
    );

    let manifest_path = cache_entry.join("manifest.toml");
    let manifest = duumbi::manifest::parse_manifest(&manifest_path).unwrap_or_else(|error| {
        panic!(
            "{}",
            StageFailure::new(module.module, SmokeStage::Manifest, error.to_string())
        )
    });
    assert_eq!(manifest.module.name, module.module);
    assert_eq!(manifest.module.version, STDLIB_VERSION);
    for expected_export in module.exports {
        assert!(
            manifest
                .exports
                .functions
                .contains(&(*expected_export).to_string()),
            "{}",
            StageFailure::new(
                module.module,
                SmokeStage::Manifest,
                format!("missing {expected_export} export")
            )
        );
    }
}

async fn seed_registry(registry: &EmbeddedRegistry, modules: &[ModuleSpec]) {
    for module in modules {
        publish_module(registry, *module).await;
    }
}

fn init_workspace_with_registry(workspace: &Path, registry_url: &str) {
    let init = run_duumbi(workspace, &["init", workspace.to_str().unwrap()]);
    assert_command_success("@duumbi/stdlib-string", SmokeStage::Install, init);
    configure_embedded_registry(workspace, registry_url);
}

fn deps_add_module(workspace: &Path, module: ModuleSpec) {
    let specifier = format!("{}@{STDLIB_VERSION}", module.module);
    let add = run_duumbi(
        workspace,
        &["deps", "add", &specifier, "--registry", "local"],
    );
    assert_command_success(module.module, SmokeStage::Install, add);

    let cfg = config::load_config(workspace).expect("config should parse");
    assert!(
        cfg.dependencies.contains_key(module.module),
        "{}",
        StageFailure::new(
            module.module,
            SmokeStage::Install,
            "config dependency was not recorded"
        )
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn embedded_registry_harness_searches_installs_and_verifies_required_module_metadata() {
    let registry = start_embedded_registry().await;
    seed_registry(&registry, REQUIRED_MODULES).await;

    let workspace = tempfile::TempDir::new().expect("workspace");
    init_workspace_with_registry(workspace.path(), &registry.url);

    let search = assert_command_success(
        "@duumbi/stdlib-string",
        SmokeStage::Search,
        run_duumbi(
            workspace.path(),
            &["search", "stdlib", "--registry", "local"],
        ),
    );
    let search_text = output_text(&search);

    for module in REQUIRED_MODULES {
        assert!(
            search_text.contains(module.module),
            "{}",
            StageFailure::new(
                module.module,
                SmokeStage::Search,
                "search output did not list module"
            )
        );

        deps_add_module(workspace.path(), *module);
        assert_installed_metadata(workspace.path(), *module);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn installed_core_modules_import_build_and_run_from_clean_workspace() {
    let registry = start_embedded_registry().await;
    seed_registry(&registry, BUILD_RUN_MODULES).await;

    let workspace = tempfile::TempDir::new().expect("workspace");
    init_workspace_with_registry(workspace.path(), &registry.url);
    for module in BUILD_RUN_MODULES {
        deps_add_module(workspace.path(), *module);
    }

    write_main_graph(workspace.path(), core_import_build_run_graph());
    assert!(
        workspace.path().join(".duumbi/graph/main.jsonld").exists(),
        "{}",
        StageFailure::new(
            "@duumbi/stdlib-math",
            SmokeStage::Import,
            "main importer graph was not written"
        )
    );
    let output_path = workspace_output_path(workspace.path());
    build_workspace(workspace.path(), &output_path, false).unwrap_or_else(|error| {
        panic!(
            "{}",
            StageFailure::new("@duumbi/stdlib-math", SmokeStage::Build, error.to_string())
        )
    });

    let run = run_workspace_binary(workspace.path(), &[]).unwrap_or_else(|error| {
        panic!(
            "{}",
            StageFailure::new("@duumbi/stdlib-math", SmokeStage::Run, error.to_string())
        )
    });
    assert_eq!(
        run.exit_code,
        0,
        "{}",
        StageFailure::new(
            "@duumbi/stdlib-math",
            SmokeStage::Run,
            format!("exit={} stderr={}", run.exit_code, run.stderr)
        )
    );
    assert_eq!(run.stdout.trim(), "7\n6");
    assert_eq!(run.stderr.trim(), "");
}

#[test]
fn production_smoke_guard_fails_closed_without_explicit_opt_in() {
    assert_eq!(
        production_smoke_decision(None, None),
        ProductionSmokeDecision::Skipped,
        "{}",
        StageFailure::new(
            "@duumbi/production-registry",
            SmokeStage::ProductionGuard,
            "missing opt-in must skip"
        )
    );
    assert_eq!(
        production_smoke_decision(Some("0"), Some("token")),
        ProductionSmokeDecision::Skipped
    );
    assert_eq!(
        production_smoke_decision(Some("1"), None),
        ProductionSmokeDecision::Skipped
    );
    assert_eq!(
        production_smoke_decision(Some("1"), Some("token")),
        ProductionSmokeDecision::Ready
    );
}

#[test]
fn stage_failure_messages_include_module_and_stage() {
    let failure = StageFailure::new("@duumbi/stdlib-json", SmokeStage::Install, "missing cache");
    assert_eq!(
        failure.to_string(),
        "module=@duumbi/stdlib-json stage=install status=failed detail=missing cache"
    );
}
