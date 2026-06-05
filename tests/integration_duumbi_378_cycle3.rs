//! DUUMBI-378 Cycle 3 deterministic E2E coverage:
//! public stdlib calls, stdin, workspace-confined file APIs, and path rejection.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use duumbi::workspace::{build_workspace, run_workspace_binary_with_stdin, workspace_output_path};

#[test]
fn stdlib_io_and_file_live_workspace_e2e() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    fs::create_dir_all(tmp.path().join(".duumbi/graph")).expect("graph dir");
    fs::create_dir_all(tmp.path().join(".duumbi/build")).expect("build dir");
    fs::write(
        tmp.path().join(".duumbi/config.toml"),
        r#"[workspace]
name = "duumbi-378-e2e"
namespace = "duumbi-378-e2e"
default-registry = "duumbi"

[registries]
duumbi = "https://registry.duumbi.dev"

[dependencies]
"@duumbi/stdlib-io" = "1.0.0"
"@duumbi/stdlib-file" = "1.0.0"
"#,
    )
    .expect("write config");

    let io_cache = tmp.path().join(".duumbi/cache/@duumbi/stdlib-io@1.0.0");
    fs::create_dir_all(io_cache.join("graph")).expect("create io cache");
    fs::write(
        io_cache.join("graph/io.jsonld"),
        include_str!("../stdlib/io.jsonld"),
    )
    .expect("write io module");
    fs::write(
        io_cache.join("manifest.toml"),
        r#"[module]
name = "@duumbi/stdlib-io"
version = "1.0.0"
description = "I/O utility functions (print wrappers, read_line, print_ln)"
license = "MPL-2.0"

[exports]
functions = ["print_i64", "print_f64", "print_bool", "print_string", "read_line", "print_ln"]
"#,
    )
    .expect("write io manifest");

    let file_cache = tmp.path().join(".duumbi/cache/@duumbi/stdlib-file@1.0.0");
    fs::create_dir_all(file_cache.join("graph")).expect("create file cache");
    fs::write(
        file_cache.join("graph/file.jsonld"),
        include_str!("../stdlib/file.jsonld"),
    )
    .expect("write file module");
    fs::write(
        file_cache.join("manifest.toml"),
        include_str!("../stdlib/file.manifest.toml"),
    )
    .expect("write file manifest");

    fs::create_dir_all(tmp.path().join("data")).expect("create data dir");
    fs::write(tmp.path().join(".duumbi/graph/main.jsonld"), main_jsonld()).expect("write main");

    let output_path = workspace_output_path(tmp.path());
    build_workspace(tmp.path(), &output_path, false).expect("workspace build");
    let run =
        run_workspace_binary_with_stdin(tmp.path(), &[], "hello duumbi\n").expect("workspace run");

    assert_eq!(run.exit_code, 0);
    assert_eq!(run.stderr, "");
    assert_eq!(run.stdout.replace("\r\n", "\n"), "hello duumbi\n1\nfalse\n");
    assert_eq!(
        fs::read_to_string(tmp.path().join("data/out.txt")).expect("read output"),
        "hello duumbi"
    );

    let config_after = fs::read_to_string(tmp.path().join(".duumbi/config.toml")).expect("config");
    assert!(
        !config_after.contains("@duumbi/stdlib-file-new"),
        "test must not introduce any non-spec file stdlib dependency"
    );
}

#[test]
fn read_line_empty_and_eof_inputs_are_deterministic() {
    let (tmp, output_path) = build_duumbi_378_workspace(read_line_echo_jsonld());

    let empty = run_workspace_binary_with_stdin(tmp.path(), &[], "\n").expect("empty stdin run");
    assert_eq!(empty.exit_code, 0);
    assert_eq!(empty.stderr, "");
    assert_eq!(normalize_stdout(&empty.stdout), "\n");

    let eof_after_bytes =
        run_workspace_binary_with_stdin(tmp.path(), &[], "hello").expect("eof stdin run");
    assert_eq!(eof_after_bytes.exit_code, 0);
    assert_eq!(eof_after_bytes.stderr, "");
    assert_eq!(normalize_stdout(&eof_after_bytes.stdout), "hello\n");

    assert!(
        output_path.exists(),
        "workspace build must produce a binary"
    );
}

