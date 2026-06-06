//! DUUMBI-380 local HTTP runtime integration tests.

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
    let tmp_path = std::env::temp_dir().join("duumbi_380_http_tests");
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

fn http_get_fixture(port: u16) -> String {
    HTTP_GET_FIXTURE.replace("__PORT__", &port.to_string())
}

const HTTP_GET_FIXTURE: &str = r#"{
  "@context": {"duumbi": "https://duumbi.dev/ns/core#"},
  "@type": "duumbi:Module",
  "@id": "duumbi:main",
  "duumbi:name": "main",
  "duumbi:functions": [{"@type": "duumbi:Function", "@id": "duumbi:main/main", "duumbi:name": "main", "duumbi:returnType": "i64", "duumbi:blocks": [{"@type": "duumbi:Block", "@id": "duumbi:main/main/entry", "duumbi:label": "entry", "duumbi:ops": [
    {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/url", "duumbi:value": "http://127.0.0.1:__PORT__/hello", "duumbi:resultType": "string"},
    {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/header_text", "duumbi:value": "{}", "duumbi:resultType": "string"},
    {"@type": "duumbi:JsonParse", "@id": "duumbi:main/main/entry/header_result", "duumbi:operand": {"@id": "duumbi:main/main/entry/header_text"}, "duumbi:resultType": "result<json,string>"},
    {"@type": "duumbi:ResultIsOk", "@id": "duumbi:main/main/entry/header_ok", "duumbi:operand": {"@id": "duumbi:main/main/entry/header_result"}},
    {"@type": "duumbi:ResultUnwrap", "@id": "duumbi:main/main/entry/headers", "duumbi:operand": {"@id": "duumbi:main/main/entry/header_result"}, "duumbi:resultType": "json"},
    {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/timeout", "duumbi:value": 2000, "duumbi:resultType": "i64"},
    {"@type": "duumbi:HttpGet", "@id": "duumbi:main/main/entry/http_get", "duumbi:operand": {"@id": "duumbi:main/main/entry/url"}, "duumbi:left": {"@id": "duumbi:main/main/entry/headers"}, "duumbi:right": {"@id": "duumbi:main/main/entry/timeout"}, "duumbi:resultType": "result<http_response,string>"},
    {"@type": "duumbi:ResultIsOk", "@id": "duumbi:main/main/entry/http_ok", "duumbi:operand": {"@id": "duumbi:main/main/entry/http_get"}},
    {"@type": "duumbi:ResultUnwrap", "@id": "duumbi:main/main/entry/response", "duumbi:operand": {"@id": "duumbi:main/main/entry/http_get"}, "duumbi:resultType": "http_response"},
    {"@type": "duumbi:HttpStatus", "@id": "duumbi:main/main/entry/status_result", "duumbi:operand": {"@id": "duumbi:main/main/entry/response"}, "duumbi:resultType": "result<i64,string>"},
    {"@type": "duumbi:ResultIsOk", "@id": "duumbi:main/main/entry/status_ok", "duumbi:operand": {"@id": "duumbi:main/main/entry/status_result"}},
    {"@type": "duumbi:ResultUnwrap", "@id": "duumbi:main/main/entry/status", "duumbi:operand": {"@id": "duumbi:main/main/entry/status_result"}, "duumbi:resultType": "i64"},
    {"@type": "duumbi:Print", "@id": "duumbi:main/main/entry/print_status", "duumbi:operand": {"@id": "duumbi:main/main/entry/status"}},
    {"@type": "duumbi:HttpBody", "@id": "duumbi:main/main/entry/body_result", "duumbi:operand": {"@id": "duumbi:main/main/entry/response"}, "duumbi:resultType": "result<string,string>"},
    {"@type": "duumbi:ResultIsOk", "@id": "duumbi:main/main/entry/body_ok", "duumbi:operand": {"@id": "duumbi:main/main/entry/body_result"}},
    {"@type": "duumbi:ResultUnwrap", "@id": "duumbi:main/main/entry/body", "duumbi:operand": {"@id": "duumbi:main/main/entry/body_result"}, "duumbi:resultType": "string"},
    {"@type": "duumbi:PrintString", "@id": "duumbi:main/main/entry/print_body", "duumbi:operand": {"@id": "duumbi:main/main/entry/body"}},
    {"@type": "duumbi:HttpHeaders", "@id": "duumbi:main/main/entry/response_headers_result", "duumbi:operand": {"@id": "duumbi:main/main/entry/response"}, "duumbi:resultType": "result<json,string>"},
    {"@type": "duumbi:ResultIsOk", "@id": "duumbi:main/main/entry/response_headers_ok", "duumbi:operand": {"@id": "duumbi:main/main/entry/response_headers_result"}},
    {"@type": "duumbi:ResultUnwrap", "@id": "duumbi:main/main/entry/response_headers", "duumbi:operand": {"@id": "duumbi:main/main/entry/response_headers_result"}, "duumbi:resultType": "json"},
    {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/header_key", "duumbi:value": "x-duumbi-test", "duumbi:resultType": "string"},
    {"@type": "duumbi:JsonGetField", "@id": "duumbi:main/main/entry/header_value_result", "duumbi:operand": {"@id": "duumbi:main/main/entry/response_headers"}, "duumbi:left": {"@id": "duumbi:main/main/entry/header_key"}, "duumbi:resultType": "result<json,string>"},
    {"@type": "duumbi:ResultIsOk", "@id": "duumbi:main/main/entry/header_value_ok", "duumbi:operand": {"@id": "duumbi:main/main/entry/header_value_result"}},
    {"@type": "duumbi:ResultUnwrap", "@id": "duumbi:main/main/entry/header_value", "duumbi:operand": {"@id": "duumbi:main/main/entry/header_value_result"}, "duumbi:resultType": "json"},
    {"@type": "duumbi:JsonStringify", "@id": "duumbi:main/main/entry/header_string_result", "duumbi:operand": {"@id": "duumbi:main/main/entry/header_value"}, "duumbi:resultType": "result<string,string>"},
    {"@type": "duumbi:ResultIsOk", "@id": "duumbi:main/main/entry/header_string_ok", "duumbi:operand": {"@id": "duumbi:main/main/entry/header_string_result"}},
    {"@type": "duumbi:ResultUnwrap", "@id": "duumbi:main/main/entry/header_string", "duumbi:operand": {"@id": "duumbi:main/main/entry/header_string_result"}, "duumbi:resultType": "string"},
    {"@type": "duumbi:PrintString", "@id": "duumbi:main/main/entry/print_header", "duumbi:operand": {"@id": "duumbi:main/main/entry/header_string"}},
    {"@type": "duumbi:HttpResponseFree", "@id": "duumbi:main/main/entry/free_result", "duumbi:operand": {"@id": "duumbi:main/main/entry/response"}, "duumbi:resultType": "result<i64,string>"},
    {"@type": "duumbi:ResultIsOk", "@id": "duumbi:main/main/entry/free_ok", "duumbi:operand": {"@id": "duumbi:main/main/entry/free_result"}},
    {"@type": "duumbi:Print", "@id": "duumbi:main/main/entry/print_free_ok", "duumbi:operand": {"@id": "duumbi:main/main/entry/free_ok"}},
    {"@type": "duumbi:HttpStatus", "@id": "duumbi:main/main/entry/status_after_free", "duumbi:operand": {"@id": "duumbi:main/main/entry/response"}, "duumbi:resultType": "result<i64,string>"},
    {"@type": "duumbi:ResultIsOk", "@id": "duumbi:main/main/entry/status_after_free_ok", "duumbi:operand": {"@id": "duumbi:main/main/entry/status_after_free"}},
    {"@type": "duumbi:Print", "@id": "duumbi:main/main/entry/print_status_after_free_ok", "duumbi:operand": {"@id": "duumbi:main/main/entry/status_after_free_ok"}},
    {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/return_zero", "duumbi:value": 0, "duumbi:resultType": "i64"},
    {"@type": "duumbi:Return", "@id": "duumbi:main/main/entry/return", "duumbi:operand": {"@id": "duumbi:main/main/entry/return_zero"}}
  ]}]}]
}"#;

#[test]
fn http_get_status_body_headers_and_release_loopback() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("loopback listener");
    let port = listener.local_addr().expect("local addr").port();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept HTTP client");
        let mut request = [0_u8; 512];
        let bytes = stream.read(&mut request).expect("read HTTP request");
        let request_text = String::from_utf8_lossy(&request[..bytes]);
        assert!(request_text.starts_with("GET /hello HTTP/1.1"));
        let body = "duumbi-ok";
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: text/plain\r\nX-Duumbi-Test: works\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        stream
            .write_all(response.as_bytes())
            .expect("write HTTP response");
    });

    let binary = compile_fixture(&http_get_fixture(port), "duumbi380_http_get");
    let output = Command::new(&binary)
        .output()
        .expect("compiled binary must run");

    server.join().expect("server must finish");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(lines, ["200", "duumbi-ok", "\"works\"", "true", "false"]);
    assert!(
        output.status.success(),
        "fixture exited unsuccessfully\nstdout:\n{}\nstderr:\n{}",
        stdout,
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = std::fs::remove_file(&binary);
}
