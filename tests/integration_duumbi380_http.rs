//! DUUMBI-380 local HTTP runtime integration tests.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use duumbi::compiler::{linker, lowering};
use duumbi::errors::DiagnosticLevel;
use duumbi::graph::builder::build_graph;
use duumbi::graph::validator::validate;
use duumbi::parser::parse_jsonld;
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::{ServerConfig, ServerConnection, StreamOwned};
use serde_json::{Value, json};

const HTTPS_FIXTURE_TIMEOUT_MS: i64 = 10_000;

#[derive(Debug)]
struct CapturedRequest {
    request_line: String,
    headers: String,
    body: String,
}

fn node_ref(id: &str) -> Value {
    json!({ "@id": id })
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

fn id(name: &str) -> String {
    format!("duumbi:main/main/entry/{name}")
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

fn parse_headers_ops(prefix: &str, headers_json: &str) -> Vec<Value> {
    vec![
        const_string(&format!("{prefix}_header_text"), headers_json),
        json!({
            "@type": "duumbi:JsonParse",
            "@id": id(&format!("{prefix}_header_result")),
            "duumbi:operand": node_ref(&id(&format!("{prefix}_header_text"))),
            "duumbi:resultType": "result<json,string>"
        }),
        json!({
            "@type": "duumbi:ResultUnwrap",
            "@id": id(&format!("{prefix}_headers")),
            "duumbi:operand": node_ref(&id(&format!("{prefix}_header_result"))),
            "duumbi:resultType": "json"
        }),
    ]
}

fn print_response_ops(prefix: &str) -> Vec<Value> {
    vec![
        json!({
            "@type": "duumbi:ResultIsOk",
            "@id": id(&format!("{prefix}_ok")),
            "duumbi:operand": node_ref(&id(&format!("{prefix}_result")))
        }),
        json!({
            "@type": "duumbi:Print",
            "@id": id(&format!("{prefix}_print_ok")),
            "duumbi:operand": node_ref(&id(&format!("{prefix}_ok")))
        }),
        json!({
            "@type": "duumbi:ResultUnwrap",
            "@id": id(&format!("{prefix}_response")),
            "duumbi:operand": node_ref(&id(&format!("{prefix}_result"))),
            "duumbi:resultType": "http_response"
        }),
        json!({
            "@type": "duumbi:HttpStatus",
            "@id": id(&format!("{prefix}_status_result")),
            "duumbi:operand": node_ref(&id(&format!("{prefix}_response"))),
            "duumbi:resultType": "result<i64,string>"
        }),
        json!({
            "@type": "duumbi:ResultUnwrap",
            "@id": id(&format!("{prefix}_status")),
            "duumbi:operand": node_ref(&id(&format!("{prefix}_status_result"))),
            "duumbi:resultType": "i64"
        }),
        json!({
            "@type": "duumbi:Print",
            "@id": id(&format!("{prefix}_print_status")),
            "duumbi:operand": node_ref(&id(&format!("{prefix}_status")))
        }),
        json!({
            "@type": "duumbi:HttpBody",
            "@id": id(&format!("{prefix}_body_result")),
            "duumbi:operand": node_ref(&id(&format!("{prefix}_response"))),
            "duumbi:resultType": "result<string,string>"
        }),
        json!({
            "@type": "duumbi:ResultUnwrap",
            "@id": id(&format!("{prefix}_body")),
            "duumbi:operand": node_ref(&id(&format!("{prefix}_body_result"))),
            "duumbi:resultType": "string"
        }),
        json!({
            "@type": "duumbi:PrintString",
            "@id": id(&format!("{prefix}_print_body")),
            "duumbi:operand": node_ref(&id(&format!("{prefix}_body")))
        }),
    ]
}

fn print_header_field_ops(prefix: &str, key: &str) -> Vec<Value> {
    vec![
        json!({
            "@type": "duumbi:HttpHeaders",
            "@id": id(&format!("{prefix}_headers_result")),
            "duumbi:operand": node_ref(&id(&format!("{prefix}_response"))),
            "duumbi:resultType": "result<json,string>"
        }),
        json!({
            "@type": "duumbi:ResultUnwrap",
            "@id": id(&format!("{prefix}_headers_json")),
            "duumbi:operand": node_ref(&id(&format!("{prefix}_headers_result"))),
            "duumbi:resultType": "json"
        }),
        const_string(&format!("{prefix}_header_key"), key),
        json!({
            "@type": "duumbi:JsonGetField",
            "@id": id(&format!("{prefix}_header_value_result")),
            "duumbi:operand": node_ref(&id(&format!("{prefix}_headers_json"))),
            "duumbi:left": node_ref(&id(&format!("{prefix}_header_key"))),
            "duumbi:resultType": "result<json,string>"
        }),
        json!({
            "@type": "duumbi:ResultUnwrap",
            "@id": id(&format!("{prefix}_header_value")),
            "duumbi:operand": node_ref(&id(&format!("{prefix}_header_value_result"))),
            "duumbi:resultType": "json"
        }),
        json!({
            "@type": "duumbi:JsonStringify",
            "@id": id(&format!("{prefix}_header_string_result")),
            "duumbi:operand": node_ref(&id(&format!("{prefix}_header_value"))),
            "duumbi:resultType": "result<string,string>"
        }),
        json!({
            "@type": "duumbi:ResultUnwrap",
            "@id": id(&format!("{prefix}_header_string")),
            "duumbi:operand": node_ref(&id(&format!("{prefix}_header_string_result"))),
            "duumbi:resultType": "string"
        }),
        json!({
            "@type": "duumbi:PrintString",
            "@id": id(&format!("{prefix}_print_header")),
            "duumbi:operand": node_ref(&id(&format!("{prefix}_header_string")))
        }),
    ]
}

fn http_call_core_ops(
    prefix: &str,
    op_type: &str,
    url: &str,
    headers_json: &str,
    body: Option<&str>,
    timeout_ms: i64,
) -> Vec<Value> {
    let mut ops = vec![const_string(&format!("{prefix}_url"), url)];
    ops.extend(parse_headers_ops(prefix, headers_json));
    ops.push(const_i64(&format!("{prefix}_timeout"), timeout_ms));
    if let Some(body_text) = body {
        ops.push(const_string(&format!("{prefix}_request_body"), body_text));
    }

    let mut call = json!({
        "@type": op_type,
        "@id": id(&format!("{prefix}_result")),
        "duumbi:operand": node_ref(&id(&format!("{prefix}_url"))),
        "duumbi:left": node_ref(&id(&format!("{prefix}_headers"))),
        "duumbi:resultType": "result<http_response,string>"
    });
    if body.is_some() {
        call["duumbi:right"] = node_ref(&id(&format!("{prefix}_request_body")));
        call["duumbi:args"] = json!([node_ref(&id(&format!("{prefix}_timeout")))]);
    } else {
        call["duumbi:right"] = node_ref(&id(&format!("{prefix}_timeout")));
    }
    ops.push(call);
    ops
}

fn http_call_ops(
    prefix: &str,
    op_type: &str,
    url: &str,
    headers_json: &str,
    body: Option<&str>,
    timeout_ms: i64,
) -> Vec<Value> {
    let mut ops = http_call_core_ops(prefix, op_type, url, headers_json, body, timeout_ms);
    ops.extend(print_response_ops(prefix));
    ops
}

fn return_zero_op() -> Value {
    json!({
        "@type": "duumbi:Return",
        "@id": id("return"),
        "duumbi:operand": node_ref(&id("return_zero"))
    })
}

fn append_return_zero(ops: &mut Vec<Value>) {
    ops.push(const_i64("return_zero", 0));
    ops.push(return_zero_op());
}

fn try_read_http_request(stream: &mut impl Read) -> std::io::Result<CapturedRequest> {
    let mut data = Vec::new();
    let mut buffer = [0_u8; 1024];
    let mut header_end = None;
    while header_end.is_none() {
        let bytes = stream.read(&mut buffer)?;
        if bytes == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "client closed before sending headers",
            ));
        }
        data.extend_from_slice(&buffer[..bytes]);
        header_end = data.windows(4).position(|window| window == b"\r\n\r\n");
    }

    let header_end = header_end.expect("headers must have terminated") + 4;
    let header_text = String::from_utf8_lossy(&data[..header_end]).to_string();
    let content_len = header_text
        .lines()
        .find_map(|line| {
            let lower = line.to_ascii_lowercase();
            lower
                .strip_prefix("content-length:")
                .and_then(|_| line.split_once(':'))
                .and_then(|(_, value)| value.trim().parse::<usize>().ok())
        })
        .unwrap_or(0);
    while data.len() < header_end + content_len {
        let bytes = stream.read(&mut buffer)?;
        if bytes == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "client closed before sending full body",
            ));
        }
        data.extend_from_slice(&buffer[..bytes]);
    }

    let body = String::from_utf8_lossy(&data[header_end..header_end + content_len]).to_string();
    let request_line = header_text
        .lines()
        .next()
        .expect("request line")
        .to_string();
    Ok(CapturedRequest {
        request_line,
        headers: header_text,
        body,
    })
}