#[test]
fn invalid_utf8_stdin_returns_error_class() {
    let (tmp, output_path) = build_duumbi_378_workspace(read_line_invalid_utf8_jsonld());
    let run = run_binary_with_stdin_bytes(&output_path, tmp.path(), Some(tmp.path()), &[0xff]);

    assert_eq!(run.exit_code, 0);
    assert_eq!(run.stderr, "");
    assert_eq!(normalize_stdout(&run.stdout), "true\n");
}

#[test]
fn file_api_error_classes_cover_path_byte_limit_utf8_and_join_policy() {
    let (tmp, output_path) = build_duumbi_378_workspace(file_error_classes_jsonld());
    fs::create_dir_all(tmp.path().join("data")).expect("data dir");
    fs::write(tmp.path().join("data/input.txt"), "hello").expect("write input");
    fs::write(tmp.path().join("data/bad.bin"), [0xff, b'\n']).expect("write invalid utf8");

    let run = run_workspace_binary_with_stdin(tmp.path(), &[], "").expect("file errors run");

    assert_eq!(run.exit_code, 0);
    assert_eq!(run.stderr, "");
    assert_eq!(
        normalize_stdout(&run.stdout),
        "true\ntrue\ntrue\ntrue\ntrue\ntrue\n"
    );
    assert!(
        output_path.exists(),
        "workspace build must produce a binary"
    );
}

#[test]
fn file_exists_overwrite_and_sorted_list_are_deterministic() {
    let (tmp, output_path) = build_duumbi_378_workspace(file_state_and_sorted_list_jsonld());
    fs::create_dir_all(tmp.path().join("data")).expect("data dir");
    fs::create_dir_all(tmp.path().join("listed")).expect("listed dir");
    fs::write(tmp.path().join("data/input.txt"), "hello").expect("write input");
    fs::write(tmp.path().join("data/out.txt"), "old").expect("write old output");
    fs::write(tmp.path().join("listed/b.txt"), "b").expect("write b");
    fs::write(tmp.path().join("listed/a.txt"), "a").expect("write a");

    let run = run_workspace_binary_with_stdin(tmp.path(), &[], "").expect("file state run");

    assert_eq!(run.exit_code, 0);
    assert_eq!(run.stderr, "");
    assert_eq!(
        normalize_stdout(&run.stdout),
        "true\nfalse\n2\na.txt\nb.txt\n"
    );
    assert_eq!(
        fs::read_to_string(tmp.path().join("data/out.txt")).expect("read overwritten output"),
        "new"
    );
    assert!(
        output_path.exists(),
        "workspace build must produce a binary"
    );
}

#[test]
fn file_apis_fail_without_workspace_root_env() {
    let (tmp, output_path) = build_duumbi_378_workspace(workspace_root_missing_jsonld());
    fs::create_dir_all(tmp.path().join("data")).expect("data dir");
    fs::write(tmp.path().join("data/input.txt"), "hello").expect("write input");

    let run = run_binary_with_stdin_bytes(&output_path, tmp.path(), None, &[]);

    assert_eq!(run.exit_code, 0);
    assert_eq!(run.stderr, "");
    assert_eq!(normalize_stdout(&run.stdout), "true\n");
}

struct BinaryRun {
    exit_code: i32,
    stdout: String,
    stderr: String,
}

