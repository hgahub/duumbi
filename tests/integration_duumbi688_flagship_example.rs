//! DUUMBI-688 flagship HTTP + SQLite + JSON example smoke test.

use std::fs;
use std::io::{ErrorKind, Read as _, Write as _};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::Value;

use duumbi::workspace::{build_workspace, workspace_output_path};

const EXAMPLE_CONFIG: &str = include_str!("../examples/flagship-http-sqlite-json/config.toml");
const EXAMPLE_GRAPH: &str = include_str!("../examples/flagship-http-sqlite-json/graph/main.jsonld");

fn duumbi_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_duumbi"))
}

fn unused_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind unused loopback port");
    listener.local_addr().expect("local addr").port()
}

fn run_duumbi(workspace: &Path, args: &[&str]) -> Output {
    let child = Command::new(duumbi_binary())
        .args(args)
        .current_dir(workspace)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|error| panic!("duumbi {args:?} should start: {error}"));
    wait_with_timeout(child, Duration::from_secs(30))
}

fn assert_duumbi_success(workspace: &Path, args: &[&str]) {
    let output = run_duumbi(workspace, args);
    assert!(
        output.status.success(),
        "duumbi {args:?} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn patch_graph_port(port: u16) -> String {
    let mut graph: Value = serde_json::from_str(EXAMPLE_GRAPH).expect("example graph parses");
    let ops = graph["duumbi:functions"][0]["duumbi:blocks"][0]["duumbi:ops"]
        .as_array_mut()
        .expect("main ops array");
    let port_node = ops
        .iter_mut()
        .find(|op| op.get("@id").and_then(Value::as_str) == Some("duumbi:main/main/entry/port"))
        .expect("port node exists");
    port_node["duumbi:value"] = Value::from(i64::from(port));
    serde_json::to_string_pretty(&graph).expect("serialize patched graph")
}

fn materialize_workspace(workspace: &Path, port: u16) {
    fs::create_dir_all(workspace).expect("create workspace");
    assert_duumbi_success(workspace, &["init"]);

    fs::write(workspace.join(".duumbi/config.toml"), EXAMPLE_CONFIG).expect("write example config");
    fs::write(
        workspace.join(".duumbi/graph/main.jsonld"),
        patch_graph_port(port),
    )
    .expect("write patched example graph");

    assert_duumbi_success(workspace, &["deps", "vendor", "--all"]);
}

fn http_get_facts(port: u16) -> Result<String, String> {
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut last_error = None::<String>;

    while Instant::now() < deadline {
        match TcpStream::connect(("127.0.0.1", port)) {
            Ok(mut stream) => {
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .expect("set read timeout");
                stream
                    .write_all(
                        b"GET /facts HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
                    )
                    .expect("write request");

                let mut bytes = Vec::new();
                let mut buf = [0_u8; 1024];
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
                thread::sleep(Duration::from_millis(25));
            }
        }
    }

    Err(format!(
        "server did not return a loopback response: {last_error:?}"
    ))
}

fn response_body(response: &str) -> &str {
    response
        .split_once("\r\n\r\n")
        .map(|(_, body)| body)
        .expect("HTTP response has header/body separator")
}

fn wait_with_timeout(mut child: std::process::Child, timeout: Duration) -> Output {
    let deadline = Instant::now() + timeout;
    loop {
        if child.try_wait().expect("poll child").is_some() {
            return child.wait_with_output().expect("collect child output");
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            return child.wait_with_output().expect("collect killed output");
        }
        thread::sleep(Duration::from_millis(25));
    }
}

#[test]
fn flagship_example_builds_serves_sqlite_json_and_exits() {
    let tmp = tempfile::TempDir::new().expect("temp workspace");
    let workspace = tmp.path().join("workspace");
    let port = unused_port();
    materialize_workspace(&workspace, port);

    let output_path = workspace_output_path(&workspace);
    build_workspace(&workspace, &output_path, true).expect("example builds offline from vendor");

    let child = Command::new(&output_path)
        .current_dir(&workspace)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn flagship example");

    let response = match http_get_facts(port) {
        Ok(response) => response,
        Err(error) => {
            let run = wait_with_timeout(child, Duration::from_secs(1));
            panic!(
                "{error}\nstdout:\n{}\nstderr:\n{}",
                String::from_utf8_lossy(&run.stdout),
                String::from_utf8_lossy(&run.stderr)
            );
        }
    };

    assert!(
        response.starts_with("HTTP/1.1 200 OK"),
        "unexpected response status: {response:?}"
    );
    let body: Value = serde_json::from_str(response_body(&response)).expect("response body JSON");
    assert_eq!(body["service"], "flagship-http-sqlite-json");
    assert_eq!(body["route"], "/facts");
    assert_eq!(body["count"], 1);
    assert_eq!(body["first_fact"], "Ada Lovelace");
    assert_eq!(body["storage"], "sqlite-memory");

    let run = wait_with_timeout(child, Duration::from_secs(3));
    assert!(
        run.status.success(),
        "example process failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&run.stdout).trim(), "");
    assert_eq!(String::from_utf8_lossy(&run.stderr).trim(), "");

    let root_readme = include_str!("../README.md");
    assert!(root_readme.contains("examples/flagship-http-sqlite-json"));
    let examples_doc = include_str!("../docs/examples.md");
    assert!(examples_doc.contains("flagship-http-sqlite-json"));
}