fn read_http_request(stream: &mut impl Read) -> CapturedRequest {
    try_read_http_request(stream).expect("read HTTP request")
}

fn tls_config() -> Arc<ServerConfig> {
    let cert = CertificateDer::from_pem_slice(LOCALHOST_CERT_PEM.as_bytes())
        .expect("embedded localhost certificate must parse");
    let key = PrivateKeyDer::from_pem_slice(LOCALHOST_KEY_PEM.as_bytes())
        .expect("embedded localhost key must parse");
    Arc::new(
        ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(vec![cert], key)
            .expect("embedded localhost TLS identity must be valid"),
    )
}

fn https_server(
    response: &'static str,
    expected_request_line: &'static str,
) -> (u16, thread::JoinHandle<CapturedRequest>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("loopback HTTPS listener");
    let port = listener.local_addr().expect("local HTTPS addr").port();
    let config = tls_config();
    let server = thread::spawn(move || {
        let (stream, _) = listener.accept().expect("accept HTTPS client");
        let connection = ServerConnection::new(config).expect("create TLS server connection");
        let mut stream = StreamOwned::new(connection, stream);
        let request = read_http_request(&mut stream);
        assert_eq!(request.request_line, expected_request_line);
        stream
            .write_all(response.as_bytes())
            .expect("write HTTPS response");
        request
    });
    (port, server)
}