fn build_duumbi_378_workspace(main_jsonld: &str) -> (tempfile::TempDir, PathBuf) {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    fs::create_dir_all(tmp.path().join(".duumbi/graph")).expect("graph dir");
    fs::create_dir_all(tmp.path().join(".duumbi/build")).expect("build dir");
    fs::write(
        tmp.path().join(".duumbi/config.toml"),
        r#"[workspace]
name = "duumbi-378-focused"
namespace = "duumbi-378-focused"
default-registry = "duumbi"

[registries]
duumbi = "https://registry.duumbi.dev"

[dependencies]
"@duumbi/stdlib-io" = "1.0.0"
"@duumbi/stdlib-file" = "1.0.0"
"#,
    )
    .expect("write config");

    let io_cache = tmp.path().join(".duumbi/cache/@duumbi/stdlib-io@1.0.0");
    fs::create_dir_all(io_cache.join("graph")).expect("create io cache");
    fs::write(
        io_cache.join("graph/io.jsonld"),
        include_str!("../stdlib/io.jsonld"),
    )
    .expect("write io module");
    fs::write(
        io_cache.join("manifest.toml"),
        r#"[module]
name = "@duumbi/stdlib-io"
version = "1.0.0"
description = "I/O utility functions (print wrappers, read_line, print_ln)"
license = "MPL-2.0"

[exports]
functions = ["print_i64", "print_f64", "print_bool", "print_string", "read_line", "print_ln"]
"#,
    )
    .expect("write io manifest");

    let file_cache = tmp.path().join(".duumbi/cache/@duumbi/stdlib-file@1.0.0");
    fs::create_dir_all(file_cache.join("graph")).expect("create file cache");
    fs::write(
        file_cache.join("graph/file.jsonld"),
        include_str!("../stdlib/file.jsonld"),
    )
    .expect("write file module");
    fs::write(
        file_cache.join("manifest.toml"),
        include_str!("../stdlib/file.manifest.toml"),
    )
    .expect("write file manifest");

    fs::write(tmp.path().join(".duumbi/graph/main.jsonld"), main_jsonld).expect("write main");
    let output_path = workspace_output_path(tmp.path());
    build_workspace(tmp.path(), &output_path, false).expect("workspace build");
    (tmp, output_path)
}

