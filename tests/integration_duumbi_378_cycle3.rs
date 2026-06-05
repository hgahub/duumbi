//! DUUMBI-378 Cycle 3 deterministic E2E coverage:
//! public stdlib calls, stdin, workspace-confined file APIs, and path rejection.

use std::fs;

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
    assert_eq!(run.stdout, "hello duumbi\n1\nfalse\n");
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