fn https_server_allowing_rejected_client() -> (u16, thread::JoinHandle<bool>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("loopback HTTPS listener");
    let port = listener.local_addr().expect("local HTTPS addr").port();
    let config = tls_config();
    let server = thread::spawn(move || {
        let (stream, _) = listener.accept().expect("accept HTTPS client");
        let connection = ServerConnection::new(config).expect("create TLS server connection");
        let mut stream = StreamOwned::new(connection, stream);
        if try_read_http_request(&mut stream).is_ok() {
            let _ = stream.write_all(
                b"HTTP/1.1 200 OK\r\nContent-Length: 11\r\nConnection: close\r\n\r\nunexpected",
            );
            return true;
        }
        false
    });
    (port, server)
}

fn https_error_fixture(url: &str, output_name: &str) -> String {
    let ops = http_call_core_ops(
        output_name,
        "duumbi:HttpGet",
        url,
        "{}",
        None,
        HTTPS_FIXTURE_TIMEOUT_MS,
    );
    http_error_fixture_ops(output_name, ops)
}

fn http_error_fixture_ops(output_name: &str, mut ops: Vec<Value>) -> String {
    ops.push(json!({
        "@type": "duumbi:ResultIsOk",
        "@id": id(&format!("{output_name}_ok")),
        "duumbi:operand": node_ref(&id(&format!("{output_name}_result")))
    }));
    ops.push(json!({
        "@type": "duumbi:Print",
        "@id": id(&format!("print_{output_name}_ok")),
        "duumbi:operand": node_ref(&id(&format!("{output_name}_ok")))
    }));
    ops.push(json!({
        "@type": "duumbi:ResultUnwrapErr",
        "@id": id(&format!("{output_name}_err")),
        "duumbi:operand": node_ref(&id(&format!("{output_name}_result"))),
        "duumbi:resultType": "string"
    }));
    ops.push(json!({
        "@type": "duumbi:PrintString",
        "@id": id(&format!("print_{output_name}_err")),
        "duumbi:operand": node_ref(&id(&format!("{output_name}_err")))
    }));
    append_return_zero(&mut ops);
    module_fixture(ops)
}

