//! DUUMBI-381 Cycle 3 evidence: embedded-registry, clean-workspace, and server E2E.

#![recursion_limit = "512"]

use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::io::{ErrorKind, Read as _, Write as _};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

use duumbi::config::{self, DependencyConfig, WorkspaceSection};
use duumbi::registry::client::{RegistryClient, RegistryCredential};
use duumbi::workspace::{build_workspace, run_workspace_binary, workspace_output_path};
use duumbi_registry::{
    AppState, AuthMode,
    auth::rate_limit::RateLimiter,
    build_app,
    db::{CreateUser, Database},
    storage::Storage,
};
use serde_json::json;

const SERVER_GRAPH: &str = include_str!("../stdlib/server.jsonld");
const SERVER_MANIFEST: &str = include_str!("../stdlib/server.manifest.toml");

fn duumbi_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_duumbi"))
}

fn run_duumbi_init(workspace: &Path) {
    let output = Command::new(duumbi_binary())
        .arg("init")
        .arg(workspace)
        .output()
        .expect("duumbi init should run");
    assert!(
        output.status.success(),
        "duumbi init failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn unused_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind unused port");
    listener.local_addr().expect("local addr").port()
}

fn write_main_graph(workspace: &Path, graph: serde_json::Value) {
    let graph_dir = workspace.join(".duumbi/graph");
    fs::create_dir_all(&graph_dir).expect("create graph dir");
    fs::write(
        graph_dir.join("main.jsonld"),
        serde_json::to_string_pretty(&graph).expect("serialize graph"),
    )
    .expect("write main graph");
}

fn make_server_package(workspace: &Path) {
    let graph_dir = workspace.join(".duumbi/graph");
    fs::create_dir_all(&graph_dir).expect("create package graph dir");
    fs::write(graph_dir.join("server.jsonld"), SERVER_GRAPH).expect("write server graph");
    fs::write(workspace.join(".duumbi/manifest.toml"), SERVER_MANIFEST)
        .expect("write server manifest");
}

