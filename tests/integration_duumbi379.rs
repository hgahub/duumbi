//! Integration coverage for DUUMBI-379 stdlib JSON and TCP runtime foundations.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

fn native_output_path(path: &std::path::Path) -> std::path::PathBuf {
    if path.exists() || std::env::consts::EXE_SUFFIX.is_empty() {
        return path.to_path_buf();
    }

    std::path::PathBuf::from(format!(
        "{}{}",
        path.display(),
        std::env::consts::EXE_SUFFIX
    ))
}

fn compile_fixture(fixture: &str, output_name: &str) -> std::path::PathBuf {
    let tmp_dir = std::env::temp_dir().join("duumbi_379_tests");
    std::fs::create_dir_all(&tmp_dir).expect("invariant: temp dir must be creatable");
    let output_binary = tmp_dir.join(output_name);

    let duumbi_output = Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--",
            "build",
            fixture,
            "-o",
            &output_binary.to_string_lossy(),
        ])
        .output()
        .expect("invariant: cargo run must be runnable");

    assert!(
        duumbi_output.status.success(),
        "duumbi build of {fixture} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&duumbi_output.stdout),
        String::from_utf8_lossy(&duumbi_output.stderr)
    );

    native_output_path(&output_binary)
}

#[test]
fn json_parse_field_array_and_stringify() {
    let binary = compile_fixture(
        "tests/fixtures/duumbi379_json_success.jsonld",
        "duumbi379_json_success",
    );

    let output = Command::new(&binary)
        .output()
        .expect("invariant: compiled binary must be runnable");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(lines[..3], ["\"duumbi\"", "3", "20"]);
    assert_eq!(lines[3].as_bytes(), b"\"caf\xc3\xa9\"");
    assert_eq!(lines[4].as_bytes(), b"\"a\\u0000b\"");
    assert!(
        output.status.success(),
        "fixture exited unsuccessfully: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = std::fs::remove_file(&binary);
}

fn write_dynamic_fixture(name: &str, content: &str) -> std::path::PathBuf {
    let tmp_dir = std::env::temp_dir().join("duumbi_379_tests");
    std::fs::create_dir_all(&tmp_dir).expect("invariant: temp dir must be creatable");
    let path = tmp_dir.join(name);
    std::fs::write(&path, content).expect("invariant: fixture must be writable");
    path
}

fn tcp_client_echo_fixture(port: u16) -> String {
    TCP_CLIENT_ECHO_FIXTURE.replace("__PORT__", &port.to_string())
}

fn tcp_listener_fixture(port: u16) -> String {
    TCP_LISTENER_FIXTURE.replace("__PORT__", &port.to_string())
}

fn tcp_refused_fixture(port: u16) -> String {
    TCP_REFUSED_FIXTURE.replace("__PORT__", &port.to_string())
}

fn tcp_failure_fixture(port: u16) -> String {
    TCP_FAILURE_FIXTURE.replace("__PORT__", &port.to_string())
}

fn tcp_invalid_utf8_fixture(port: u16) -> String {
    TCP_INVALID_UTF8_FIXTURE.replace("__PORT__", &port.to_string())
}

const TCP_CLIENT_ECHO_FIXTURE: &str = r#"{
  "@context": {"duumbi": "https://duumbi.dev/ns/core#"},
  "@type": "duumbi:Module",
  "@id": "duumbi:main",
  "duumbi:name": "main",
  "duumbi:functions": [{"@type": "duumbi:Function", "@id": "duumbi:main/main", "duumbi:name": "main", "duumbi:returnType": "i64", "duumbi:blocks": [{"@type": "duumbi:Block", "@id": "duumbi:main/main/entry", "duumbi:label": "entry", "duumbi:ops": [
    {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/0", "duumbi:value": "127.0.0.1", "duumbi:resultType": "string"},
    {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/1", "duumbi:value": __PORT__, "duumbi:resultType": "i64"},
    {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/2", "duumbi:value": 2000, "duumbi:resultType": "i64"},
    {"@type": "duumbi:TcpConnect", "@id": "duumbi:main/main/entry/3", "duumbi:operand": {"@id": "duumbi:main/main/entry/0"}, "duumbi:left": {"@id": "duumbi:main/main/entry/1"}, "duumbi:right": {"@id": "duumbi:main/main/entry/2"}, "duumbi:resultType": "result<tcp_socket,string>"},
    {"@type": "duumbi:ResultIsOk", "@id": "duumbi:main/main/entry/3_check", "duumbi:operand": {"@id": "duumbi:main/main/entry/3"}},
    {"@type": "duumbi:ResultUnwrap", "@id": "duumbi:main/main/entry/4", "duumbi:operand": {"@id": "duumbi:main/main/entry/3"}, "duumbi:resultType": "tcp_socket"},
    {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/5", "duumbi:value": "ping", "duumbi:resultType": "string"},
    {"@type": "duumbi:TcpWrite", "@id": "duumbi:main/main/entry/6", "duumbi:operand": {"@id": "duumbi:main/main/entry/4"}, "duumbi:left": {"@id": "duumbi:main/main/entry/5"}, "duumbi:right": {"@id": "duumbi:main/main/entry/2"}, "duumbi:resultType": "result<i64,string>"},
    {"@type": "duumbi:ResultIsOk", "@id": "duumbi:main/main/entry/6_check", "duumbi:operand": {"@id": "duumbi:main/main/entry/6"}},
    {"@type": "duumbi:ResultUnwrap", "@id": "duumbi:main/main/entry/7", "duumbi:operand": {"@id": "duumbi:main/main/entry/6"}, "duumbi:resultType": "i64"},
    {"@type": "duumbi:Print", "@id": "duumbi:main/main/entry/8", "duumbi:operand": {"@id": "duumbi:main/main/entry/7"}},
    {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/9", "duumbi:value": 4, "duumbi:resultType": "i64"},
    {"@type": "duumbi:TcpRead", "@id": "duumbi:main/main/entry/10", "duumbi:operand": {"@id": "duumbi:main/main/entry/4"}, "duumbi:left": {"@id": "duumbi:main/main/entry/9"}, "duumbi:right": {"@id": "duumbi:main/main/entry/2"}, "duumbi:resultType": "result<string,string>"},
    {"@type": "duumbi:ResultIsOk", "@id": "duumbi:main/main/entry/10_check", "duumbi:operand": {"@id": "duumbi:main/main/entry/10"}},
    {"@type": "duumbi:ResultUnwrap", "@id": "duumbi:main/main/entry/11", "duumbi:operand": {"@id": "duumbi:main/main/entry/10"}, "duumbi:resultType": "string"},
    {"@type": "duumbi:PrintString", "@id": "duumbi:main/main/entry/12", "duumbi:operand": {"@id": "duumbi:main/main/entry/11"}},
    {"@type": "duumbi:TcpClose", "@id": "duumbi:main/main/entry/13", "duumbi:operand": {"@id": "duumbi:main/main/entry/4"}, "duumbi:resultType": "result<i64,string>"},
    {"@type": "duumbi:ResultIsOk", "@id": "duumbi:main/main/entry/14", "duumbi:operand": {"@id": "duumbi:main/main/entry/13"}},
    {"@type": "duumbi:Print", "@id": "duumbi:main/main/entry/15", "duumbi:operand": {"@id": "duumbi:main/main/entry/14"}},
    {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/16", "duumbi:value": 0, "duumbi:resultType": "i64"},
    {"@type": "duumbi:Return", "@id": "duumbi:main/main/entry/17", "duumbi:operand": {"@id": "duumbi:main/main/entry/16"}}
  ]}]}]
}"#;

#[test]
fn tcp_connect_write_read_close_loopback() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("loopback listener");
    let port = listener.local_addr().expect("local addr").port();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept echo client");
        let mut buf = [0_u8; 4];
        stream.read_exact(&mut buf).expect("read ping");
        assert_eq!(&buf, b"ping");
        stream.write_all(&buf).expect("write echo");
    });

    let fixture = write_dynamic_fixture(
        "duumbi379_tcp_client.jsonld",
        &tcp_client_echo_fixture(port),
    );
    let binary = compile_fixture(&fixture.to_string_lossy(), "duumbi379_tcp_client");
    let output = Command::new(&binary).output().expect("binary runs");
    server.join().expect("server thread joins");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(lines, vec!["4", "ping", "true"]);
    assert!(
        output.status.success(),
        "tcp client fixture failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = std::fs::remove_file(&binary);
    let _ = std::fs::remove_file(&fixture);
}

const TCP_LISTENER_FIXTURE: &str = r#"{
  "@context": {"duumbi": "https://duumbi.dev/ns/core#"},
  "@type": "duumbi:Module",
  "@id": "duumbi:main",
  "duumbi:name": "main",
  "duumbi:functions": [{"@type": "duumbi:Function", "@id": "duumbi:main/main", "duumbi:name": "main", "duumbi:returnType": "i64", "duumbi:blocks": [{"@type": "duumbi:Block", "@id": "duumbi:main/main/entry", "duumbi:label": "entry", "duumbi:ops": [
    {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/0", "duumbi:value": "127.0.0.1", "duumbi:resultType": "string"},
    {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/1", "duumbi:value": __PORT__, "duumbi:resultType": "i64"},
    {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/2", "duumbi:value": 3000, "duumbi:resultType": "i64"},
    {"@type": "duumbi:TcpListen", "@id": "duumbi:main/main/entry/3", "duumbi:operand": {"@id": "duumbi:main/main/entry/0"}, "duumbi:left": {"@id": "duumbi:main/main/entry/1"}, "duumbi:right": {"@id": "duumbi:main/main/entry/2"}, "duumbi:resultType": "result<tcp_listener,string>"},
    {"@type": "duumbi:ResultIsOk", "@id": "duumbi:main/main/entry/3_check", "duumbi:operand": {"@id": "duumbi:main/main/entry/3"}},
    {"@type": "duumbi:ResultUnwrap", "@id": "duumbi:main/main/entry/4", "duumbi:operand": {"@id": "duumbi:main/main/entry/3"}, "duumbi:resultType": "tcp_listener"},
    {"@type": "duumbi:TcpAccept", "@id": "duumbi:main/main/entry/5", "duumbi:operand": {"@id": "duumbi:main/main/entry/4"}, "duumbi:left": {"@id": "duumbi:main/main/entry/2"}, "duumbi:resultType": "result<tcp_socket,string>"},
    {"@type": "duumbi:ResultIsOk", "@id": "duumbi:main/main/entry/5_check", "duumbi:operand": {"@id": "duumbi:main/main/entry/5"}},
    {"@type": "duumbi:ResultUnwrap", "@id": "duumbi:main/main/entry/6", "duumbi:operand": {"@id": "duumbi:main/main/entry/5"}, "duumbi:resultType": "tcp_socket"},
    {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/7", "duumbi:value": 4, "duumbi:resultType": "i64"},
    {"@type": "duumbi:TcpRead", "@id": "duumbi:main/main/entry/8", "duumbi:operand": {"@id": "duumbi:main/main/entry/6"}, "duumbi:left": {"@id": "duumbi:main/main/entry/7"}, "duumbi:right": {"@id": "duumbi:main/main/entry/2"}, "duumbi:resultType": "result<string,string>"},
    {"@type": "duumbi:ResultIsOk", "@id": "duumbi:main/main/entry/8_check", "duumbi:operand": {"@id": "duumbi:main/main/entry/8"}},
    {"@type": "duumbi:ResultUnwrap", "@id": "duumbi:main/main/entry/9", "duumbi:operand": {"@id": "duumbi:main/main/entry/8"}, "duumbi:resultType": "string"},
    {"@type": "duumbi:PrintString", "@id": "duumbi:main/main/entry/10", "duumbi:operand": {"@id": "duumbi:main/main/entry/9"}},
    {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/11", "duumbi:value": "pong", "duumbi:resultType": "string"},
    {"@type": "duumbi:TcpWrite", "@id": "duumbi:main/main/entry/12", "duumbi:operand": {"@id": "duumbi:main/main/entry/6"}, "duumbi:left": {"@id": "duumbi:main/main/entry/11"}, "duumbi:right": {"@id": "duumbi:main/main/entry/2"}, "duumbi:resultType": "result<i64,string>"},
    {"@type": "duumbi:ResultIsOk", "@id": "duumbi:main/main/entry/12_check", "duumbi:operand": {"@id": "duumbi:main/main/entry/12"}},
    {"@type": "duumbi:ResultUnwrap", "@id": "duumbi:main/main/entry/13", "duumbi:operand": {"@id": "duumbi:main/main/entry/12"}, "duumbi:resultType": "i64"},
    {"@type": "duumbi:Print", "@id": "duumbi:main/main/entry/14", "duumbi:operand": {"@id": "duumbi:main/main/entry/13"}},
    {"@type": "duumbi:TcpClose", "@id": "duumbi:main/main/entry/15", "duumbi:operand": {"@id": "duumbi:main/main/entry/6"}, "duumbi:resultType": "result<i64,string>"},
    {"@type": "duumbi:ResultIsOk", "@id": "duumbi:main/main/entry/16", "duumbi:operand": {"@id": "duumbi:main/main/entry/15"}},
    {"@type": "duumbi:Print", "@id": "duumbi:main/main/entry/17", "duumbi:operand": {"@id": "duumbi:main/main/entry/16"}},
    {"@type": "duumbi:TcpListenerClose", "@id": "duumbi:main/main/entry/18", "duumbi:operand": {"@id": "duumbi:main/main/entry/4"}, "duumbi:resultType": "result<i64,string>"},
    {"@type": "duumbi:ResultIsOk", "@id": "duumbi:main/main/entry/19", "duumbi:operand": {"@id": "duumbi:main/main/entry/18"}},
    {"@type": "duumbi:Print", "@id": "duumbi:main/main/entry/20", "duumbi:operand": {"@id": "duumbi:main/main/entry/19"}},
    {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/21", "duumbi:value": 0, "duumbi:resultType": "i64"},
    {"@type": "duumbi:Return", "@id": "duumbi:main/main/entry/22", "duumbi:operand": {"@id": "duumbi:main/main/entry/21"}}
  ]}]}]
}"#;

#[test]
fn tcp_listen_accept_read_write_loopback() {
    let probe = TcpListener::bind("127.0.0.1:0").expect("free loopback port");
    let port = probe.local_addr().expect("local addr").port();
    drop(probe);

    let fixture =
        write_dynamic_fixture("duumbi379_tcp_listener.jsonld", &tcp_listener_fixture(port));
    let binary = compile_fixture(&fixture.to_string_lossy(), "duumbi379_tcp_listener");
    let mut child = Command::new(&binary)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn listener fixture");

    let mut stream = {
        let mut connected = None;
        for _ in 0..100 {
            match TcpStream::connect(("127.0.0.1", port)) {
                Ok(stream) => {
                    connected = Some(stream);
                    break;
                }
                Err(_) => thread::sleep(Duration::from_millis(50)),
            }
        }
        match connected {
            Some(stream) => stream,
            None => {
                let _ = child.kill();
                let output = child.wait_with_output().expect("wait after failed connect");
                panic!(
                    "connect to DUUMBI listener failed\nstdout:\n{}\nstderr:\n{}",
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                );
            }
        }
    };
    stream.write_all(b"ping").expect("write ping");
    let mut response = [0_u8; 4];
    stream.read_exact(&mut response).expect("read pong");
    assert_eq!(&response, b"pong");

    let output = child.wait_with_output().expect("wait for listener fixture");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(lines, vec!["ping", "4", "true", "true"]);
    assert!(
        output.status.success(),
        "tcp listener fixture failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = std::fs::remove_file(&binary);
    let _ = std::fs::remove_file(&fixture);
}

const TCP_REFUSED_FIXTURE: &str = r#"{
  "@context": {"duumbi": "https://duumbi.dev/ns/core#"},
  "@type": "duumbi:Module",
  "@id": "duumbi:main",
  "duumbi:name": "main",
  "duumbi:functions": [{"@type": "duumbi:Function", "@id": "duumbi:main/main", "duumbi:name": "main", "duumbi:returnType": "i64", "duumbi:blocks": [{"@type": "duumbi:Block", "@id": "duumbi:main/main/entry", "duumbi:label": "entry", "duumbi:ops": [
    {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/0", "duumbi:value": "127.0.0.1", "duumbi:resultType": "string"},
    {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/1", "duumbi:value": __PORT__, "duumbi:resultType": "i64"},
    {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/2", "duumbi:value": 150, "duumbi:resultType": "i64"},
    {"@type": "duumbi:TcpConnect", "@id": "duumbi:main/main/entry/3", "duumbi:operand": {"@id": "duumbi:main/main/entry/0"}, "duumbi:left": {"@id": "duumbi:main/main/entry/1"}, "duumbi:right": {"@id": "duumbi:main/main/entry/2"}, "duumbi:resultType": "result<tcp_socket,string>"},
    {"@type": "duumbi:ResultIsOk", "@id": "duumbi:main/main/entry/4", "duumbi:operand": {"@id": "duumbi:main/main/entry/3"}},
    {"@type": "duumbi:Print", "@id": "duumbi:main/main/entry/5", "duumbi:operand": {"@id": "duumbi:main/main/entry/4"}},
    {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/6", "duumbi:value": 0, "duumbi:resultType": "i64"},
    {"@type": "duumbi:Return", "@id": "duumbi:main/main/entry/7", "duumbi:operand": {"@id": "duumbi:main/main/entry/6"}}
  ]}]}]
}"#;

#[test]
fn tcp_connect_refused_is_bounded_error() {
    let probe = TcpListener::bind("127.0.0.1:0").expect("free loopback port");
    let port = probe.local_addr().expect("local addr").port();
    drop(probe);

    let fixture = write_dynamic_fixture("duumbi379_tcp_refused.jsonld", &tcp_refused_fixture(port));
    let binary = compile_fixture(&fixture.to_string_lossy(), "duumbi379_tcp_refused");
    let started = Instant::now();
    let output = Command::new(&binary).output().expect("binary runs");
    let elapsed = started.elapsed();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(lines, vec!["false"]);
    assert!(
        elapsed < Duration::from_secs(5),
        "refused connect took {elapsed:?}"
    );
    assert!(
        output.status.success(),
        "tcp refused fixture failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = std::fs::remove_file(&binary);
    let _ = std::fs::remove_file(&fixture);
}

const TCP_FAILURE_FIXTURE: &str = r#"{
  "@context": {"duumbi": "https://duumbi.dev/ns/core#"},
  "@type": "duumbi:Module",
  "@id": "duumbi:main",
  "duumbi:name": "main",
  "duumbi:functions": [{"@type": "duumbi:Function", "@id": "duumbi:main/main", "duumbi:name": "main", "duumbi:returnType": "i64", "duumbi:blocks": [{"@type": "duumbi:Block", "@id": "duumbi:main/main/entry", "duumbi:label": "entry", "duumbi:ops": [
    {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/host", "duumbi:value": "127.0.0.1", "duumbi:resultType": "string"},
    {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/port", "duumbi:value": __PORT__, "duumbi:resultType": "i64"},
    {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/timeout", "duumbi:value": 500, "duumbi:resultType": "i64"},
    {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/short_timeout", "duumbi:value": 150, "duumbi:resultType": "i64"},
    {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/zero", "duumbi:value": 0, "duumbi:resultType": "i64"},
    {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/four", "duumbi:value": 4, "duumbi:resultType": "i64"},
    {"@type": "duumbi:TcpConnect", "@id": "duumbi:main/main/entry/bad_port", "duumbi:operand": {"@id": "duumbi:main/main/entry/host"}, "duumbi:left": {"@id": "duumbi:main/main/entry/zero"}, "duumbi:right": {"@id": "duumbi:main/main/entry/timeout"}, "duumbi:resultType": "result<tcp_socket,string>"},
    {"@type": "duumbi:ResultIsOk", "@id": "duumbi:main/main/entry/bad_port_ok", "duumbi:operand": {"@id": "duumbi:main/main/entry/bad_port"}},
    {"@type": "duumbi:Print", "@id": "duumbi:main/main/entry/bad_port_print", "duumbi:operand": {"@id": "duumbi:main/main/entry/bad_port_ok"}},
    {"@type": "duumbi:TcpConnect", "@id": "duumbi:main/main/entry/bad_timeout", "duumbi:operand": {"@id": "duumbi:main/main/entry/host"}, "duumbi:left": {"@id": "duumbi:main/main/entry/port"}, "duumbi:right": {"@id": "duumbi:main/main/entry/zero"}, "duumbi:resultType": "result<tcp_socket,string>"},
    {"@type": "duumbi:ResultIsOk", "@id": "duumbi:main/main/entry/bad_timeout_ok", "duumbi:operand": {"@id": "duumbi:main/main/entry/bad_timeout"}},
    {"@type": "duumbi:Print", "@id": "duumbi:main/main/entry/bad_timeout_print", "duumbi:operand": {"@id": "duumbi:main/main/entry/bad_timeout_ok"}},
    {"@type": "duumbi:TcpConnect", "@id": "duumbi:main/main/entry/connect", "duumbi:operand": {"@id": "duumbi:main/main/entry/host"}, "duumbi:left": {"@id": "duumbi:main/main/entry/port"}, "duumbi:right": {"@id": "duumbi:main/main/entry/timeout"}, "duumbi:resultType": "result<tcp_socket,string>"},
    {"@type": "duumbi:ResultIsOk", "@id": "duumbi:main/main/entry/connect_ok", "duumbi:operand": {"@id": "duumbi:main/main/entry/connect"}},
    {"@type": "duumbi:Print", "@id": "duumbi:main/main/entry/connect_print", "duumbi:operand": {"@id": "duumbi:main/main/entry/connect_ok"}},
    {"@type": "duumbi:ResultUnwrap", "@id": "duumbi:main/main/entry/socket", "duumbi:operand": {"@id": "duumbi:main/main/entry/connect"}, "duumbi:resultType": "tcp_socket"},
    {"@type": "duumbi:TcpRead", "@id": "duumbi:main/main/entry/bad_max", "duumbi:operand": {"@id": "duumbi:main/main/entry/socket"}, "duumbi:left": {"@id": "duumbi:main/main/entry/zero"}, "duumbi:right": {"@id": "duumbi:main/main/entry/timeout"}, "duumbi:resultType": "result<string,string>"},
    {"@type": "duumbi:ResultIsOk", "@id": "duumbi:main/main/entry/bad_max_ok", "duumbi:operand": {"@id": "duumbi:main/main/entry/bad_max"}},
    {"@type": "duumbi:Print", "@id": "duumbi:main/main/entry/bad_max_print", "duumbi:operand": {"@id": "duumbi:main/main/entry/bad_max_ok"}},
    {"@type": "duumbi:TcpRead", "@id": "duumbi:main/main/entry/read_timeout", "duumbi:operand": {"@id": "duumbi:main/main/entry/socket"}, "duumbi:left": {"@id": "duumbi:main/main/entry/four"}, "duumbi:right": {"@id": "duumbi:main/main/entry/short_timeout"}, "duumbi:resultType": "result<string,string>"},
    {"@type": "duumbi:ResultIsOk", "@id": "duumbi:main/main/entry/read_timeout_ok", "duumbi:operand": {"@id": "duumbi:main/main/entry/read_timeout"}},
    {"@type": "duumbi:Print", "@id": "duumbi:main/main/entry/read_timeout_print", "duumbi:operand": {"@id": "duumbi:main/main/entry/read_timeout_ok"}},
    {"@type": "duumbi:TcpClose", "@id": "duumbi:main/main/entry/close", "duumbi:operand": {"@id": "duumbi:main/main/entry/socket"}, "duumbi:resultType": "result<i64,string>"},
    {"@type": "duumbi:ResultIsOk", "@id": "duumbi:main/main/entry/close_ok", "duumbi:operand": {"@id": "duumbi:main/main/entry/close"}},
    {"@type": "duumbi:Print", "@id": "duumbi:main/main/entry/close_print", "duumbi:operand": {"@id": "duumbi:main/main/entry/close_ok"}},
    {"@type": "duumbi:TcpRead", "@id": "duumbi:main/main/entry/read_closed", "duumbi:operand": {"@id": "duumbi:main/main/entry/socket"}, "duumbi:left": {"@id": "duumbi:main/main/entry/four"}, "duumbi:right": {"@id": "duumbi:main/main/entry/timeout"}, "duumbi:resultType": "result<string,string>"},
    {"@type": "duumbi:ResultIsOk", "@id": "duumbi:main/main/entry/read_closed_ok", "duumbi:operand": {"@id": "duumbi:main/main/entry/read_closed"}},
    {"@type": "duumbi:Print", "@id": "duumbi:main/main/entry/read_closed_print", "duumbi:operand": {"@id": "duumbi:main/main/entry/read_closed_ok"}},
    {"@type": "duumbi:Return", "@id": "duumbi:main/main/entry/return", "duumbi:operand": {"@id": "duumbi:main/main/entry/zero"}}
  ]}]}]
}"#;

#[test]
fn tcp_invalid_args_timeout_and_closed_socket_are_errors() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("loopback listener");
    let port = listener.local_addr().expect("local addr").port();
    let server = thread::spawn(move || {
        let (_stream, _) = listener.accept().expect("accept idle client");
        thread::sleep(Duration::from_millis(500));
    });

    let fixture =
        write_dynamic_fixture("duumbi379_tcp_failures.jsonld", &tcp_failure_fixture(port));
    let binary = compile_fixture(&fixture.to_string_lossy(), "duumbi379_tcp_failures");
    let started = Instant::now();
    let output = Command::new(&binary).output().expect("binary runs");
    let elapsed = started.elapsed();
    server.join().expect("server thread joins");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(
        lines,
        vec!["false", "false", "true", "false", "false", "true", "false"]
    );
    assert!(
        elapsed < Duration::from_secs(5),
        "failure fixture took {elapsed:?}"
    );
    assert!(
        output.status.success(),
        "tcp failure fixture failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = std::fs::remove_file(&binary);
    let _ = std::fs::remove_file(&fixture);
}

const TCP_INVALID_UTF8_FIXTURE: &str = r#"{
  "@context": {"duumbi": "https://duumbi.dev/ns/core#"},
  "@type": "duumbi:Module",
  "@id": "duumbi:main",
  "duumbi:name": "main",
  "duumbi:functions": [{"@type": "duumbi:Function", "@id": "duumbi:main/main", "duumbi:name": "main", "duumbi:returnType": "i64", "duumbi:blocks": [{"@type": "duumbi:Block", "@id": "duumbi:main/main/entry", "duumbi:label": "entry", "duumbi:ops": [
    {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/0", "duumbi:value": "127.0.0.1", "duumbi:resultType": "string"},
    {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/1", "duumbi:value": __PORT__, "duumbi:resultType": "i64"},
    {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/2", "duumbi:value": 1000, "duumbi:resultType": "i64"},
    {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/3", "duumbi:value": 3, "duumbi:resultType": "i64"},
    {"@type": "duumbi:TcpConnect", "@id": "duumbi:main/main/entry/4", "duumbi:operand": {"@id": "duumbi:main/main/entry/0"}, "duumbi:left": {"@id": "duumbi:main/main/entry/1"}, "duumbi:right": {"@id": "duumbi:main/main/entry/2"}, "duumbi:resultType": "result<tcp_socket,string>"},
    {"@type": "duumbi:ResultIsOk", "@id": "duumbi:main/main/entry/4_check", "duumbi:operand": {"@id": "duumbi:main/main/entry/4"}},
    {"@type": "duumbi:ResultUnwrap", "@id": "duumbi:main/main/entry/5", "duumbi:operand": {"@id": "duumbi:main/main/entry/4"}, "duumbi:resultType": "tcp_socket"},
    {"@type": "duumbi:TcpRead", "@id": "duumbi:main/main/entry/6", "duumbi:operand": {"@id": "duumbi:main/main/entry/5"}, "duumbi:left": {"@id": "duumbi:main/main/entry/3"}, "duumbi:right": {"@id": "duumbi:main/main/entry/2"}, "duumbi:resultType": "result<string,string>"},
    {"@type": "duumbi:ResultIsOk", "@id": "duumbi:main/main/entry/7", "duumbi:operand": {"@id": "duumbi:main/main/entry/6"}},
    {"@type": "duumbi:Print", "@id": "duumbi:main/main/entry/8", "duumbi:operand": {"@id": "duumbi:main/main/entry/7"}},
    {"@type": "duumbi:TcpClose", "@id": "duumbi:main/main/entry/9", "duumbi:operand": {"@id": "duumbi:main/main/entry/5"}, "duumbi:resultType": "result<i64,string>"},
    {"@type": "duumbi:ResultIsOk", "@id": "duumbi:main/main/entry/10", "duumbi:operand": {"@id": "duumbi:main/main/entry/9"}},
    {"@type": "duumbi:Print", "@id": "duumbi:main/main/entry/11", "duumbi:operand": {"@id": "duumbi:main/main/entry/10"}},
    {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/12", "duumbi:value": 0, "duumbi:resultType": "i64"},
    {"@type": "duumbi:Return", "@id": "duumbi:main/main/entry/13", "duumbi:operand": {"@id": "duumbi:main/main/entry/12"}}
  ]}]}]
}"#;

#[test]
fn tcp_invalid_utf8_read_is_recoverable_error() {
    run_invalid_utf8_read_case(
        "duumbi379_tcp_invalid_utf8_byte",
        &[0xff],
        vec!["false", "true"],
    );
    run_invalid_utf8_read_case(
        "duumbi379_tcp_invalid_utf8_surrogate",
        &[0xed, 0xa0, 0x80],
        vec!["false", "true"],
    );
}

fn run_invalid_utf8_read_case(name: &str, payload: &'static [u8], expected: Vec<&str>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("loopback listener");
    let port = listener.local_addr().expect("local addr").port();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept client");
        stream
            .write_all(payload)
            .expect("write invalid utf-8 bytes");
    });

    let fixture = write_dynamic_fixture(&format!("{name}.jsonld"), &tcp_invalid_utf8_fixture(port));
    let binary = compile_fixture(&fixture.to_string_lossy(), name);
    let output = Command::new(&binary).output().expect("binary runs");
    server.join().expect("server thread joins");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(lines, expected);
    assert!(
        output.status.success(),
        "tcp invalid utf8 fixture failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = std::fs::remove_file(&binary);
    let _ = std::fs::remove_file(&fixture);
}

#[test]
fn json_errors_are_recoverable_results() {
    let binary = compile_fixture(
        "tests/fixtures/duumbi379_json_errors.jsonld",
        "duumbi379_json_errors",
    );

    let output = Command::new(&binary)
        .output()
        .expect("invariant: compiled binary must be runnable");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(lines, vec!["false", "false", "false", "false", "false"]);
    assert!(
        output.status.success(),
        "fixture exited unsuccessfully: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = std::fs::remove_file(&binary);
}