fn https_get_fixture(port: u16) -> String {
    let mut ops = http_call_ops(
        "https",
        "duumbi:HttpGet",
        &format!("https://127.0.0.1:{port}/secure"),
        "{}",
        None,
        HTTPS_FIXTURE_TIMEOUT_MS,
    );
    append_return_zero(&mut ops);
    module_fixture(ops)
}

const LOCALHOST_CERT_PEM: &str = r#"-----BEGIN CERTIFICATE-----
MIIDTDCCAjSgAwIBAgIUYDdQZ5VhAc/aTXd5wNeBzMsfVIQwDQYJKoZIhvcNAQEL
BQAwFDESMBAGA1UEAwwJbG9jYWxob3N0MB4XDTI2MDYwNzA4MjExMFoXDTM2MDYw
NDA4MjExMFowFDESMBAGA1UEAwwJbG9jYWxob3N0MIIBIjANBgkqhkiG9w0BAQEF
AAOCAQ8AMIIBCgKCAQEAsiJgZKvgoHFC/PptktMkOGgxnTYw7mtiY4RAdIExexg6
euQq0kmrBcK3HgSgDm0glrNg2TsZFNfb8V/CkA4DEdyVrxO6SeTG2pUiJWPw2bbi
fJTRm14BCliu0bKRQ/9JAjdXqF5zpN8Th+QryRmxZQ3OQW6FaHeqbZeJmpMkix9g
fVfq9jEYZwkV45KDjP8pYxumQrUk7kI883G7jiuNHMMEmW1np/2eLTK4zt5O6mVT
qjpEzyUj4MAzwak83lGPt9uEzQCsqmiRIwTH4yjof/WaNgqNW2hxu7s9c/FFAx/w
KilFj3FLuYzLYtVADPR50ub9dDIyf/N9ji07NDc+iwIDAQABo4GVMIGSMB0GA1Ud
DgQWBBSeZ3Xn+Mz8do+LbTM7csn/s0gMCzAfBgNVHSMEGDAWgBSeZ3Xn+Mz8do+L
bTM7csn/s0gMCzAPBgNVHRMBAf8EBTADAQH/MA4GA1UdDwEB/wQEAwIBpjATBgNV
HSUEDDAKBggrBgEFBQcDATAaBgNVHREEEzARgglsb2NhbGhvc3SHBH8AAAEwDQYJ
KoZIhvcNAQELBQADggEBAJZYw5Fj/59Sgqxlt557v25gje8O/lH0A+psqZ10jDqu
7/EBFO6XjTp5kpo9mcn/X0+Zo7R5fl460j/L4Vk/esKkgJyQWtiV5kbPZY4iHl9A
nYQlMC5dhIFaoXGpqxNni9VeqLcwqu14LEhf3KKrFNBfOYohjIVKKw/PETEZIbWL
8jakVzpzPm3P35rKx+diCIc59F6y++shDDXapdpUBhfwgkQqSh6UoR9s6PhPuU/M
fPcJOIfIekChVy/x+UfWugBw7k3S9aUxDLKBYkkddLj8Z/h31cBH8LgoDInhdben
CgKT1ksR1qH9SJIooDKJxkFGxCU5VIfh5V+Lf3ypKi8=
-----END CERTIFICATE-----"#;