fn run_binary_with_stdin_bytes(
    output_path: &Path,
    current_dir: &Path,
    workspace_root: Option<&Path>,
    stdin: &[u8],
) -> BinaryRun {
    let mut command = Command::new(output_path);
    command
        .current_dir(current_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(root) = workspace_root {
        command.env("DUUMBI_WORKSPACE_ROOT", root);
    } else {
        command.env_remove("DUUMBI_WORKSPACE_ROOT");
    }

    let mut child = command.spawn().expect("spawn compiled binary");
    child
        .stdin
        .as_mut()
        .expect("stdin pipe")
        .write_all(stdin)
        .expect("write stdin");
    let output = child.wait_with_output().expect("wait for binary");

    BinaryRun {
        exit_code: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

fn normalize_stdout(stdout: &str) -> String {
    stdout.replace("\r\n", "\n")
}

fn main_jsonld() -> &'static str {
    r#"{
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
            {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/0",
              "duumbi:value": "data", "duumbi:resultType": "string"},
            {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/1",
              "duumbi:value": "out.txt", "duumbi:resultType": "string"},
            {"@type": "duumbi:Call", "@id": "duumbi:main/main/entry/2",
              "duumbi:function": "path_join", "duumbi:module": "file",
              "duumbi:args": [
                {"@id": "duumbi:main/main/entry/0"},
                {"@id": "duumbi:main/main/entry/1"}
              ],
              "duumbi:resultType": "result<string,string>"},
            {"@type": "duumbi:ResultUnwrap", "@id": "duumbi:main/main/entry/3",
              "duumbi:operand": {"@id": "duumbi:main/main/entry/2"},
              "duumbi:resultType": "string"},
            {"@type": "duumbi:Call", "@id": "duumbi:main/main/entry/4",
              "duumbi:function": "read_line", "duumbi:module": "io",
              "duumbi:args": [],
              "duumbi:resultType": "result<string,string>"},
            {"@type": "duumbi:ResultUnwrap", "@id": "duumbi:main/main/entry/5",
              "duumbi:operand": {"@id": "duumbi:main/main/entry/4"},
              "duumbi:resultType": "string"},
            {"@type": "duumbi:Call", "@id": "duumbi:main/main/entry/6",
              "duumbi:function": "write_file", "duumbi:module": "file",
              "duumbi:args": [
                {"@id": "duumbi:main/main/entry/3"},
                {"@id": "duumbi:main/main/entry/5"}
              ],
              "duumbi:resultType": "result<i64,string>"},
            {"@type": "duumbi:ResultUnwrap", "@id": "duumbi:main/main/entry/7",
              "duumbi:operand": {"@id": "duumbi:main/main/entry/6"},
              "duumbi:resultType": "i64"},
            {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/8",
              "duumbi:value": 128, "duumbi:resultType": "i64"},
            {"@type": "duumbi:Call", "@id": "duumbi:main/main/entry/9",
              "duumbi:function": "read_file", "duumbi:module": "file",
              "duumbi:args": [
                {"@id": "duumbi:main/main/entry/3"},
                {"@id": "duumbi:main/main/entry/8"}
              ],
              "duumbi:resultType": "result<string,string>"},
            {"@type": "duumbi:ResultUnwrap", "@id": "duumbi:main/main/entry/10",
              "duumbi:operand": {"@id": "duumbi:main/main/entry/9"},
              "duumbi:resultType": "string"},
            {"@type": "duumbi:Call", "@id": "duumbi:main/main/entry/11",
              "duumbi:function": "print_ln", "duumbi:module": "io",
              "duumbi:args": [{"@id": "duumbi:main/main/entry/10"}],
              "duumbi:resultType": "result<i64,string>"},
            {"@type": "duumbi:ResultUnwrap", "@id": "duumbi:main/main/entry/12",
              "duumbi:operand": {"@id": "duumbi:main/main/entry/11"},
              "duumbi:resultType": "i64"},
            {"@type": "duumbi:Call", "@id": "duumbi:main/main/entry/13",
              "duumbi:function": "list_dir", "duumbi:module": "file",
              "duumbi:args": [{"@id": "duumbi:main/main/entry/0"}],
              "duumbi:resultType": "result<array<string>,string>"},
            {"@type": "duumbi:ResultUnwrap", "@id": "duumbi:main/main/entry/14",
              "duumbi:operand": {"@id": "duumbi:main/main/entry/13"},
              "duumbi:resultType": "array<string>"},
            {"@type": "duumbi:ArrayLength", "@id": "duumbi:main/main/entry/15",
              "duumbi:array": {"@id": "duumbi:main/main/entry/14"},
              "duumbi:resultType": "i64"},
            {"@type": "duumbi:Print", "@id": "duumbi:main/main/entry/16",
              "duumbi:operand": {"@id": "duumbi:main/main/entry/15"}},
            {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/17",
              "duumbi:value": "../escape.txt", "duumbi:resultType": "string"},
            {"@type": "duumbi:Call", "@id": "duumbi:main/main/entry/18",
              "duumbi:function": "file_exists", "duumbi:module": "file",
              "duumbi:args": [{"@id": "duumbi:main/main/entry/17"}],
              "duumbi:resultType": "result<bool,string>"},
            {"@type": "duumbi:ResultIsOk", "@id": "duumbi:main/main/entry/19",
              "duumbi:operand": {"@id": "duumbi:main/main/entry/18"},
              "duumbi:resultType": "bool"},
            {"@type": "duumbi:Print", "@id": "duumbi:main/main/entry/20",
              "duumbi:operand": {"@id": "duumbi:main/main/entry/19"}},
            {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/21",
              "duumbi:value": 0, "duumbi:resultType": "i64"},
            {"@type": "duumbi:Return", "@id": "duumbi:main/main/entry/22",
              "duumbi:operand": {"@id": "duumbi:main/main/entry/21"}}
          ]
        }]
      }]
    }"#
}

fn read_line_echo_jsonld() -> &'static str {
    r#"{
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
            {"@type": "duumbi:Call", "@id": "duumbi:main/main/entry/0",
              "duumbi:function": "read_line", "duumbi:module": "io",
              "duumbi:args": [], "duumbi:resultType": "result<string,string>"},
            {"@type": "duumbi:ResultUnwrap", "@id": "duumbi:main/main/entry/1",
              "duumbi:operand": {"@id": "duumbi:main/main/entry/0"},
              "duumbi:resultType": "string"},
            {"@type": "duumbi:Call", "@id": "duumbi:main/main/entry/2",
              "duumbi:function": "print_ln", "duumbi:module": "io",
              "duumbi:args": [{"@id": "duumbi:main/main/entry/1"}],
              "duumbi:resultType": "result<i64,string>"},
            {"@type": "duumbi:ResultUnwrap", "@id": "duumbi:main/main/entry/3",
              "duumbi:operand": {"@id": "duumbi:main/main/entry/2"},
              "duumbi:resultType": "i64"},
            {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/4",
              "duumbi:value": 0, "duumbi:resultType": "i64"},
            {"@type": "duumbi:Return", "@id": "duumbi:main/main/entry/5",
              "duumbi:operand": {"@id": "duumbi:main/main/entry/4"}}
          ]
        }]
      }]
    }"#
}

