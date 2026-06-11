//! DUUMBI-380 HTTP + JSON + SQLite runtime composition E2E tests.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::Path;
use std::process::Command;
use std::thread;

use duumbi::compiler::{linker, lowering};
use duumbi::errors::DiagnosticLevel;
use duumbi::graph::builder::build_graph;
use duumbi::graph::validator::validate;
use duumbi::parser::parse_jsonld;
use serde_json::{Value, json};

fn node_ref(id: &str) -> Value {
    json!({ "@id": id })
}

fn id(name: &str) -> String {
    format!("duumbi:main/main/entry/{name}")
}

fn module_fixture(ops: Vec<Value>) -> String {
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
                "duumbi:ops": ops
            }]
        }]
    })
    .to_string()
}

fn const_string(name: &str, value: &str) -> Value {
    json!({
        "@type": "duumbi:Const",
        "@id": id(name),
        "duumbi:value": value,
        "duumbi:resultType": "string"
    })
}

fn const_i64(name: &str, value: i64) -> Value {
    json!({
        "@type": "duumbi:Const",
        "@id": id(name),
        "duumbi:value": value,
        "duumbi:resultType": "i64"
    })
}

fn array_new(name: &str) -> Value {
    json!({
        "@type": "duumbi:ArrayNew",
        "@id": id(name),
        "duumbi:resultType": "array<string>"
    })
}

fn array_push(array_name: &str, element_name: &str, op_name: &str) -> Value {
    json!({
        "@type": "duumbi:ArrayPush",
        "@id": id(op_name),
        "duumbi:array": node_ref(&id(array_name)),
        "duumbi:element": node_ref(&id(element_name))
    })
}

fn empty_params(prefix: &str) -> Vec<Value> {
    vec![array_new(&format!("{prefix}_params"))]
}

fn unwrap_result(op_name: &str, result_name: &str, result_type: &str) -> Value {
    json!({
        "@type": "duumbi:ResultUnwrap",
        "@id": id(op_name),
        "duumbi:operand": node_ref(&id(result_name)),
        "duumbi:resultType": result_type
    })
}

fn print_i64(name: &str, operand_name: &str) -> Value {
    json!({
        "@type": "duumbi:Print",
        "@id": id(name),
        "duumbi:operand": node_ref(&id(operand_name))
    })
}

fn print_string(name: &str, operand_name: &str) -> Value {
    json!({
        "@type": "duumbi:PrintString",
        "@id": id(name),
        "duumbi:operand": node_ref(&id(operand_name))
    })
}

fn db_open_ops(prefix: &str, path: &str) -> Vec<Value> {
    vec![
        const_string(&format!("{prefix}_path"), path),
        json!({
            "@type": "duumbi:DbOpen",
            "@id": id(&format!("{prefix}_open_result")),
            "duumbi:operand": node_ref(&id(&format!("{prefix}_path"))),
            "duumbi:resultType": "result<db_connection,string>"
        }),
        unwrap_result(
            &format!("{prefix}_conn"),
            &format!("{prefix}_open_result"),
            "db_connection",
        ),
    ]
}

fn db_execute_ops(prefix: &str, conn_name: &str, sql: &str, params_name: &str) -> Vec<Value> {
    vec![
        const_string(&format!("{prefix}_sql"), sql),
        json!({
            "@type": "duumbi:DbExecute",
            "@id": id(&format!("{prefix}_execute_result")),
            "duumbi:operand": node_ref(&id(conn_name)),
            "duumbi:left": node_ref(&id(&format!("{prefix}_sql"))),
            "duumbi:right": node_ref(&id(params_name)),
            "duumbi:resultType": "result<i64,string>"
        }),
        unwrap_result(
            &format!("{prefix}_changed"),
            &format!("{prefix}_execute_result"),
            "i64",
        ),
        print_i64(
            &format!("{prefix}_print_changed"),
            &format!("{prefix}_changed"),
        ),
    ]
}

