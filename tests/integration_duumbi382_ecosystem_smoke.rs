//! DUUMBI-382 ecosystem smoke harness evidence.
//!
//! This file covers the embedded-registry harness, required-module matrix, and
//! per-module clean-workspace import/build/run smoke tests.

use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::io::{ErrorKind, Read as _, Write as _};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Output, Stdio};
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
const PRODUCTION_REGISTRY_URL: &str = "https://registry.duumbi.dev";

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

fn normalized_stdout(stdout: &str) -> String {
    stdout.trim().replace("\r\n", "\n")
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

    let output = child
        .wait_with_output()
        .expect("collect duumbi command output");

    DuumbiOutput {
        args: args.iter().map(|arg| (*arg).to_string()).collect(),
        status: output.status,
        stdout: output.stdout,
        stderr: output.stderr,
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

fn const_string(name: &str, value: &str) -> Value {
    json!({
        "@type": "duumbi:Const",
        "@id": main_id(name),
        "duumbi:value": value,
        "duumbi:resultType": "string"
    })
}

fn const_i64(name: &str, value: i64) -> Value {
    json!({
        "@type": "duumbi:Const",
        "@id": main_id(name),
        "duumbi:value": value,
        "duumbi:resultType": "i64"
    })
}

fn module_call(
    name: &str,
    module: &str,
    function: &str,
    args: Vec<Value>,
    result_type: &str,
) -> Value {
    json!({
        "@type": "duumbi:Call",
        "@id": main_id(name),
        "duumbi:module": module,
        "duumbi:function": function,
        "duumbi:args": args,
        "duumbi:resultType": result_type
    })
}

fn result_unwrap(name: &str, result_name: &str, result_type: &str) -> Value {
    json!({
        "@type": "duumbi:ResultUnwrap",
        "@id": main_id(name),
        "duumbi:operand": node_ref(&main_id(result_name)),
        "duumbi:resultType": result_type
    })
}

fn result_is_ok(name: &str, result_name: &str) -> Value {
    json!({
        "@type": "duumbi:ResultIsOk",
        "@id": main_id(name),
        "duumbi:operand": node_ref(&main_id(result_name))
    })
}

fn print_value(name: &str, value_name: &str) -> Value {
    json!({
        "@type": "duumbi:Print",
        "@id": main_id(name),
        "duumbi:operand": node_ref(&main_id(value_name))
    })
}

fn print_string(name: &str, value_name: &str) -> Value {
    json!({
        "@type": "duumbi:PrintString",
        "@id": main_id(name),
        "duumbi:operand": node_ref(&main_id(value_name))
    })
}

fn return_zero_ops() -> Vec<Value> {
    vec![
        const_i64("return_zero", 0),
        json!({
            "@type": "duumbi:Return",
            "@id": main_id("return"),
            "duumbi:operand": node_ref(&main_id("return_zero"))
        }),
    ]
}

fn array_new(name: &str) -> Value {
    json!({
        "@type": "duumbi:ArrayNew",
        "@id": main_id(name),
        "duumbi:resultType": "array<string>"
    })
}

fn array_push(array_name: &str, element_name: &str, op_name: &str) -> Value {
    json!({
        "@type": "duumbi:ArrayPush",
        "@id": main_id(op_name),
        "duumbi:array": node_ref(&main_id(array_name)),
        "duumbi:element": node_ref(&main_id(element_name))
    })
}

fn params_ops(prefix: &str, values: &[&str]) -> Vec<Value> {
    let mut ops = vec![array_new(&format!("{prefix}_params"))];
    for (idx, value) in values.iter().enumerate() {
        let value_name = format!("{prefix}_param_{idx}");
        ops.push(const_string(&value_name, value));
        ops.push(array_push(
            &format!("{prefix}_params"),
            &value_name,
            &format!("{prefix}_push_{idx}"),
        ));
    }
    ops
}

fn import(module: &str, path: &str, functions: &[&str]) -> Value {
    json!({
        "duumbi:module": module,
        "duumbi:path": path,
        "duumbi:functions": functions
    })
}

fn main_module(imports: Vec<Value>, mut ops: Vec<Value>) -> Value {
    ops.extend(return_zero_ops());
    json!({
        "@context": {"duumbi": "https://duumbi.dev/ns/core#"},
        "@type": "duumbi:Module",
        "@id": "duumbi:main",
        "duumbi:name": "main",
        "duumbi:imports": imports,
        "duumbi:functions": [{
            "@type": "duumbi:Function",
            "@id": "duumbi:main/main",
            "duumbi:name": "main",
            "duumbi:returnType": "i64",
            "duumbi:blocks": [{
                "@type": "duumbi:Block",
                "@id": "duumbi:main/main/entry",
                "duumbi:label": "entry",
                "duumbi:ops": ops
            }]
        }]
    })
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

fn json_behavior_graph() -> Value {
    main_module(
        vec![import(
            "json",
            "@duumbi/stdlib-json",
            &["parse", "stringify", "get_field", "array_len"],
        )],
        vec![
            const_string("input", r#"{"name":"duumbi","items":[10,20,30]}"#),
            module_call(
                "parse_result",
                "json",
                "parse",
                vec![node_ref(&main_id("input"))],
                "result<json,string>",
            ),
            result_unwrap("root", "parse_result", "json"),
            const_string("name_key", "name"),
            module_call(
                "name_result",
                "json",
                "get_field",
                vec![node_ref(&main_id("root")), node_ref(&main_id("name_key"))],
                "result<json,string>",
            ),
            result_unwrap("name_json", "name_result", "json"),
            module_call(
                "name_string_result",
                "json",
                "stringify",
                vec![node_ref(&main_id("name_json"))],
                "result<string,string>",
            ),
            result_unwrap("name_string", "name_string_result", "string"),
            print_string("print_name", "name_string"),
            const_string("items_key", "items"),
            module_call(
                "items_result",
                "json",
                "get_field",
                vec![node_ref(&main_id("root")), node_ref(&main_id("items_key"))],
                "result<json,string>",
            ),
            result_unwrap("items_json", "items_result", "json"),
            module_call(
                "items_len_result",
                "json",
                "array_len",
                vec![node_ref(&main_id("items_json"))],
                "result<i64,string>",
            ),
            result_unwrap("items_len", "items_len_result", "i64"),
            print_value("print_len", "items_len"),
        ],
    )
}

fn file_behavior_graph() -> Value {
    main_module(
        vec![import(
            "file",
            "@duumbi/stdlib-file",
            &["path_join", "write_file", "file_exists", "read_file"],
        )],
        vec![
            const_string("left", "."),
            const_string("right", "duumbi382-file-smoke.txt"),
            module_call(
                "path_result",
                "file",
                "path_join",
                vec![node_ref(&main_id("left")), node_ref(&main_id("right"))],
                "result<string,string>",
            ),
            result_unwrap("path", "path_result", "string"),
            const_string("contents", "hello-file"),
            module_call(
                "write_result",
                "file",
                "write_file",
                vec![node_ref(&main_id("path")), node_ref(&main_id("contents"))],
                "result<i64,string>",
            ),
            result_unwrap("written", "write_result", "i64"),
            module_call(
                "exists_result",
                "file",
                "file_exists",
                vec![node_ref(&main_id("path"))],
                "result<bool,string>",
            ),
            result_unwrap("exists", "exists_result", "bool"),
            print_value("print_exists", "exists"),
            const_i64("max_bytes", 64),
            module_call(
                "read_result",
                "file",
                "read_file",
                vec![node_ref(&main_id("path")), node_ref(&main_id("max_bytes"))],
                "result<string,string>",
            ),
            result_unwrap("read_contents", "read_result", "string"),
            print_string("print_contents", "read_contents"),
        ],
    )
}

fn net_client_graph(port: u16) -> Value {
    main_module(
        vec![import(
            "net",
            "@duumbi/stdlib-net",
            &["tcp_connect", "tcp_write", "tcp_read", "tcp_close"],
        )],
        vec![
            const_string("host", "127.0.0.1"),
            const_i64("port", i64::from(port)),
            const_i64("timeout", 2000),
            module_call(
                "connect_result",
                "net",
                "tcp_connect",
                vec![
                    node_ref(&main_id("host")),
                    node_ref(&main_id("port")),
                    node_ref(&main_id("timeout")),
                ],
                "result<tcp_socket,string>",
            ),
            result_unwrap("socket", "connect_result", "tcp_socket"),
            const_string("payload", "ping"),
            module_call(
                "write_result",
                "net",
                "tcp_write",
                vec![
                    node_ref(&main_id("socket")),
                    node_ref(&main_id("payload")),
                    node_ref(&main_id("timeout")),
                ],
                "result<i64,string>",
            ),
            result_unwrap("write_len", "write_result", "i64"),
            print_value("print_write_len", "write_len"),
            const_i64("max_bytes", 4),
            module_call(
                "read_result",
                "net",
                "tcp_read",
                vec![
                    node_ref(&main_id("socket")),
                    node_ref(&main_id("max_bytes")),
                    node_ref(&main_id("timeout")),
                ],
                "result<string,string>",
            ),
            result_unwrap("read_body", "read_result", "string"),
            print_string("print_read_body", "read_body"),
            module_call(
                "close_result",
                "net",
                "tcp_close",
                vec![node_ref(&main_id("socket"))],
                "result<i64,string>",
            ),
            result_is_ok("close_ok", "close_result"),
            print_value("print_close_ok", "close_ok"),
        ],
    )
}

fn http_get_graph(port: u16) -> Value {
    let url = format!("http://127.0.0.1:{port}/health");
    main_module(
        vec![import(
            "http",
            "@duumbi/stdlib-http",
            &["http_get", "http_status", "http_body", "http_response_free"],
        )],
        vec![
            const_string("url", &url),
            const_string("headers_text", "{}"),
            json!({
                "@type": "duumbi:JsonParse",
                "@id": main_id("headers_result"),
                "duumbi:operand": node_ref(&main_id("headers_text")),
                "duumbi:resultType": "result<json,string>"
            }),
            result_unwrap("headers", "headers_result", "json"),
            const_i64("timeout", 2000),
            module_call(
                "get_result",
                "http",
                "http_get",
                vec![
                    node_ref(&main_id("url")),
                    node_ref(&main_id("headers")),
                    node_ref(&main_id("timeout")),
                ],
                "result<http_response,string>",
            ),
            result_unwrap("response", "get_result", "http_response"),
            module_call(
                "status_result",
                "http",
                "http_status",
                vec![node_ref(&main_id("response"))],
                "result<i64,string>",
            ),
            result_unwrap("status", "status_result", "i64"),
            print_value("print_status", "status"),
            module_call(
                "body_result",
                "http",
                "http_body",
                vec![node_ref(&main_id("response"))],
                "result<string,string>",
            ),
            result_unwrap("body", "body_result", "string"),
            print_string("print_body", "body"),
            module_call(
                "free_result",
                "http",
                "http_response_free",
                vec![node_ref(&main_id("response"))],
                "result<i64,string>",
            ),
            result_unwrap("free_code", "free_result", "i64"),
        ],
    )
}

fn db_memory_graph() -> Value {
    let mut ops = vec![
        const_string("path", ":memory:"),
        module_call(
            "open_result",
            "db",
            "db_open",
            vec![node_ref(&main_id("path"))],
            "result<db_connection,string>",
        ),
        result_unwrap("conn", "open_result", "db_connection"),
    ];
    ops.extend(params_ops("create", &[]));
    ops.extend([
        const_string("create_sql", "create table users(name text not null)"),
        module_call(
            "create_result",
            "db",
            "db_execute",
            vec![
                node_ref(&main_id("conn")),
                node_ref(&main_id("create_sql")),
                node_ref(&main_id("create_params")),
            ],
            "result<i64,string>",
        ),
        result_unwrap("create_changed", "create_result", "i64"),
        print_value("print_create_changed", "create_changed"),
    ]);
    ops.extend(params_ops("insert", &["Ada"]));
    ops.extend([
        const_string("insert_sql", "insert into users(name) values (?)"),
        module_call(
            "insert_result",
            "db",
            "db_execute",
            vec![
                node_ref(&main_id("conn")),
                node_ref(&main_id("insert_sql")),
                node_ref(&main_id("insert_params")),
            ],
            "result<i64,string>",
        ),
        result_unwrap("insert_changed", "insert_result", "i64"),
        print_value("print_insert_changed", "insert_changed"),
    ]);
    ops.extend(params_ops("select", &["Ada"]));
    ops.extend([
        const_string("select_sql", "select name from users where name = ?"),
        module_call(
            "query_result",
            "db",
            "db_query",
            vec![
                node_ref(&main_id("conn")),
                node_ref(&main_id("select_sql")),
                node_ref(&main_id("select_params")),
            ],
            "result<db_rows,string>",
        ),
        result_unwrap("rows", "query_result", "db_rows"),
        module_call(
            "rows_len_result",
            "db",
            "db_rows_len",
            vec![node_ref(&main_id("rows"))],
            "result<i64,string>",
        ),
        result_unwrap("rows_len", "rows_len_result", "i64"),
        print_value("print_rows_len", "rows_len"),
        const_i64("row_index", 0),
        const_string("column_name", "name"),
        module_call(
            "row_get_result",
            "db",
            "db_row_get",
            vec![
                node_ref(&main_id("rows")),
                node_ref(&main_id("row_index")),
                node_ref(&main_id("column_name")),
            ],
            "result<string,string>",
        ),
        result_unwrap("row_name", "row_get_result", "string"),
        print_string("print_row_name", "row_name"),
        module_call(
            "free_rows_result",
            "db",
            "db_rows_free",
            vec![node_ref(&main_id("rows"))],
            "result<i64,string>",
        ),
        result_unwrap("free_rows_code", "free_rows_result", "i64"),
        module_call(
            "close_result",
            "db",
            "db_close",
            vec![node_ref(&main_id("conn"))],
            "result<i64,string>",
        ),
        result_unwrap("close_code", "close_result", "i64"),
    ]);

    main_module(
        vec![import(
            "db",
            "@duumbi/stdlib-db",
            &[
                "db_open",
                "db_execute",
                "db_query",
                "db_rows_len",
                "db_row_get",
                "db_rows_free",
                "db_close",
            ],
        )],
        ops,
    )
}

fn server_health_graph(port: u16) -> Value {
    main_module(
        vec![import(
            "server",
            "@duumbi/stdlib-server",
            &[
                "server_new",
                "route_add_static",
                "server_start",
                "server_close",
            ],
        )],
        vec![
            const_string("host", "127.0.0.1"),
            const_i64("port", i64::from(port)),
            const_i64("init_timeout", 1000),
            module_call(
                "new_result",
                "server",
                "server_new",
                vec![
                    node_ref(&main_id("host")),
                    node_ref(&main_id("port")),
                    node_ref(&main_id("init_timeout")),
                ],
                "result<http_server,string>",
            ),
            result_unwrap("server", "new_result", "http_server"),
            const_string("method", "GET"),
            const_string("path", "/health"),
            const_i64("status", 200),
            const_string("headers_text", "{}"),
            json!({
                "@type": "duumbi:JsonParse",
                "@id": main_id("headers_result"),
                "duumbi:operand": node_ref(&main_id("headers_text")),
                "duumbi:resultType": "result<json,string>"
            }),
            result_unwrap("headers", "headers_result", "json"),
            const_string("body", "ok"),
            module_call(
                "route_result",
                "server",
                "route_add_static",
                vec![
                    node_ref(&main_id("server")),
                    node_ref(&main_id("method")),
                    node_ref(&main_id("path")),
                    node_ref(&main_id("status")),
                    node_ref(&main_id("headers")),
                    node_ref(&main_id("body")),
                ],
                "result<i64,string>",
            ),
            result_unwrap("route_code", "route_result", "i64"),
            const_i64("max_requests", 1),
            const_i64("serve_timeout", 2000),
            module_call(
                "start_result",
                "server",
                "server_start",
                vec![
                    node_ref(&main_id("server")),
                    node_ref(&main_id("max_requests")),
                    node_ref(&main_id("serve_timeout")),
                ],
                "result<i64,string>",
            ),
            result_unwrap("start_code", "start_result", "i64"),
            module_call(
                "close_result",
                "server",
                "server_close",
                vec![node_ref(&main_id("server"))],
                "result<i64,string>",
            ),
            result_unwrap("close_code", "close_result", "i64"),
        ],
    )
}

fn unused_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind unused port");
    listener.local_addr().expect("local addr").port()
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
                return String::from_utf8(bytes).map_err(|error| error.to_string());
            }
            Err(error) => {
                last_error = Some(error.to_string());
                thread::sleep(Duration::from_millis(20));
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
        if let Some(status) = child.try_wait().expect("poll child") {
            let mut stdout = Vec::new();
            if let Some(mut pipe) = child.stdout.take() {
                pipe.read_to_end(&mut stdout)
                    .expect("read child stdout after exit");
            }
            let mut stderr = Vec::new();
            if let Some(mut pipe) = child.stderr.take() {
                pipe.read_to_end(&mut stderr)
                    .expect("read child stderr after exit");
            }
            return Output {
                status,
                stdout,
                stderr,
            };
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            return child.wait_with_output().expect("collect killed output");
        }
        thread::sleep(Duration::from_millis(20));
    }
}

fn accept_with_timeout(listener: &TcpListener, timeout: Duration, context: &str) -> TcpStream {
    listener
        .set_nonblocking(true)
        .unwrap_or_else(|error| panic!("{context}: set nonblocking listener: {error}"));
    let deadline = Instant::now() + timeout;

    loop {
        match listener.accept() {
            Ok((stream, _)) => {
                stream
                    .set_nonblocking(false)
                    .unwrap_or_else(|error| panic!("{context}: restore blocking stream: {error}"));
                return stream;
            }
            Err(error) if error.kind() == ErrorKind::WouldBlock => {
                assert!(Instant::now() < deadline, "{context}: timed out");
                thread::sleep(Duration::from_millis(20));
            }
            Err(error) if error.kind() == ErrorKind::Interrupted => {}
            Err(error) => panic!("{context}: {error}"),
        }
    }
}

fn start_one_shot_http_server() -> (u16, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("loopback listener");
    let port = listener.local_addr().expect("local addr").port();
    let handle = thread::spawn(move || {
        let mut stream =
            accept_with_timeout(&listener, Duration::from_secs(30), "accept HTTP client");
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .expect("set HTTP fixture read timeout");
        stream
            .set_write_timeout(Some(Duration::from_secs(2)))
            .expect("set HTTP fixture write timeout");
        let mut buffer = [0_u8; 1024];
        let bytes_read = stream.read(&mut buffer).expect("read HTTP request");
        assert!(bytes_read > 0, "HTTP fixture received an empty request");
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok")
            .expect("write HTTP response");
    });
    (port, handle)
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

async fn clean_workspace_with_modules(modules: &[ModuleSpec]) -> tempfile::TempDir {
    let registry = start_embedded_registry().await;
    seed_registry(&registry, modules).await;

    let workspace = tempfile::TempDir::new().expect("workspace");
    init_workspace_with_registry(workspace.path(), &registry.url);
    for module in modules {
        deps_add_module(workspace.path(), *module);
    }
    workspace
}

fn build_and_run_graph(
    workspace: &Path,
    module: &'static str,
    graph: Value,
) -> duumbi::workspace::BinaryRunOutput {
    write_main_graph(workspace, graph);
    let output_path = workspace_output_path(workspace);
    build_workspace(workspace, &output_path, false).unwrap_or_else(|error| {
        panic!(
            "{}",
            StageFailure::new(module, SmokeStage::Build, error.to_string())
        )
    });
    run_workspace_binary(workspace, &[]).unwrap_or_else(|error| {
        panic!(
            "{}",
            StageFailure::new(module, SmokeStage::Run, error.to_string())
        )
    })
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
    assert_eq!(normalized_stdout(&run.stdout), "7\n6");
    assert_eq!(run.stderr.trim(), "");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn installed_json_module_import_build_and_run_from_clean_workspace() {
    let workspace = clean_workspace_with_modules(&[REQUIRED_MODULES[5]]).await;
    let run = build_and_run_graph(
        workspace.path(),
        "@duumbi/stdlib-json",
        json_behavior_graph(),
    );

    assert_eq!(run.exit_code, 0);
    assert_eq!(normalized_stdout(&run.stdout), "\"duumbi\"\n3");
    assert_eq!(run.stderr.trim(), "");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn installed_file_module_uses_temporary_workspace_storage() {
    let workspace = clean_workspace_with_modules(&[REQUIRED_MODULES[4]]).await;
    let run = build_and_run_graph(
        workspace.path(),
        "@duumbi/stdlib-file",
        file_behavior_graph(),
    );

    assert_eq!(run.exit_code, 0);
    assert_eq!(normalized_stdout(&run.stdout), "true\nhello-file");
    assert_eq!(run.stderr.trim(), "");
    assert!(
        workspace.path().join("duumbi382-file-smoke.txt").exists(),
        "{}",
        StageFailure::new(
            "@duumbi/stdlib-file",
            SmokeStage::Run,
            "compiled file smoke did not create the expected workspace file"
        )
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn installed_db_module_uses_temporary_memory_storage() {
    let workspace = clean_workspace_with_modules(&[REQUIRED_MODULES[8]]).await;
    let run = build_and_run_graph(workspace.path(), "@duumbi/stdlib-db", db_memory_graph());

    assert_eq!(run.exit_code, 0);
    assert_eq!(normalized_stdout(&run.stdout), "0\n1\n1\nAda");
    assert_eq!(run.stderr.trim(), "");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn installed_http_module_uses_loopback_fixture_only() {
    let (port, server) = start_one_shot_http_server();
    let workspace = clean_workspace_with_modules(&[REQUIRED_MODULES[7]]).await;
    let run = build_and_run_graph(
        workspace.path(),
        "@duumbi/stdlib-http",
        http_get_graph(port),
    );
    server.join().expect("HTTP fixture joins");

    assert_eq!(run.exit_code, 0);
    assert_eq!(normalized_stdout(&run.stdout), "200\nok");
    assert_eq!(run.stderr.trim(), "");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn installed_net_module_uses_loopback_and_explicit_timeouts() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("loopback listener");
    let port = listener.local_addr().expect("local addr").port();
    let server = thread::spawn(move || {
        let mut stream =
            accept_with_timeout(&listener, Duration::from_secs(30), "accept echo client");
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .expect("set echo read timeout");
        stream
            .set_write_timeout(Some(Duration::from_secs(2)))
            .expect("set echo write timeout");
        let mut buf = [0_u8; 4];
        stream.read_exact(&mut buf).expect("read ping");
        assert_eq!(&buf, b"ping");
        stream.write_all(&buf).expect("write echo");
    });

    let workspace = clean_workspace_with_modules(&[REQUIRED_MODULES[6]]).await;
    let run = build_and_run_graph(
        workspace.path(),
        "@duumbi/stdlib-net",
        net_client_graph(port),
    );
    server.join().expect("TCP fixture joins");

    assert_eq!(run.exit_code, 0);
    assert_eq!(normalized_stdout(&run.stdout), "4\nping\ntrue");
    assert_eq!(run.stderr.trim(), "");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn installed_server_module_exits_after_one_loopback_request() {
    let workspace = clean_workspace_with_modules(&[REQUIRED_MODULES[9]]).await;
    let port = unused_port();
    write_main_graph(workspace.path(), server_health_graph(port));

    let output_path = workspace_output_path(workspace.path());
    build_workspace(workspace.path(), &output_path, false).unwrap_or_else(|error| {
        panic!(
            "{}",
            StageFailure::new(
                "@duumbi/stdlib-server",
                SmokeStage::Build,
                error.to_string()
            )
        )
    });

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
                "{}",
                StageFailure::new(
                    "@duumbi/stdlib-server",
                    SmokeStage::Run,
                    format!(
                        "{error}\nstdout:\n{}\nstderr:\n{}",
                        String::from_utf8_lossy(&run.stdout),
                        String::from_utf8_lossy(&run.stderr)
                    )
                )
            );
        }
    };
    assert!(
        response.starts_with("HTTP/1.1 200 OK"),
        "unexpected response: {response:?}"
    );
    assert!(response.ends_with("ok"));

    let run = wait_with_timeout(child, Duration::from_secs(3));
    assert!(
        run.status.success(),
        "{}",
        StageFailure::new(
            "@duumbi/stdlib-server",
            SmokeStage::Run,
            format!(
                "server fixture failed\nstdout:\n{}\nstderr:\n{}",
                String::from_utf8_lossy(&run.stdout),
                String::from_utf8_lossy(&run.stderr)
            )
        )
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "manual production registry smoke requires explicit DUUMBI_PRODUCTION_REGISTRY_SMOKE=1 opt-in"]
async fn production_registry_smoke_is_manual_gated_and_credentialed() {
    let opt_in = std::env::var("DUUMBI_PRODUCTION_REGISTRY_SMOKE").ok();
    let token = std::env::var("DUUMBI_PRODUCTION_REGISTRY_TOKEN")
        .ok()
        .or_else(|| std::env::var("DUUMBI_REGISTRY_TOKEN").ok());

    if production_smoke_decision(opt_in.as_deref(), token.as_deref())
        == ProductionSmokeDecision::Skipped
    {
        eprintln!(
            "production-gated: set DUUMBI_PRODUCTION_REGISTRY_SMOKE=1 and a production registry token to run"
        );
        return;
    }

    let registry_url = std::env::var("DUUMBI_PRODUCTION_REGISTRY_URL")
        .unwrap_or_else(|_| PRODUCTION_REGISTRY_URL.to_string());
    assert_eq!(
        registry_url.trim_end_matches('/'),
        PRODUCTION_REGISTRY_URL,
        "{}",
        StageFailure::new(
            "@duumbi/production-registry",
            SmokeStage::ProductionGuard,
            "production smoke may only target https://registry.duumbi.dev without a reviewed override"
        )
    );

    let client = registry_client(PRODUCTION_REGISTRY_URL, token.as_deref());
    let search = client
        .search("local", "stdlib")
        .await
        .unwrap_or_else(|error| {
            panic!(
                "{}",
                StageFailure::new(
                    "@duumbi/production-registry",
                    SmokeStage::Search,
                    error.to_string()
                )
            )
        });
    let search_hits: Vec<&str> = search
        .results
        .iter()
        .map(|result| result.name.as_str())
        .collect();

    let cache = tempfile::TempDir::new().expect("production smoke cache");
    for module in REQUIRED_MODULES {
        assert!(
            search_hits.contains(&module.module),
            "{}",
            StageFailure::new(
                module.module,
                SmokeStage::Search,
                "production search output did not list required module"
            )
        );

        let info = client
            .fetch_module_info("local", module.module)
            .await
            .unwrap_or_else(|error| {
                panic!(
                    "{}",
                    StageFailure::new(module.module, SmokeStage::Manifest, error.to_string())
                )
            });
        assert!(
            info.versions
                .iter()
                .any(|version| version.version == STDLIB_VERSION && !version.yanked),
            "{}",
            StageFailure::new(
                module.module,
                SmokeStage::Manifest,
                format!("missing non-yanked {STDLIB_VERSION} production version")
            )
        );

        let manifest = client
            .download_module("local", module.module, STDLIB_VERSION, cache.path())
            .await
            .unwrap_or_else(|error| {
                panic!(
                    "{}",
                    StageFailure::new(module.module, SmokeStage::Install, error.to_string())
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
                    format!("production manifest missing {expected_export} export")
                )
            );
        }
    }
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