fn read_line_invalid_utf8_jsonld() -> &'static str {
    r#"{
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
            {"@type": "duumbi:Call", "@id": "duumbi:main/main/entry/0",
              "duumbi:function": "read_line", "duumbi:module": "io",
              "duumbi:args": [], "duumbi:resultType": "result<string,string>"},
            {"@type": "duumbi:ResultUnwrapErr", "@id": "duumbi:main/main/entry/1",
              "duumbi:operand": {"@id": "duumbi:main/main/entry/0"},
              "duumbi:resultType": "string"},
            {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/2",
              "duumbi:value": "stdin_invalid_utf8", "duumbi:resultType": "string"},
            {"@type": "duumbi:StringContains", "@id": "duumbi:main/main/entry/3",
              "duumbi:left": {"@id": "duumbi:main/main/entry/1"},
              "duumbi:right": {"@id": "duumbi:main/main/entry/2"},
              "duumbi:resultType": "bool"},
            {"@type": "duumbi:Print", "@id": "duumbi:main/main/entry/4",
              "duumbi:operand": {"@id": "duumbi:main/main/entry/3"}},
            {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/5",
              "duumbi:value": 0, "duumbi:resultType": "i64"},
            {"@type": "duumbi:Return", "@id": "duumbi:main/main/entry/6",
              "duumbi:operand": {"@id": "duumbi:main/main/entry/5"}}
          ]
        }]
      }]
    }"#
}