async fn start_test_server() -> (String, String, tempfile::TempDir) {
    let tmp = tempfile::TempDir::new().expect("temp dir");

    let database = Database::open(":memory:").expect("in-memory db");
    database.migrate().expect("migration");

    let token = "duu_duumbi_381_cycle3_token";
    let user_id = database
        .create_user(&CreateUser {
            username: "duumbi381",
            display_name: None,
            avatar_url: None,
            email: None,
            password_hash: None,
        })
        .expect("create test user");
    database
        .create_token(user_id, "duumbi-381-cycle3", token)
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

    (format!("http://{}", addr), token.to_string(), tmp)
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

fn assert_default_dependency_policy(workspace: &Path) {
    let cfg = config::load_config(workspace).expect("config should parse");
    let actual: BTreeSet<&str> = cfg.dependencies.keys().map(String::as_str).collect();
    let expected = BTreeSet::from([
        "@duumbi/stdlib-io",
        "@duumbi/stdlib-lang",
        "@duumbi/stdlib-math",
        "@duumbi/stdlib-string",
    ]);
    assert_eq!(
        actual, expected,
        "default dependencies must remain core-only"
    );

    for opt_in_module in [
        "@duumbi/stdlib-json",
        "@duumbi/stdlib-net",
        "@duumbi/stdlib-server",
        "@duumbi/stdlib-http",
        "@duumbi/stdlib-db",
    ] {
        assert!(
            !cfg.dependencies.contains_key(opt_in_module),
            "{opt_in_module} must not be a default dependency"
        );
    }
}

fn configure_local_registry(workspace: &Path, registry_url: &str) {
    let mut cfg = config::load_config(workspace).expect("config should parse");
    cfg.registries
        .insert("local".to_string(), registry_url.to_string());
    let ws = cfg.workspace.get_or_insert_with(WorkspaceSection::default);
    ws.default_registry = Some("local".to_string());
    config::save_config(workspace, &cfg).expect("save local registry config");
}

fn add_downloaded_server_dependency(workspace: &Path) {
    let mut cfg = config::load_config(workspace).expect("config should parse");
    cfg.dependencies.insert(
        "@duumbi/stdlib-server".to_string(),
        DependencyConfig::VersionWithRegistry {
            version: "1.0.0".to_string(),
            registry: "local".to_string(),
        },
    );
    config::save_config(workspace, &cfg).expect("save dependency config");
}

fn server_importer_graph(port: u16) -> serde_json::Value {
    json!({
        "@context": {"duumbi": "https://duumbi.dev/ns/core#"},
        "@type": "duumbi:Module",
        "@id": "duumbi:main",
        "duumbi:name": "main",
        "duumbi:imports": [{
            "duumbi:module": "server",
            "duumbi:path": "@duumbi/stdlib-server",
            "duumbi:functions": [
                "server_new",
                "route_add_static",
                "server_start",
                "server_close"
            ]
        }],
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
                    {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/0", "duumbi:value": "127.0.0.1", "duumbi:resultType": "string"},
                    {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/1", "duumbi:value": i64::from(port), "duumbi:resultType": "i64"},
                    {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/2", "duumbi:value": 1000, "duumbi:resultType": "i64"},
                    {"@type": "duumbi:Call", "@id": "duumbi:main/main/entry/3", "duumbi:module": "server", "duumbi:function": "server_new", "duumbi:args": [
                        {"@id": "duumbi:main/main/entry/0"}, {"@id": "duumbi:main/main/entry/1"}, {"@id": "duumbi:main/main/entry/2"}
                    ], "duumbi:resultType": "result<http_server,string>"},
                    {"@type": "duumbi:ResultUnwrap", "@id": "duumbi:main/main/entry/4", "duumbi:operand": {"@id": "duumbi:main/main/entry/3"}, "duumbi:resultType": "http_server"},
                    {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/5", "duumbi:value": "GET", "duumbi:resultType": "string"},
                    {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/6", "duumbi:value": "/health", "duumbi:resultType": "string"},
                    {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/7", "duumbi:value": 200, "duumbi:resultType": "i64"},
                    {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/8", "duumbi:value": "{}", "duumbi:resultType": "string"},
                    {"@type": "duumbi:JsonParse", "@id": "duumbi:main/main/entry/9", "duumbi:operand": {"@id": "duumbi:main/main/entry/8"}, "duumbi:resultType": "result<json,string>"},
                    {"@type": "duumbi:ResultUnwrap", "@id": "duumbi:main/main/entry/10", "duumbi:operand": {"@id": "duumbi:main/main/entry/9"}, "duumbi:resultType": "json"},
                    {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/11", "duumbi:value": "ok", "duumbi:resultType": "string"},
                    {"@type": "duumbi:Call", "@id": "duumbi:main/main/entry/12", "duumbi:module": "server", "duumbi:function": "route_add_static", "duumbi:args": [
                        {"@id": "duumbi:main/main/entry/4"}, {"@id": "duumbi:main/main/entry/5"}, {"@id": "duumbi:main/main/entry/6"},
                        {"@id": "duumbi:main/main/entry/7"}, {"@id": "duumbi:main/main/entry/10"}, {"@id": "duumbi:main/main/entry/11"}
                    ], "duumbi:resultType": "result<i64,string>"},
                    {"@type": "duumbi:ResultUnwrap", "@id": "duumbi:main/main/entry/13", "duumbi:operand": {"@id": "duumbi:main/main/entry/12"}, "duumbi:resultType": "i64"},
                    {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/14", "duumbi:value": 1, "duumbi:resultType": "i64"},
                    {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/15", "duumbi:value": 2000, "duumbi:resultType": "i64"},
                    {"@type": "duumbi:Call", "@id": "duumbi:main/main/entry/16", "duumbi:module": "server", "duumbi:function": "server_start", "duumbi:args": [
                        {"@id": "duumbi:main/main/entry/4"}, {"@id": "duumbi:main/main/entry/14"}, {"@id": "duumbi:main/main/entry/15"}
                    ], "duumbi:resultType": "result<i64,string>"},
                    {"@type": "duumbi:ResultUnwrap", "@id": "duumbi:main/main/entry/17", "duumbi:operand": {"@id": "duumbi:main/main/entry/16"}, "duumbi:resultType": "i64"},
                    {"@type": "duumbi:Call", "@id": "duumbi:main/main/entry/18", "duumbi:module": "server", "duumbi:function": "server_close", "duumbi:args": [
                        {"@id": "duumbi:main/main/entry/4"}
                    ], "duumbi:resultType": "result<i64,string>"},
                    {"@type": "duumbi:ResultUnwrap", "@id": "duumbi:main/main/entry/19", "duumbi:operand": {"@id": "duumbi:main/main/entry/18"}, "duumbi:resultType": "i64"},
                    {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/20", "duumbi:value": 0, "duumbi:resultType": "i64"},
                    {"@type": "duumbi:Return", "@id": "duumbi:main/main/entry/21", "duumbi:operand": {"@id": "duumbi:main/main/entry/20"}}
                ]
            }]
        }]
    })
}

fn server_negative_graph(
    timeout_port: u16,
    invalid_port: u16,
    closed_port: u16,
) -> serde_json::Value {
    json!({
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
                    {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/host", "duumbi:value": "127.0.0.1", "duumbi:resultType": "string"},
                    {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/timeout_port", "duumbi:value": i64::from(timeout_port), "duumbi:resultType": "i64"},
                    {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/init_timeout", "duumbi:value": 1000, "duumbi:resultType": "i64"},
                    {"@type": "duumbi:ServerNew", "@id": "duumbi:main/main/entry/timeout_new", "duumbi:operand": {"@id": "duumbi:main/main/entry/host"}, "duumbi:left": {"@id": "duumbi:main/main/entry/timeout_port"}, "duumbi:right": {"@id": "duumbi:main/main/entry/init_timeout"}, "duumbi:resultType": "result<http_server,string>"},
                    {"@type": "duumbi:ResultUnwrap", "@id": "duumbi:main/main/entry/timeout_server", "duumbi:operand": {"@id": "duumbi:main/main/entry/timeout_new"}, "duumbi:resultType": "http_server"},
                    {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/get", "duumbi:value": "GET", "duumbi:resultType": "string"},
                    {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/health", "duumbi:value": "/health", "duumbi:resultType": "string"},
                    {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/status", "duumbi:value": 200, "duumbi:resultType": "i64"},
                    {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/headers_src", "duumbi:value": "{}", "duumbi:resultType": "string"},
                    {"@type": "duumbi:JsonParse", "@id": "duumbi:main/main/entry/headers_parse", "duumbi:operand": {"@id": "duumbi:main/main/entry/headers_src"}, "duumbi:resultType": "result<json,string>"},
                    {"@type": "duumbi:ResultUnwrap", "@id": "duumbi:main/main/entry/headers", "duumbi:operand": {"@id": "duumbi:main/main/entry/headers_parse"}, "duumbi:resultType": "json"},
                    {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/body", "duumbi:value": "ok", "duumbi:resultType": "string"},
                    {"@type": "duumbi:RouteAddStatic", "@id": "duumbi:main/main/entry/route_ok", "duumbi:operand": {"@id": "duumbi:main/main/entry/timeout_server"}, "duumbi:args": [
                        {"@id": "duumbi:main/main/entry/get"}, {"@id": "duumbi:main/main/entry/health"}, {"@id": "duumbi:main/main/entry/status"}, {"@id": "duumbi:main/main/entry/headers"}, {"@id": "duumbi:main/main/entry/body"}
                    ], "duumbi:resultType": "result<i64,string>"},
                    {"@type": "duumbi:ResultUnwrap", "@id": "duumbi:main/main/entry/route_ok_unwrap", "duumbi:operand": {"@id": "duumbi:main/main/entry/route_ok"}, "duumbi:resultType": "i64"},
                    {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/max_one", "duumbi:value": 1, "duumbi:resultType": "i64"},
                    {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/short_timeout", "duumbi:value": 50, "duumbi:resultType": "i64"},
                    {"@type": "duumbi:ServerStart", "@id": "duumbi:main/main/entry/timeout_start", "duumbi:operand": {"@id": "duumbi:main/main/entry/timeout_server"}, "duumbi:left": {"@id": "duumbi:main/main/entry/max_one"}, "duumbi:right": {"@id": "duumbi:main/main/entry/short_timeout"}, "duumbi:resultType": "result<i64,string>"},
                    {"@type": "duumbi:ResultUnwrapErr", "@id": "duumbi:main/main/entry/timeout_err", "duumbi:operand": {"@id": "duumbi:main/main/entry/timeout_start"}, "duumbi:resultType": "string"},
                    {"@type": "duumbi:ServerClose", "@id": "duumbi:main/main/entry/timeout_close", "duumbi:operand": {"@id": "duumbi:main/main/entry/timeout_server"}, "duumbi:resultType": "result<i64,string>"},
                    {"@type": "duumbi:ResultUnwrap", "@id": "duumbi:main/main/entry/timeout_close_unwrap", "duumbi:operand": {"@id": "duumbi:main/main/entry/timeout_close"}, "duumbi:resultType": "i64"},

                    {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/invalid_port", "duumbi:value": i64::from(invalid_port), "duumbi:resultType": "i64"},
                    {"@type": "duumbi:ServerNew", "@id": "duumbi:main/main/entry/invalid_new", "duumbi:operand": {"@id": "duumbi:main/main/entry/host"}, "duumbi:left": {"@id": "duumbi:main/main/entry/invalid_port"}, "duumbi:right": {"@id": "duumbi:main/main/entry/init_timeout"}, "duumbi:resultType": "result<http_server,string>"},
                    {"@type": "duumbi:ResultUnwrap", "@id": "duumbi:main/main/entry/invalid_server", "duumbi:operand": {"@id": "duumbi:main/main/entry/invalid_new"}, "duumbi:resultType": "http_server"},
                    {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/bad_path", "duumbi:value": "health", "duumbi:resultType": "string"},
                    {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/headers2_src", "duumbi:value": "{}", "duumbi:resultType": "string"},
                    {"@type": "duumbi:JsonParse", "@id": "duumbi:main/main/entry/headers2_parse", "duumbi:operand": {"@id": "duumbi:main/main/entry/headers2_src"}, "duumbi:resultType": "result<json,string>"},
                    {"@type": "duumbi:ResultUnwrap", "@id": "duumbi:main/main/entry/headers2", "duumbi:operand": {"@id": "duumbi:main/main/entry/headers2_parse"}, "duumbi:resultType": "json"},
                    {"@type": "duumbi:RouteAddStatic", "@id": "duumbi:main/main/entry/invalid_route", "duumbi:operand": {"@id": "duumbi:main/main/entry/invalid_server"}, "duumbi:args": [
                        {"@id": "duumbi:main/main/entry/get"}, {"@id": "duumbi:main/main/entry/bad_path"}, {"@id": "duumbi:main/main/entry/status"}, {"@id": "duumbi:main/main/entry/headers2"}, {"@id": "duumbi:main/main/entry/body"}
                    ], "duumbi:resultType": "result<i64,string>"},
                    {"@type": "duumbi:ResultUnwrapErr", "@id": "duumbi:main/main/entry/invalid_route_err", "duumbi:operand": {"@id": "duumbi:main/main/entry/invalid_route"}, "duumbi:resultType": "string"},
                    {"@type": "duumbi:ServerClose", "@id": "duumbi:main/main/entry/invalid_close", "duumbi:operand": {"@id": "duumbi:main/main/entry/invalid_server"}, "duumbi:resultType": "result<i64,string>"},
                    {"@type": "duumbi:ResultUnwrap", "@id": "duumbi:main/main/entry/invalid_close_unwrap", "duumbi:operand": {"@id": "duumbi:main/main/entry/invalid_close"}, "duumbi:resultType": "i64"},

                    {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/closed_port", "duumbi:value": i64::from(closed_port), "duumbi:resultType": "i64"},
                    {"@type": "duumbi:ServerNew", "@id": "duumbi:main/main/entry/closed_new", "duumbi:operand": {"@id": "duumbi:main/main/entry/host"}, "duumbi:left": {"@id": "duumbi:main/main/entry/closed_port"}, "duumbi:right": {"@id": "duumbi:main/main/entry/init_timeout"}, "duumbi:resultType": "result<http_server,string>"},
                    {"@type": "duumbi:ResultUnwrap", "@id": "duumbi:main/main/entry/closed_server", "duumbi:operand": {"@id": "duumbi:main/main/entry/closed_new"}, "duumbi:resultType": "http_server"},
                    {"@type": "duumbi:ServerClose", "@id": "duumbi:main/main/entry/closed_close", "duumbi:operand": {"@id": "duumbi:main/main/entry/closed_server"}, "duumbi:resultType": "result<i64,string>"},
                    {"@type": "duumbi:ResultUnwrap", "@id": "duumbi:main/main/entry/closed_close_unwrap", "duumbi:operand": {"@id": "duumbi:main/main/entry/closed_close"}, "duumbi:resultType": "i64"},
                    {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/headers3_src", "duumbi:value": "{}", "duumbi:resultType": "string"},
                    {"@type": "duumbi:JsonParse", "@id": "duumbi:main/main/entry/headers3_parse", "duumbi:operand": {"@id": "duumbi:main/main/entry/headers3_src"}, "duumbi:resultType": "result<json,string>"},
                    {"@type": "duumbi:ResultUnwrap", "@id": "duumbi:main/main/entry/headers3", "duumbi:operand": {"@id": "duumbi:main/main/entry/headers3_parse"}, "duumbi:resultType": "json"},
                    {"@type": "duumbi:RouteAddStatic", "@id": "duumbi:main/main/entry/closed_route", "duumbi:operand": {"@id": "duumbi:main/main/entry/closed_server"}, "duumbi:args": [
                        {"@id": "duumbi:main/main/entry/get"}, {"@id": "duumbi:main/main/entry/health"}, {"@id": "duumbi:main/main/entry/status"}, {"@id": "duumbi:main/main/entry/headers3"}, {"@id": "duumbi:main/main/entry/body"}
                    ], "duumbi:resultType": "result<i64,string>"},
                    {"@type": "duumbi:ResultUnwrapErr", "@id": "duumbi:main/main/entry/closed_route_err", "duumbi:operand": {"@id": "duumbi:main/main/entry/closed_route"}, "duumbi:resultType": "string"},
                    {"@type": "duumbi:ServerStart", "@id": "duumbi:main/main/entry/closed_start", "duumbi:operand": {"@id": "duumbi:main/main/entry/closed_server"}, "duumbi:left": {"@id": "duumbi:main/main/entry/max_one"}, "duumbi:right": {"@id": "duumbi:main/main/entry/short_timeout"}, "duumbi:resultType": "result<i64,string>"},
                    {"@type": "duumbi:ResultUnwrapErr", "@id": "duumbi:main/main/entry/closed_start_err", "duumbi:operand": {"@id": "duumbi:main/main/entry/closed_start"}, "duumbi:resultType": "string"},
                    {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/return_zero", "duumbi:value": 0, "duumbi:resultType": "i64"},
                    {"@type": "duumbi:Return", "@id": "duumbi:main/main/entry/return", "duumbi:operand": {"@id": "duumbi:main/main/entry/return_zero"}}
                ]
            }]
        }]
    })
}

fn http_get_health(port: u16) -> Result<String, String> {
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut last_error = None::<String>;

    while Instant::now() < deadline {
        match TcpStream::connect(("127.0.0.1", port)) {
            Ok(mut stream) => {
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .expect("set read timeout");
                stream
                    .write_all(
                        b"GET /health HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
                    )
                    .expect("write request");
                let mut bytes = Vec::new();
                let mut buf = [0_u8; 512];
                loop {
                    match stream.read(&mut buf) {
                        Ok(0) => break,
                        Ok(n) => bytes.extend_from_slice(&buf[..n]),
                        Err(error)
                            if error.kind() == ErrorKind::ConnectionReset && !bytes.is_empty() =>
                        {
                            break;
                        }
                        Err(error) => return Err(format!("read response: {error}")),
                    }
                }
                let response = String::from_utf8(bytes).expect("response should be utf8");
                return Ok(response);
            }
            Err(error) => {
                last_error = Some(error.to_string());
                std::thread::sleep(Duration::from_millis(20));
            }
        }
    }

    Err(format!(
        "server did not return a loopback response: {last_error:?}"
    ))
}

fn wait_with_timeout(mut child: std::process::Child, timeout: Duration) -> Output {
    let deadline = Instant::now() + timeout;
    loop {
        if child.try_wait().expect("poll child").is_some() {
            return child.wait_with_output().expect("collect child output");
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let output = child
                .wait_with_output()
                .expect("collect killed child output");
            panic!(
                "compiled server fixture timed out\nstdout:\n{}\nstderr:\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[tokio::test]
async fn embedded_registry_clean_workspace_import_build_run_server() {
    let (registry_url, token, _registry_tmp) = start_test_server().await;
    let publisher = registry_client(&registry_url, Some(&token));
    let public_client = registry_client(&registry_url, None);

    let package_ws = tempfile::TempDir::new().expect("package workspace");
    make_server_package(package_ws.path());
    let tarball = duumbi::registry::package::pack_module(package_ws.path()).expect("pack server");
    let published = publisher
        .publish("local", "@duumbi/stdlib-server", &tarball)
        .await
        .expect("publish server to embedded registry");
    assert_eq!(published.name, "@duumbi/stdlib-server");
    assert_eq!(published.version, "1.0.0");

    let search = public_client
        .search("local", "stdlib-server")
        .await
        .expect("search embedded registry");
    assert!(
        search
            .results
            .iter()
            .any(|hit| { hit.name == "@duumbi/stdlib-server" && hit.latest_version == "1.0.0" })
    );

    let workspace = tempfile::TempDir::new().expect("clean workspace");
    run_duumbi_init(workspace.path());
    assert_default_dependency_policy(workspace.path());
    assert!(
        workspace
            .path()
            .join(".duumbi/cache/@duumbi/stdlib-server@1.0.0/graph/server.jsonld")
            .exists(),
        "server may be cached for explicit dependency use"
    );

    configure_local_registry(workspace.path(), &registry_url);
    let cache_dir = workspace.path().join(".duumbi/cache");
    let manifest = public_client
        .download_module("local", "@duumbi/stdlib-server", "1.0.0", &cache_dir)
        .await
        .expect("download server module without credentials");
    assert_eq!(manifest.module.name, "@duumbi/stdlib-server");
    assert_eq!(manifest.module.version, "1.0.0");

    let cache_entry = cache_dir.join("@duumbi/stdlib-server@1.0.0");
    assert!(cache_entry.join("manifest.toml").exists());
    assert!(cache_entry.join("graph/server.jsonld").exists());
    assert!(cache_entry.join(".integrity").exists());

    add_downloaded_server_dependency(workspace.path());

    let port = unused_port();
    write_main_graph(workspace.path(), server_importer_graph(port));
    let output_path = workspace_output_path(workspace.path());
    build_workspace(workspace.path(), &output_path, false).expect("build clean importer");

    let mut child = Command::new(&output_path)
        .current_dir(workspace.path())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn server fixture");

    let response = match http_get_health(port) {
        Ok(response) => response,
        Err(error) => {
            let _ = child.kill();
            let run = child.wait_with_output().expect("collect child output");
            panic!(
                "{error}\nstdout:\n{}\nstderr:\n{}",
                String::from_utf8_lossy(&run.stdout),
                String::from_utf8_lossy(&run.stderr)
            );
        }
    };
    assert!(
        response.starts_with("HTTP/1.1 200 OK"),
        "unexpected response: {response:?}"
    );
    assert!(response.contains("Content-Length: 2"));
    assert!(response.ends_with("ok"));

    let run = wait_with_timeout(child, Duration::from_secs(3));
    assert!(
        run.status.success(),
        "server fixture failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
}

#[test]
fn compiled_server_negative_paths_are_bounded_and_visible() {
    let workspace = tempfile::TempDir::new().expect("negative workspace");
    write_main_graph(
        workspace.path(),
        server_negative_graph(unused_port(), unused_port(), unused_port()),
    );

    let output_path = workspace_output_path(workspace.path());
    build_workspace(workspace.path(), &output_path, false).expect("build negative fixture");
    let run = run_workspace_binary(workspace.path(), &[]).expect("run negative fixture");

    assert_eq!(run.exit_code, 0);
    assert_eq!(run.stdout.trim(), "");
    assert_eq!(run.stderr.trim(), "");
}