const LOCALHOST_KEY_PEM: &str = concat!(
    "-----BEGIN ",
    "PRIVATE KEY-----\n",
    "MIIEvAIBADANBgkqhkiG9w0BAQEFAASCBKYwggSiAgEAAoIBAQCyImBkq+CgcUL8\n",
    "+m2S0yQ4aDGdNjDua2JjhEB0gTF7GDp65CrSSasFwrceBKAObSCWs2DZOxkU19vx\n",
    "X8KQDgMR3JWvE7pJ5MbalSIlY/DZtuJ8lNGbXgEKWK7RspFD/0kCN1eoXnOk3xOH\n",
    "5CvJGbFlDc5BboVod6ptl4makySLH2B9V+r2MRhnCRXjkoOM/yljG6ZCtSTuQjzz\n",
    "cbuOK40cwwSZbWen/Z4tMrjO3k7qZVOqOkTPJSPgwDPBqTzeUY+324TNAKyqaJEj\n",
    "BMfjKOh/9Zo2Co1baHG7uz1z8UUDH/AqKUWPcUu5jMti1UAM9HnS5v10MjJ/832O\n",
    "LTs0Nz6LAgMBAAECggEAOWuuYdUrv9wyqUR6bLFGBC7GC3TL+hbAbO7VLhj1H8ZU\n",
    "F4gUK5wWlnFJQNJh27SepCVnay7LK3ZXjit9lp0FrUzLLVfxHV+zIAOhsabRHQUA\n",
    "ZgM8u9XmBPVISQ1EGUShvqajSYFEytkjvRK2cIkpLzdvjJT5SQ8F73TBJQQYbAWj\n",
    "Z67bk9BBTnSTb0Tq7QUtv+7JAcc6n7eMK7R3QuNIIOaNLXSQ4Vyoq/UGBX4hW9ax\n",
    "XkN8bocIisvMUR3ODXSKw7mJD2/ytgwjLTxtUo85/uPyF1D1eiDCFKyS4zeNYlSz\n",
    "1R9FZVrCsEPD5zR+68LqBhwqb6nKPzC6i0TZRi4ckQKBgQDrqGG9yaj044/H3Q35\n",
    "3wVhBsww29TjM5Oz4Klhdb1+tQesxzHlpE7PblXgeENF+tESsDKdxTAsbmFzec8G\n",
    "idfH5TuGtkQfQIOqT0yiB/ZQP/RTIt7WmvnYueNv2TgHlzlKpTk7x22DImQBohjl\n",
    "ABbQkwt7QExmN50iTRnlpXRLcQKBgQDBgtQMgfXGf7Zt+gHzhoe5Z9J2Kh6IWpoI\n",
    "dhzYcjS816h2UPQ+WqXJCVsm3oH9GlrwX7uBnkCFlmrOpEBH00ckgSVE/3nPdpJL\n",
    "1SLJO2b8iaKQKWmwKI223e3jNqKez0YnLbXrd2pqutE4kqbq0+/uhBX71W9FuVIA\n",
    "1EyROHbTuwKBgE84EHtrYifIo9ntHrij1zwRu+ykycEC4qEyYd5IZUZF9umHIOfw\n",
    "vymODsJhy0OoGEZvAuT0l8gn5wyZoxWwmuAw2Dzl4qqa1mgXNky13oCFr02PSFfe\n",
    "SyUnACTmYaZzmKfWORI2bUMK+ZFu+21oBUNiWxa4u7YU9fbE8nK3lwuRAoGAelkZ\n",
    "cP8KQgKleUtEyJAaaCM4cfWXcGa4VPk4q7EpnuxLWuM8SeBOSZlcxGqSjVCIhspA\n",
    "Z2eDK/M6fIRlEASJSo9M3R8aCQ3S2ZdccxbXunvbCILmi7ZYQ3J14d69WuN6W3MP\n",
    "Pl02L10Gw1oVpwtw+8EPlTYRMGhHbLbN4lNs7dkCgYAgDemgP5ZxyA/NieCs8P6q\n",
    "TqqMQ4n02Lr3cGqMMcA15bHCekT8WGpNJ6SM0/VEqKsx6ze5rbHsEuPNz5lYr9ll\n",
    "ZkBcsc1bHU2WgexD/d2GuCZzBS75xdnsAyAw8EQl9E+HEZXL88OPCM6HXhMsEoEq\n",
    "dBo+K8gP0lOPAAXxlJhIhA==\n",
    "-----END ",
    "PRIVATE KEY-----",
);

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