fn file_error_classes_jsonld() -> &'static str {
    r#"{
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
            {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/0",
              "duumbi:value": "/tmp/duumbi.txt", "duumbi:resultType": "string"},
            {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/1",
              "duumbi:value": 16, "duumbi:resultType": "i64"},
            {"@type": "duumbi:Call", "@id": "duumbi:main/main/entry/2",
              "duumbi:function": "read_file", "duumbi:module": "file",
              "duumbi:args": [
                {"@id": "duumbi:main/main/entry/0"},
                {"@id": "duumbi:main/main/entry/1"}],
              "duumbi:resultType": "result<string,string>"},
            {"@type": "duumbi:ResultUnwrapErr", "@id": "duumbi:main/main/entry/3",
              "duumbi:operand": {"@id": "duumbi:main/main/entry/2"},
              "duumbi:resultType": "string"},
            {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/4",
              "duumbi:value": "path_policy", "duumbi:resultType": "string"},
            {"@type": "duumbi:StringContains", "@id": "duumbi:main/main/entry/5",
              "duumbi:left": {"@id": "duumbi:main/main/entry/3"},
              "duumbi:right": {"@id": "duumbi:main/main/entry/4"},
              "duumbi:resultType": "bool"},
            {"@type": "duumbi:Print", "@id": "duumbi:main/main/entry/6",
              "duumbi:operand": {"@id": "duumbi:main/main/entry/5"}},

            {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/7",
              "duumbi:value": "../escape.txt", "duumbi:resultType": "string"},
            {"@type": "duumbi:Call", "@id": "duumbi:main/main/entry/8",
              "duumbi:function": "file_exists", "duumbi:module": "file",
              "duumbi:args": [{"@id": "duumbi:main/main/entry/7"}],
              "duumbi:resultType": "result<bool,string>"},
            {"@type": "duumbi:ResultUnwrapErr", "@id": "duumbi:main/main/entry/9",
              "duumbi:operand": {"@id": "duumbi:main/main/entry/8"},
              "duumbi:resultType": "string"},
            {"@type": "duumbi:StringContains", "@id": "duumbi:main/main/entry/10",
              "duumbi:left": {"@id": "duumbi:main/main/entry/9"},
              "duumbi:right": {"@id": "duumbi:main/main/entry/4"},
              "duumbi:resultType": "bool"},
            {"@type": "duumbi:Print", "@id": "duumbi:main/main/entry/11",
              "duumbi:operand": {"@id": "duumbi:main/main/entry/10"}},

            {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/12",
              "duumbi:value": "data/input.txt", "duumbi:resultType": "string"},
            {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/13",
              "duumbi:value": 4, "duumbi:resultType": "i64"},
            {"@type": "duumbi:Call", "@id": "duumbi:main/main/entry/14",
              "duumbi:function": "read_file", "duumbi:module": "file",
              "duumbi:args": [
                {"@id": "duumbi:main/main/entry/12"},
                {"@id": "duumbi:main/main/entry/13"}],
              "duumbi:resultType": "result<string,string>"},
            {"@type": "duumbi:ResultUnwrapErr", "@id": "duumbi:main/main/entry/15",
              "duumbi:operand": {"@id": "duumbi:main/main/entry/14"},
              "duumbi:resultType": "string"},
            {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/16",
              "duumbi:value": "byte_limit", "duumbi:resultType": "string"},
            {"@type": "duumbi:StringContains", "@id": "duumbi:main/main/entry/17",
              "duumbi:left": {"@id": "duumbi:main/main/entry/15"},
              "duumbi:right": {"@id": "duumbi:main/main/entry/16"},
              "duumbi:resultType": "bool"},
            {"@type": "duumbi:Print", "@id": "duumbi:main/main/entry/18",
              "duumbi:operand": {"@id": "duumbi:main/main/entry/17"}},

            {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/19",
              "duumbi:value": "data/bad.bin", "duumbi:resultType": "string"},
            {"@type": "duumbi:Call", "@id": "duumbi:main/main/entry/20",
              "duumbi:function": "read_file", "duumbi:module": "file",
              "duumbi:args": [
                {"@id": "duumbi:main/main/entry/19"},
                {"@id": "duumbi:main/main/entry/1"}],
              "duumbi:resultType": "result<string,string>"},
            {"@type": "duumbi:ResultUnwrapErr", "@id": "duumbi:main/main/entry/21",
              "duumbi:operand": {"@id": "duumbi:main/main/entry/20"},
              "duumbi:resultType": "string"},
            {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/22",
              "duumbi:value": "invalid_utf8", "duumbi:resultType": "string"},
            {"@type": "duumbi:StringContains", "@id": "duumbi:main/main/entry/23",
              "duumbi:left": {"@id": "duumbi:main/main/entry/21"},
              "duumbi:right": {"@id": "duumbi:main/main/entry/22"},
              "duumbi:resultType": "bool"},
            {"@type": "duumbi:Print", "@id": "duumbi:main/main/entry/24",
              "duumbi:operand": {"@id": "duumbi:main/main/entry/23"}},

            {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/25",
              "duumbi:value": "", "duumbi:resultType": "string"},
            {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/26",
              "duumbi:value": "file.txt", "duumbi:resultType": "string"},
            {"@type": "duumbi:Call", "@id": "duumbi:main/main/entry/27",
              "duumbi:function": "path_join", "duumbi:module": "file",
              "duumbi:args": [
                {"@id": "duumbi:main/main/entry/25"},
                {"@id": "duumbi:main/main/entry/26"}],
              "duumbi:resultType": "result<string,string>"},
            {"@type": "duumbi:ResultUnwrapErr", "@id": "duumbi:main/main/entry/28",
              "duumbi:operand": {"@id": "duumbi:main/main/entry/27"},
              "duumbi:resultType": "string"},
            {"@type": "duumbi:StringContains", "@id": "duumbi:main/main/entry/29",
              "duumbi:left": {"@id": "duumbi:main/main/entry/28"},
              "duumbi:right": {"@id": "duumbi:main/main/entry/4"},
              "duumbi:resultType": "bool"},
            {"@type": "duumbi:Print", "@id": "duumbi:main/main/entry/30",
              "duumbi:operand": {"@id": "duumbi:main/main/entry/29"}},

            {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/31",
              "duumbi:value": "dir", "duumbi:resultType": "string"},
            {"@type": "duumbi:Call", "@id": "duumbi:main/main/entry/32",
              "duumbi:function": "path_join", "duumbi:module": "file",
              "duumbi:args": [
                {"@id": "duumbi:main/main/entry/31"},
                {"@id": "duumbi:main/main/entry/25"}],
              "duumbi:resultType": "result<string,string>"},
            {"@type": "duumbi:ResultUnwrapErr", "@id": "duumbi:main/main/entry/33",
              "duumbi:operand": {"@id": "duumbi:main/main/entry/32"},
              "duumbi:resultType": "string"},
            {"@type": "duumbi:StringContains", "@id": "duumbi:main/main/entry/34",
              "duumbi:left": {"@id": "duumbi:main/main/entry/33"},
              "duumbi:right": {"@id": "duumbi:main/main/entry/4"},
              "duumbi:resultType": "bool"},
            {"@type": "duumbi:Print", "@id": "duumbi:main/main/entry/35",
              "duumbi:operand": {"@id": "duumbi:main/main/entry/34"}},

            {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/36",
              "duumbi:value": 0, "duumbi:resultType": "i64"},
            {"@type": "duumbi:Return", "@id": "duumbi:main/main/entry/37",
              "duumbi:operand": {"@id": "duumbi:main/main/entry/36"}}
          ]
        }]
      }]
    }"#
}