fn db_query_ops(prefix: &str, conn_name: &str, sql: &str, params_name: &str) -> Vec<Value> {
    vec![
        const_string(&format!("{prefix}_sql"), sql),
        json!({
            "@type": "duumbi:DbQuery",
            "@id": id(&format!("{prefix}_query_result")),
            "duumbi:operand": node_ref(&id(conn_name)),
            "duumbi:left": node_ref(&id(&format!("{prefix}_sql"))),
            "duumbi:right": node_ref(&id(params_name)),
            "duumbi:resultType": "result<db_rows,string>"
        }),
        unwrap_result(
            &format!("{prefix}_rows"),
            &format!("{prefix}_query_result"),
            "db_rows",
        ),
    ]
}

fn close_result_ops(prefix: &str, op_type: &str, operand_name: &str) -> Vec<Value> {
    vec![
        json!({
            "@type": op_type,
            "@id": id(&format!("{prefix}_result")),
            "duumbi:operand": node_ref(&id(operand_name)),
            "duumbi:resultType": "result<i64,string>"
        }),
        unwrap_result(
            &format!("{prefix}_code"),
            &format!("{prefix}_result"),
            "i64",
        ),
        print_i64(&format!("{prefix}_print_code"), &format!("{prefix}_code")),
    ]
}

fn return_zero_ops() -> Vec<Value> {
    vec![
        const_i64("return_zero", 0),
        json!({
            "@type": "duumbi:Return",
            "@id": id("return"),
            "duumbi:operand": node_ref(&id("return_zero"))
        }),
    ]
}

fn native_output_path(path: &Path) -> std::path::PathBuf {
    if path.exists() || std::env::consts::EXE_SUFFIX.is_empty() {
        return path.to_path_buf();
    }

    std::path::PathBuf::from(format!(
        "{}{}",
        path.display(),
        std::env::consts::EXE_SUFFIX
    ))
}

fn compile_fixture(json: &str, output_name: &str) -> std::path::PathBuf {
    let module = parse_jsonld(json).expect("fixture must parse");
    let graph = build_graph(&module).expect("fixture must build");
    let diagnostics = validate(&graph);
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.level == DiagnosticLevel::Error)
        .collect();
    assert!(errors.is_empty(), "validation errors: {errors:?}");

    let object = lowering::compile_to_object(&graph).expect("fixture must lower");
    let tmp_path = std::env::temp_dir().join("duumbi_380_e2e_tests");
    std::fs::create_dir_all(&tmp_path).expect("temp test dir must be creatable");
    let object_path = tmp_path.join(format!("{output_name}.o"));
    let runtime_o = tmp_path.join(format!("{output_name}_runtime.o"));
    let binary = tmp_path.join(output_name);

    std::fs::write(&object_path, object).expect("object must be writable");
    linker::compile_runtime(Path::new("runtime/duumbi_runtime.c"), &runtime_o)
        .expect("runtime must compile");
    linker::link(&object_path, &runtime_o, &binary).expect("binary must link");
    native_output_path(&binary)
}

fn local_json_server() -> (u16, thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("loopback listener");
    let port = listener.local_addr().expect("local addr").port();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept HTTP client");
        let mut request = [0_u8; 1024];
        let bytes = stream.read(&mut request).expect("read HTTP request");
        let request_text = String::from_utf8_lossy(&request[..bytes]).to_string();
        let body = r#"{"name":"duumbi","stars":3}"#;
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        stream
            .write_all(response.as_bytes())
            .expect("write HTTP response");
        request_text
            .lines()
            .next()
            .expect("request line")
            .to_string()
    });
    (port, server)
}