#[test]
fn https_get_succeeds_with_trusted_local_certificate() {
    let (port, server) = https_server(
        "HTTP/1.1 200 OK\r\nContent-Length: 9\r\nConnection: close\r\n\r\nsecure-ok",
        "GET /secure HTTP/1.1",
    );
    let ca_path = std::env::temp_dir().join("duumbi380_localhost_ca.pem");
    std::fs::write(&ca_path, LOCALHOST_CERT_PEM).expect("write localhost CA fixture");

    let binary = compile_fixture(&https_get_fixture(port), "duumbi380_https_get");
    let output = Command::new(&binary)
        .env("CURL_CA_BUNDLE", &ca_path)
        .env("SSL_CERT_FILE", &ca_path)
        .output()
        .expect("compiled binary must run");

    let captured = server.join().expect("server must finish");
    assert!(captured.headers.to_ascii_lowercase().contains("host:"));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(lines, ["true", "200", "secure-ok"]);
    assert!(
        output.status.success(),
        "fixture exited unsuccessfully\nstdout:\n{}\nstderr:\n{}",
        stdout,
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = std::fs::remove_file(&binary);
    let _ = std::fs::remove_file(ca_path);
}

#[test]
fn https_get_untrusted_local_certificate_returns_tls_error() {
    let (port, server) = https_server_allowing_rejected_client();
    let fixture = https_error_fixture(
        &format!("https://127.0.0.1:{port}/secure"),
        "https_untrusted",
    );
    let binary = compile_fixture(&fixture, "duumbi380_https_untrusted");
    let output = Command::new(&binary)
        .env_remove("CURL_CA_BUNDLE")
        .env_remove("SSL_CERT_FILE")
        .output()
        .expect("compiled binary must run");

    let client_sent_request = server.join().expect("server must finish");
    assert!(
        !client_sent_request,
        "untrusted HTTPS client should reject the certificate before sending HTTP"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(lines[0], "false");
    assert!(lines[1].contains("http_tls"), "{stdout}");
    assert!(
        output.status.success(),
        "fixture exited unsuccessfully\nstdout:\n{}\nstderr:\n{}",
        stdout,
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = std::fs::remove_file(&binary);
}

fn http_methods_fixture(port: u16) -> String {
    let base = format!("http://127.0.0.1:{port}");
    let mut ops = Vec::new();
    ops.extend(http_call_ops(
        "post",
        "duumbi:HttpPost",
        &format!("{base}/post"),
        r#"{"Content-Type":"application/json","X-Duumbi-Method":"post"}"#,
        Some(r#"{"name":"Ada"}"#),
        2000,
    ));
    ops.extend(http_call_ops(
        "put",
        "duumbi:HttpPut",
        &format!("{base}/put"),
        r#"{"Content-Type":"text/plain","X-Duumbi-Method":"put"}"#,
        Some("updated"),
        2000,
    ));
    ops.extend(http_call_ops(
        "delete",
        "duumbi:HttpDelete",
        &format!("{base}/delete"),
        r#"{"X-Duumbi-Method":"delete"}"#,
        None,
        2000,
    ));
    ops.extend(http_call_ops(
        "missing",
        "duumbi:HttpGet",
        &format!("{base}/missing"),
        "{}",
        None,
        2000,
    ));
    ops.extend(http_call_ops(
        "redirect",
        "duumbi:HttpGet",
        &format!("{base}/redirect"),
        "{}",
        None,
        2000,
    ));
    ops.extend(print_header_field_ops("redirect", "location"));
    append_return_zero(&mut ops);
    module_fixture(ops)
}

#[test]
fn http_methods_non_2xx_and_redirect_are_inspectable() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("loopback listener");
    let port = listener.local_addr().expect("local addr").port();
    let server = thread::spawn(move || {
        let responses = [
            (
                "HTTP/1.1 201 Created\r\nContent-Length: 7\r\nConnection: close\r\n\r\npost-ok",
                "POST /post HTTP/1.1",
            ),
            (
                "HTTP/1.1 202 Accepted\r\nContent-Length: 6\r\nConnection: close\r\n\r\nput-ok",
                "PUT /put HTTP/1.1",
            ),
            (
                "HTTP/1.1 200 OK\r\nContent-Length: 9\r\nConnection: close\r\n\r\ndelete-ok",
                "DELETE /delete HTTP/1.1",
            ),
            (
                "HTTP/1.1 404 Not Found\r\nContent-Length: 7\r\nConnection: close\r\n\r\nmissing",
                "GET /missing HTTP/1.1",
            ),
            (
                "HTTP/1.1 302 Found\r\nContent-Length: 8\r\nLocation: /final\r\nConnection: close\r\n\r\nredirect",
                "GET /redirect HTTP/1.1",
            ),
        ];
        let mut captured = Vec::new();
        for (response, expected_line) in responses {
            let (mut stream, _) = listener.accept().expect("accept HTTP client");
            let request = read_http_request(&mut stream);
            assert_eq!(request.request_line, expected_line);
            stream
                .write_all(response.as_bytes())
                .expect("write HTTP response");
            captured.push(request);
        }
        captured
    });

    let binary = compile_fixture(&http_methods_fixture(port), "duumbi380_http_methods");
    let output = Command::new(&binary)
        .output()
        .expect("compiled binary must run");

    let captured = server.join().expect("server must finish");
    assert_eq!(captured[0].body, r#"{"name":"Ada"}"#);
    assert!(
        captured[0]
            .headers
            .to_ascii_lowercase()
            .contains("content-type: application/json")
    );
    assert_eq!(captured[1].body, "updated");
    assert!(
        captured[1]
            .headers
            .to_ascii_lowercase()
            .contains("content-type: text/plain")
    );
    assert!(captured[2].body.is_empty());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(
        lines,
        [
            "true",
            "201",
            "post-ok",
            "true",
            "202",
            "put-ok",
            "true",
            "200",
            "delete-ok",
            "true",
            "404",
            "missing",
            "true",
            "302",
            "redirect",
            "\"/final\"",
        ]
    );
    assert!(
        output.status.success(),
        "fixture exited unsuccessfully\nstdout:\n{}\nstderr:\n{}",
        stdout,
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = std::fs::remove_file(&binary);
}

fn http_invalid_headers_fixture() -> String {
    let mut ops = vec![const_string("url", "http://127.0.0.1:9/invalid")];
    ops.extend(parse_headers_ops("bad", "[]"));
    ops.push(const_i64("timeout", 2000));
    ops.push(json!({
        "@type": "duumbi:HttpGet",
        "@id": id("bad_result"),
        "duumbi:operand": node_ref(&id("url")),
        "duumbi:left": node_ref(&id("bad_headers")),
        "duumbi:right": node_ref(&id("timeout")),
        "duumbi:resultType": "result<http_response,string>"
    }));
    ops.push(json!({
        "@type": "duumbi:ResultIsOk",
        "@id": id("bad_ok"),
        "duumbi:operand": node_ref(&id("bad_result"))
    }));
    ops.push(json!({
        "@type": "duumbi:Print",
        "@id": id("print_bad_ok"),
        "duumbi:operand": node_ref(&id("bad_ok"))
    }));
    ops.push(json!({
        "@type": "duumbi:ResultUnwrapErr",
        "@id": id("bad_err"),
        "duumbi:operand": node_ref(&id("bad_result")),
        "duumbi:resultType": "string"
    }));
    ops.push(json!({
        "@type": "duumbi:PrintString",
        "@id": id("print_bad_err"),
        "duumbi:operand": node_ref(&id("bad_err"))
    }));
    append_return_zero(&mut ops);
    module_fixture(ops)
}

#[test]
fn http_invalid_header_json_is_recoverable_error() {
    let binary = compile_fixture(
        &http_invalid_headers_fixture(),
        "duumbi380_http_bad_headers",
    );
    let output = Command::new(&binary)
        .output()
        .expect("compiled binary must run");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(lines[0], "false");
    assert!(lines[1].contains("http_headers"), "{stdout}");
    assert!(
        output.status.success(),
        "fixture exited unsuccessfully\nstdout:\n{}\nstderr:\n{}",
        stdout,
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = std::fs::remove_file(&binary);
}

fn http_timeout_fixture(port: u16) -> String {
    let mut ops = http_call_core_ops(
        "slow",
        "duumbi:HttpGet",
        &format!("http://127.0.0.1:{port}/slow"),
        "{}",
        None,
        100,
    );
    ops.push(json!({
        "@type": "duumbi:ResultIsOk",
        "@id": id("slow_ok"),
        "duumbi:operand": node_ref(&id("slow_result"))
    }));
    ops.push(json!({
        "@type": "duumbi:Print",
        "@id": id("print_slow_ok"),
        "duumbi:operand": node_ref(&id("slow_ok"))
    }));
    ops.push(json!({
        "@type": "duumbi:ResultUnwrapErr",
        "@id": id("slow_err"),
        "duumbi:operand": node_ref(&id("slow_result")),
        "duumbi:resultType": "string"
    }));
    ops.push(json!({
        "@type": "duumbi:PrintString",
        "@id": id("print_slow_err"),
        "duumbi:operand": node_ref(&id("slow_err"))
    }));
    append_return_zero(&mut ops);
    module_fixture(ops)
}

#[test]
fn http_timeout_returns_error_without_hanging() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("loopback listener");
    let port = listener.local_addr().expect("local addr").port();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept HTTP client");
        let request = read_http_request(&mut stream);
        assert_eq!(request.request_line, "GET /slow HTTP/1.1");
        thread::sleep(Duration::from_millis(500));
        let _ = stream.write_all(
            b"HTTP/1.1 200 OK\r\nContent-Length: 8\r\nConnection: close\r\n\r\ntoo-late",
        );
    });

    let binary = compile_fixture(&http_timeout_fixture(port), "duumbi380_http_timeout");
    let started = Instant::now();
    let output = Command::new(&binary)
        .output()
        .expect("compiled binary must run");
    let elapsed = started.elapsed();

    server.join().expect("server must finish");
    assert!(
        elapsed < Duration::from_secs(10),
        "timeout fixture took too long: {elapsed:?}"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(lines[0], "false");
    assert!(lines[1].contains("http_timeout"), "{stdout}");
    assert!(
        output.status.success(),
        "fixture exited unsuccessfully\nstdout:\n{}\nstderr:\n{}",
        stdout,
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = std::fs::remove_file(&binary);
}