fn file_state_and_sorted_list_jsonld() -> &'static str {
    r#"{
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
            {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/0",
              "duumbi:value": "data/out.txt", "duumbi:resultType": "string"},
            {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/1",
              "duumbi:value": "new", "duumbi:resultType": "string"},
            {"@type": "duumbi:Call", "@id": "duumbi:main/main/entry/2",
              "duumbi:function": "write_file", "duumbi:module": "file",
              "duumbi:args": [
                {"@id": "duumbi:main/main/entry/0"},
                {"@id": "duumbi:main/main/entry/1"}],
              "duumbi:resultType": "result<i64,string>"},
            {"@type": "duumbi:ResultUnwrap", "@id": "duumbi:main/main/entry/3",
              "duumbi:operand": {"@id": "duumbi:main/main/entry/2"},
              "duumbi:resultType": "i64"},

            {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/4",
              "duumbi:value": "data/input.txt", "duumbi:resultType": "string"},
            {"@type": "duumbi:Call", "@id": "duumbi:main/main/entry/5",
              "duumbi:function": "file_exists", "duumbi:module": "file",
              "duumbi:args": [{"@id": "duumbi:main/main/entry/4"}],
              "duumbi:resultType": "result<bool,string>"},
            {"@type": "duumbi:ResultUnwrap", "@id": "duumbi:main/main/entry/6",
              "duumbi:operand": {"@id": "duumbi:main/main/entry/5"},
              "duumbi:resultType": "bool"},
            {"@type": "duumbi:Print", "@id": "duumbi:main/main/entry/7",
              "duumbi:operand": {"@id": "duumbi:main/main/entry/6"}},

            {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/8",
              "duumbi:value": "data/missing.txt", "duumbi:resultType": "string"},
            {"@type": "duumbi:Call", "@id": "duumbi:main/main/entry/9",
              "duumbi:function": "file_exists", "duumbi:module": "file",
              "duumbi:args": [{"@id": "duumbi:main/main/entry/8"}],
              "duumbi:resultType": "result<bool,string>"},
            {"@type": "duumbi:ResultUnwrap", "@id": "duumbi:main/main/entry/10",
              "duumbi:operand": {"@id": "duumbi:main/main/entry/9"},
              "duumbi:resultType": "bool"},
            {"@type": "duumbi:Print", "@id": "duumbi:main/main/entry/11",
              "duumbi:operand": {"@id": "duumbi:main/main/entry/10"}},

            {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/12",
              "duumbi:value": "listed", "duumbi:resultType": "string"},
            {"@type": "duumbi:Call", "@id": "duumbi:main/main/entry/13",
              "duumbi:function": "list_dir", "duumbi:module": "file",
              "duumbi:args": [{"@id": "duumbi:main/main/entry/12"}],
              "duumbi:resultType": "result<array<string>,string>"},
            {"@type": "duumbi:ResultUnwrap", "@id": "duumbi:main/main/entry/14",
              "duumbi:operand": {"@id": "duumbi:main/main/entry/13"},
              "duumbi:resultType": "array<string>"},
            {"@type": "duumbi:ArrayLength", "@id": "duumbi:main/main/entry/15",
              "duumbi:array": {"@id": "duumbi:main/main/entry/14"},
              "duumbi:resultType": "i64"},
            {"@type": "duumbi:Print", "@id": "duumbi:main/main/entry/16",
              "duumbi:operand": {"@id": "duumbi:main/main/entry/15"}},

            {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/17",
              "duumbi:value": 0, "duumbi:resultType": "i64"},
            {"@type": "duumbi:ArrayGet", "@id": "duumbi:main/main/entry/18",
              "duumbi:array": {"@id": "duumbi:main/main/entry/14"},
              "duumbi:index": {"@id": "duumbi:main/main/entry/17"},
              "duumbi:resultType": "string"},
            {"@type": "duumbi:PrintString", "@id": "duumbi:main/main/entry/19",
              "duumbi:operand": {"@id": "duumbi:main/main/entry/18"}},

            {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/21",
              "duumbi:value": 1, "duumbi:resultType": "i64"},
            {"@type": "duumbi:ArrayGet", "@id": "duumbi:main/main/entry/22",
              "duumbi:array": {"@id": "duumbi:main/main/entry/14"},
              "duumbi:index": {"@id": "duumbi:main/main/entry/21"},
              "duumbi:resultType": "string"},
            {"@type": "duumbi:PrintString", "@id": "duumbi:main/main/entry/23",
              "duumbi:operand": {"@id": "duumbi:main/main/entry/22"}},

            {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/25",
              "duumbi:value": 0, "duumbi:resultType": "i64"},
            {"@type": "duumbi:Return", "@id": "duumbi:main/main/entry/26",
              "duumbi:operand": {"@id": "duumbi:main/main/entry/25"}}
          ]
        }]
      }]
    }"#
}

