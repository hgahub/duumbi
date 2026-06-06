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

fn http_error_fixture(url: &str, output_name: &str) -> String {
    let mut ops = http_call_core_ops(output_name, "duumbi:HttpGet", url, "{}", None, 2000);
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
        2000,
    );
    append_return_zero(&mut ops);
    module_fixture(ops)
}

const LOCALHOST_CERT_PEM: &str = r#"-----BEGIN CERTIFICATE-----
MIIDJTCCAg2gAwIBAgIUP3HoO5IJ4KLq462gEZS/J+Lc8DkwDQYJKoZIhvcNAQEL
BQAwFDESMBAGA1UEAwwJbG9jYWxob3N0MB4XDTI2MDYwNjIyMjAxN1oXDTM2MDYw
MzIyMjAxN1owFDESMBAGA1UEAwwJbG9jYWxob3N0MIIBIjANBgkqhkiG9w0BAQEF
AAOCAQ8AMIIBCgKCAQEAp3yCNnwZYpwUVxgKVwXlTDtkEIUvx/kBRyihB8PoBPXG
2d3RNfEwobNk7ud0ai7dZ59KhJdkRj9wSSDcn4GaN2aPUrIoI1F1obma8IGCsdUn
UKnuwu0mxjGWbM6+8b1RG9Fr/sfZ09rO7N5Mt/AELz80PCB/Nmbr5oU9vtB8mVF6
/CYW8ujozejF5nijVAYcv9wAN9LofROs5j5rHsiokEnx7LoRjHvYkK/4LqKDAkjg
5cQ/9A5c14aQTGv9luTAMS3IwuPLoTBh/m1GATR2ekd8HsYD0X7W35CqRhZAXDvs
NiP4mrFZFIYk0Z2Vx5n5b8ntDODUO/HMvdn1nPQKYwIDAQABo28wbTAdBgNVHQ4E
FgQULQRYnKo7RpIvJe2kIbaG2DfPQyEwHwYDVR0jBBgwFoAULQRYnKo7RpIvJe2k
IbaG2DfPQyEwDwYDVR0TAQH/BAUwAwEB/zAaBgNVHREEEzARgglsb2NhbGhvc3SH
BH8AAAEwDQYJKoZIhvcNAQELBQADggEBABF0yfAwZQyoZ2pELyJA+EgdlEpxSNNG
1R4L2x9JEKWfxqlJbiVd9YV/OdFWAcTOztUupUPKG9Os1S4rvfbYBjoBRS8UNXdU
ccIV3cqQHRouaEsySsVV0QNWCo4tUK30u+G7mz4tt4mgP+FaKHBKdbo+ca6MTngr
W1sIwkJ8TK2kkUJyNXXveoVdCYe2idVh36gqAPzUXBgP6HuABHU2H4p9GqMYjoMJ
2QsMnGsoc+WMXdAhhHIKcQvOV8oCwDc5+Vs23rUrhRYoPTh0w4M2DX1d8S8xeQa6
cb2oQnrzplTrHBoHQY7CkBfTg0G5VYSaEaPHdNE0pkKVDnlPQ4WgXEg=
-----END CERTIFICATE-----"#;

const LOCALHOST_KEY_PEM: &str = concat!(
    "-----BEGIN ",
    "PRIVATE KEY-----\n",
    "MIIEvAIBADANBgkqhkiG9w0BAQEFAASCBKYwggSiAgEAAoIBAQCnfII2fBlinBRX\n",
    "GApXBeVMO2QQhS/H+QFHKKEHw+gE9cbZ3dE18TChs2Tu53RqLt1nn0qEl2RGP3BJ\n",
    "INyfgZo3Zo9SsigjUXWhuZrwgYKx1SdQqe7C7SbGMZZszr7xvVEb0Wv+x9nT2s7s\n",
    "3ky38AQvPzQ8IH82ZuvmhT2+0HyZUXr8Jhby6OjN6MXmeKNUBhy/3AA30uh9E6zm\n",
    "PmseyKiQSfHsuhGMe9iQr/guooMCSODlxD/0DlzXhpBMa/2W5MAxLcjC48uhMGH+\n",
    "bUYBNHZ6R3wexgPRftbfkKpGFkBcO+w2I/iasVkUhiTRnZXHmflvye0M4NQ78cy9\n",
    "2fWc9ApjAgMBAAECggEAB9bck2dImuR6UT9HUJ5uhpxrCRjqzR3bEPUWYHIrfHvy\n",
    "hEUNI0y4PYFTkpkTylqKM2zxxHX/lAgpHcsjeHXM/ZXX1IORPGH2Mw0ocuRk9STo\n",
    "c66YhdgqzfEJPOuKZW86iiZJu0GocPGXaN/Y0G00DPAU5lGREr9LgF0xMCq7AkQK\n",
    "LrapzOObD/z0vFmIdIsosIzQhXMdzQwSio+/1Ey2xrM3amwHwMyr6RNrrsPj2A55\n",
    "2hRKkLOLk/mfpzfdLmgH0C1j9TNxO70xvfd1gCJJnraypFhfuj8R+u9uEpN6Ntzg\n",
    "obgWX5KEUUSoJfiVVVv3pHyU/UynFCQk+H0srT7ogQKBgQDQUHAz+m3mx0vFuFVX\n",
    "29g8tKEY3AFbmm8JCM8ff8vcMNHboCrDJJgi9L4Aw9UoIfzB1RmuK1HjTOct/DWZ\n",
    "yrWHxQBf01HHR/yz4l2QVibNFAI5GhfH0SsDFmyYbuSrRqziy1BAZgW3j2VEDpm0\n",
    "MjTqIvCwY1qfPNMZKMSk5CFzgQKBgQDN031tgKVoSvY83l+mQ0JCtnBZ4SCahEPz\n",
    "DZspEPv1Gz6o9e/5lmFq7//BvWAaPoiiYLrfe10ae+A3461SipzHfWTa2FYZ8sQk\n",
    "r8LO3S+mD6irYDrkh6A2Q0kfkYu7XKe9dvOXhQFutqlVts+yL4Mah1+6RdMfG08u\n",
    "CtO0lU4f4wKBgAwgUpfEATfI7DFDTLyDkK/f9+zBidayQ7pr59q2jsBvmxfE2Bhp\n",
    "/e0zAAh9XeArMlJ6PDd2UBsCNAbqQpiEQ1L29dGeNIl8OEqkZ7vqN/ICMyrtyOqZ\n",
    "034nhQTOl8Mcpx3AphhJmBWaZFO04d+qeIgUppwt/G1+le9F/0R1/ziBAoGAOvCY\n",
    "F1Zih2YH81A+la7m95GkxKgqHPVJO/2mc/EQJZVCsUGUEaXVibjmRUWEkp9boxwO\n",
    "B1cdRys3/uksxdk5ogqvadfPeCjDsDnAkFpYfbY4N7MbyjtoToGgG/Ei0WlsA15f\n",
    "zQDicyDNhuUNvtnKMjuX1xCNr3ezidzB2RF0SL8CgYB5ERPRn8flEE+XCAV7IE2/\n",
    "gMENNnxAWvSj2OcK72MvJzYSgKXzYUcgOAWcoZCnvQg8fOVLoftJlPaN1wcEmzSx\n",
    "9DvpUjavjN5/aeY2FT8eAWovK/UeDXr/U7sJWoinWEpHckhVRpA6MN2bvrGHPHVB\n",
    "o+d+u+0IL7fa5LmY788KBA==\n",
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
    let fixture = http_error_fixture(
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
        elapsed < Duration::from_secs(2),
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