fn composition_fixture(port: u16) -> String {
    let mut ops = Vec::new();
    let url = format!("http://127.0.0.1:{port}/payload");

    ops.extend([
        const_string("url", &url),
        const_string("header_text", "{}"),
        json!({
            "@type": "duumbi:JsonParse",
            "@id": id("headers_result"),
            "duumbi:operand": node_ref(&id("header_text")),
            "duumbi:resultType": "result<json,string>"
        }),
        unwrap_result("headers", "headers_result", "json"),
        const_i64("timeout", 2000),
        json!({
            "@type": "duumbi:HttpGet",
            "@id": id("http_result"),
            "duumbi:operand": node_ref(&id("url")),
            "duumbi:left": node_ref(&id("headers")),
            "duumbi:right": node_ref(&id("timeout")),
            "duumbi:resultType": "result<http_response,string>"
        }),
        unwrap_result("response", "http_result", "http_response"),
        json!({
            "@type": "duumbi:HttpStatus",
            "@id": id("status_result"),
            "duumbi:operand": node_ref(&id("response")),
            "duumbi:resultType": "result<i64,string>"
        }),
        unwrap_result("status", "status_result", "i64"),
        print_i64("print_status", "status"),
        json!({
            "@type": "duumbi:HttpBody",
            "@id": id("body_result"),
            "duumbi:operand": node_ref(&id("response")),
            "duumbi:resultType": "result<string,string>"
        }),
        unwrap_result("body", "body_result", "string"),
        json!({
            "@type": "duumbi:JsonParse",
            "@id": id("json_result"),
            "duumbi:operand": node_ref(&id("body")),
            "duumbi:resultType": "result<json,string>"
        }),
        unwrap_result("json_doc", "json_result", "json"),
        const_string("name_key", "name"),
        json!({
            "@type": "duumbi:JsonGetField",
            "@id": id("name_result"),
            "duumbi:operand": node_ref(&id("json_doc")),
            "duumbi:left": node_ref(&id("name_key")),
            "duumbi:resultType": "result<json,string>"
        }),
        unwrap_result("name_json", "name_result", "json"),
        json!({
            "@type": "duumbi:JsonStringify",
            "@id": id("name_string_result"),
            "duumbi:operand": node_ref(&id("name_json")),
            "duumbi:resultType": "result<string,string>"
        }),
        unwrap_result("name_string", "name_string_result", "string"),
        print_string("print_name_json", "name_string"),
    ]);

    ops.extend(db_open_ops("mem", ":memory:"));
    ops.extend(empty_params("create"));
    ops.extend(db_execute_ops(
        "create",
        "mem_conn",
        "create table facts(name text not null)",
        "create_params",
    ));
    ops.extend([
        array_new("insert_params"),
        array_push("insert_params", "name_string", "insert_push_0"),
    ]);
    ops.extend(db_execute_ops(
        "insert",
        "mem_conn",
        "insert into facts(name) values (?)",
        "insert_params",
    ));
    ops.extend(empty_params("select"));
    ops.extend(db_query_ops(
        "select",
        "mem_conn",
        "select name from facts order by rowid",
        "select_params",
    ));
    ops.extend([
        json!({
            "@type": "duumbi:DbRowsLen",
            "@id": id("rows_len_result"),
            "duumbi:operand": node_ref(&id("select_rows")),
            "duumbi:resultType": "result<i64,string>"
        }),
        unwrap_result("rows_len", "rows_len_result", "i64"),
        print_i64("print_rows_len", "rows_len"),
        const_i64("row_index", 0),
        const_string("column_name", "name"),
        json!({
            "@type": "duumbi:DbRowGet",
            "@id": id("row_value_result"),
            "duumbi:operand": node_ref(&id("select_rows")),
            "duumbi:left": node_ref(&id("row_index")),
            "duumbi:right": node_ref(&id("column_name")),
            "duumbi:resultType": "result<string,string>"
        }),
        unwrap_result("row_value", "row_value_result", "string"),
        print_string("print_row_value", "row_value"),
    ]);
    ops.extend(close_result_ops(
        "rows_close",
        "duumbi:DbRowsFree",
        "select_rows",
    ));
    ops.extend(close_result_ops("db_close", "duumbi:DbClose", "mem_conn"));
    ops.extend(close_result_ops(
        "http_free",
        "duumbi:HttpResponseFree",
        "response",
    ));
    ops.extend(return_zero_ops());

    module_fixture(ops)
}

#[test]
fn http_json_sqlite_composition_e2e_round_trips_payload() {
    let (port, server) = local_json_server();
    let binary = compile_fixture(&composition_fixture(port), "duumbi380_http_json_sqlite_e2e");
    let output = Command::new(&binary)
        .output()
        .expect("compiled binary must run");

    let request_line = server.join().expect("server must finish");
    assert_eq!(request_line, "GET /payload HTTP/1.1");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(
        lines,
        [
            "200",
            "\"duumbi\"",
            "0",
            "1",
            "1",
            "\"duumbi\"",
            "0",
            "0",
            "0"
        ],
        "{stdout}"
    );
    assert!(
        output.status.success(),
        "fixture exited unsuccessfully\nstdout:\n{}\nstderr:\n{}",
        stdout,
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = std::fs::remove_file(&binary);
}