fn workspace_root_missing_jsonld() -> &'static str {
    r#"{
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
            {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/0",
              "duumbi:value": "data/input.txt", "duumbi:resultType": "string"},
            {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/1",
              "duumbi:value": 64, "duumbi:resultType": "i64"},
            {"@type": "duumbi:Call", "@id": "duumbi:main/main/entry/2",
              "duumbi:function": "read_file", "duumbi:module": "file",
              "duumbi:args": [
                {"@id": "duumbi:main/main/entry/0"},
                {"@id": "duumbi:main/main/entry/1"}],
              "duumbi:resultType": "result<string,string>"},
            {"@type": "duumbi:ResultUnwrapErr", "@id": "duumbi:main/main/entry/3",
              "duumbi:operand": {"@id": "duumbi:main/main/entry/2"},
              "duumbi:resultType": "string"},
            {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/4",
              "duumbi:value": "workspace_root_unavailable", "duumbi:resultType": "string"},
            {"@type": "duumbi:StringContains", "@id": "duumbi:main/main/entry/5",
              "duumbi:left": {"@id": "duumbi:main/main/entry/3"},
              "duumbi:right": {"@id": "duumbi:main/main/entry/4"},
              "duumbi:resultType": "bool"},
            {"@type": "duumbi:Print", "@id": "duumbi:main/main/entry/6",
              "duumbi:operand": {"@id": "duumbi:main/main/entry/5"}},
            {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/7",
              "duumbi:value": 0, "duumbi:resultType": "i64"},
            {"@type": "duumbi:Return", "@id": "duumbi:main/main/entry/8",
              "duumbi:operand": {"@id": "duumbi:main/main/entry/7"}}
          ]
        }]
      }]
    }"#
}
