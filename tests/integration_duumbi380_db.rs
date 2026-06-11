//! DUUMBI-380 local SQLite runtime integration tests.

use std::path::Path;
use std::process::Command;

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

fn print_result_ok(prefix: &str, result_name: &str) -> Vec<Value> {
    vec![
        json!({
            "@type": "duumbi:ResultIsOk",
            "@id": id(&format!("{prefix}_ok")),
            "duumbi:operand": node_ref(&id(result_name))
        }),
        json!({
            "@type": "duumbi:Print",
            "@id": id(&format!("{prefix}_print_ok")),
            "duumbi:operand": node_ref(&id(&format!("{prefix}_ok")))
        }),
    ]
}

fn print_result_err(prefix: &str, result_name: &str) -> Vec<Value> {
    let mut ops = print_result_ok(prefix, result_name);
    ops.extend([
        json!({
            "@type": "duumbi:ResultUnwrapErr",
            "@id": id(&format!("{prefix}_err")),
            "duumbi:operand": node_ref(&id(result_name)),
            "duumbi:resultType": "string"
        }),
        json!({
            "@type": "duumbi:PrintString",
            "@id": id(&format!("{prefix}_print_err")),
            "duumbi:operand": node_ref(&id(&format!("{prefix}_err")))
        }),
    ]);
    ops
}

fn db_open_ops(prefix: &str, path: &str) -> Vec<Value> {
    let result_name = format!("{prefix}_open_result");
    let conn_name = format!("{prefix}_conn");
    let mut ops = vec![
        const_string(&format!("{prefix}_path"), path),
        json!({
            "@type": "duumbi:DbOpen",
            "@id": id(&result_name),
            "duumbi:operand": node_ref(&id(&format!("{prefix}_path"))),
            "duumbi:resultType": "result<db_connection,string>"
        }),
    ];
    ops.extend(print_result_ok(&format!("{prefix}_open"), &result_name));
    ops.push(json!({
        "@type": "duumbi:ResultUnwrap",
        "@id": id(&conn_name),
        "duumbi:operand": node_ref(&id(&result_name)),
        "duumbi:resultType": "db_connection"
    }));
    ops
}

fn db_execute_ops(prefix: &str, conn_name: &str, sql: &str, params: &[&str]) -> Vec<Value> {
    let mut ops = params_ops(prefix, params);
    let result_name = format!("{prefix}_execute_result");
    ops.extend([
        const_string(&format!("{prefix}_sql"), sql),
        json!({
            "@type": "duumbi:DbExecute",
            "@id": id(&result_name),
            "duumbi:operand": node_ref(&id(conn_name)),
            "duumbi:left": node_ref(&id(&format!("{prefix}_sql"))),
            "duumbi:right": node_ref(&id(&format!("{prefix}_params"))),
            "duumbi:resultType": "result<i64,string>"
        }),
        json!({
            "@type": "duumbi:ResultUnwrap",
            "@id": id(&format!("{prefix}_changed")),
            "duumbi:operand": node_ref(&id(&result_name)),
            "duumbi:resultType": "i64"
        }),
        json!({
            "@type": "duumbi:Print",
            "@id": id(&format!("{prefix}_print_changed")),
            "duumbi:operand": node_ref(&id(&format!("{prefix}_changed")))
        }),
    ]);
    ops
}

fn db_query_ops(prefix: &str, conn_name: &str, sql: &str, params: &[&str]) -> Vec<Value> {
    let mut ops = params_ops(prefix, params);
    let result_name = format!("{prefix}_query_result");
    ops.extend([
        const_string(&format!("{prefix}_sql"), sql),
        json!({
            "@type": "duumbi:DbQuery",
            "@id": id(&result_name),
            "duumbi:operand": node_ref(&id(conn_name)),
            "duumbi:left": node_ref(&id(&format!("{prefix}_sql"))),
            "duumbi:right": node_ref(&id(&format!("{prefix}_params"))),
            "duumbi:resultType": "result<db_rows,string>"
        }),
        json!({
            "@type": "duumbi:ResultUnwrap",
            "@id": id(&format!("{prefix}_rows")),
            "duumbi:operand": node_ref(&id(&result_name)),
            "duumbi:resultType": "db_rows"
        }),
    ]);
    ops
}

fn print_rows_len_ops(prefix: &str, rows_name: &str) -> Vec<Value> {
    vec![
        json!({
            "@type": "duumbi:DbRowsLen",
            "@id": id(&format!("{prefix}_len_result")),
            "duumbi:operand": node_ref(&id(rows_name)),
            "duumbi:resultType": "result<i64,string>"
        }),
        json!({
            "@type": "duumbi:ResultUnwrap",
            "@id": id(&format!("{prefix}_len")),
            "duumbi:operand": node_ref(&id(&format!("{prefix}_len_result"))),
            "duumbi:resultType": "i64"
        }),
        json!({
            "@type": "duumbi:Print",
            "@id": id(&format!("{prefix}_print_len")),
            "duumbi:operand": node_ref(&id(&format!("{prefix}_len")))
        }),
    ]
}

fn db_row_get_result(prefix: &str, rows_name: &str, row_index: i64, column: &str) -> Vec<Value> {
    vec![
        const_i64(&format!("{prefix}_row_index"), row_index),
        const_string(&format!("{prefix}_column"), column),
        json!({
            "@type": "duumbi:DbRowGet",
            "@id": id(&format!("{prefix}_row_result")),
            "duumbi:operand": node_ref(&id(rows_name)),
            "duumbi:left": node_ref(&id(&format!("{prefix}_row_index"))),
            "duumbi:right": node_ref(&id(&format!("{prefix}_column"))),
            "duumbi:resultType": "result<string,string>"
        }),
    ]
}

fn print_row_value_ops(prefix: &str, rows_name: &str, row_index: i64, column: &str) -> Vec<Value> {
    let mut ops = db_row_get_result(prefix, rows_name, row_index, column);
    ops.extend([
        json!({
            "@type": "duumbi:ResultUnwrap",
            "@id": id(&format!("{prefix}_value")),
            "duumbi:operand": node_ref(&id(&format!("{prefix}_row_result"))),
            "duumbi:resultType": "string"
        }),
        json!({
            "@type": "duumbi:PrintString",
            "@id": id(&format!("{prefix}_print_value")),
            "duumbi:operand": node_ref(&id(&format!("{prefix}_value")))
        }),
    ]);
    ops
}

fn print_row_error_ops(prefix: &str, rows_name: &str, row_index: i64, column: &str) -> Vec<Value> {
    let mut ops = db_row_get_result(prefix, rows_name, row_index, column);
    ops.extend(print_result_err(prefix, &format!("{prefix}_row_result")));
    ops
}

fn close_conn_ops(prefix: &str, conn_name: &str) -> Vec<Value> {
    let result_name = format!("{prefix}_close_result");
    vec![
        json!({
            "@type": "duumbi:DbClose",
            "@id": id(&result_name),
            "duumbi:operand": node_ref(&id(conn_name)),
            "duumbi:resultType": "result<i64,string>"
        }),
        json!({
            "@type": "duumbi:ResultUnwrap",
            "@id": id(&format!("{prefix}_close_code")),
            "duumbi:operand": node_ref(&id(&result_name)),
            "duumbi:resultType": "i64"
        }),
        json!({
            "@type": "duumbi:Print",
            "@id": id(&format!("{prefix}_print_close_code")),
            "duumbi:operand": node_ref(&id(&format!("{prefix}_close_code")))
        }),
    ]
}

fn close_rows_ops(prefix: &str, rows_name: &str) -> Vec<Value> {
    let result_name = format!("{prefix}_rows_close_result");
    vec![
        json!({
            "@type": "duumbi:DbRowsFree",
            "@id": id(&result_name),
            "duumbi:operand": node_ref(&id(rows_name)),
            "duumbi:resultType": "result<i64,string>"
        }),
        json!({
            "@type": "duumbi:ResultUnwrap",
            "@id": id(&format!("{prefix}_rows_close_code")),
            "duumbi:operand": node_ref(&id(&result_name)),
            "duumbi:resultType": "i64"
        }),
        json!({
            "@type": "duumbi:Print",
            "@id": id(&format!("{prefix}_print_rows_close_code")),
            "duumbi:operand": node_ref(&id(&format!("{prefix}_rows_close_code")))
        }),
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
    let tmp_path = std::env::temp_dir().join("duumbi_380_db_tests");
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

fn memory_db_fixture() -> String {
    let mut ops = Vec::new();
    ops.extend(db_open_ops("mem", ":memory:"));
    ops.extend(db_execute_ops(
        "create",
        "mem_conn",
        "create table users(name text not null, note text)",
        &[],
    ));
    ops.extend(db_execute_ops(
        "insert_ada",
        "mem_conn",
        "insert into users(name, note) values (?, 'founder')",
        &["Ada"],
    ));
    ops.extend(db_execute_ops(
        "insert_injection",
        "mem_conn",
        "insert into users(name, note) values (?, null)",
        &["Ada'); DROP TABLE users; --"],
    ));
    ops.extend(db_query_ops(
        "select_ada",
        "mem_conn",
        "select name from users where name = ?",
        &["Ada"],
    ));
    ops.extend(print_rows_len_ops("select_ada", "select_ada_rows"));
    ops.extend(print_row_value_ops(
        "select_ada_name",
        "select_ada_rows",
        0,
        "name",
    ));
    ops.extend(db_query_ops(
        "select_injection",
        "mem_conn",
        "select name from users where name = ?",
        &["Ada'); DROP TABLE users; --"],
    ));
    ops.extend(print_rows_len_ops(
        "select_injection",
        "select_injection_rows",
    ));
    ops.extend(print_row_value_ops(
        "select_injection_name",
        "select_injection_rows",
        0,
        "name",
    ));
    ops.extend(db_query_ops(
        "empty",
        "mem_conn",
        "select name from users where name = ?",
        &["Grace"],
    ));
    ops.extend(print_rows_len_ops("empty", "empty_rows"));
    ops.extend(print_row_error_ops("empty_row", "empty_rows", 0, "name"));
    ops.extend(print_row_error_ops(
        "missing_column",
        "select_ada_rows",
        0,
        "missing",
    ));
    ops.extend(db_query_ops(
        "select_null",
        "mem_conn",
        "select note from users where name = ?",
        &["Ada'); DROP TABLE users; --"],
    ));
    ops.extend(print_row_error_ops(
        "null_note",
        "select_null_rows",
        0,
        "note",
    ));
    ops.extend(close_rows_ops("close_rows", "select_ada_rows"));
    ops.extend([json!({
        "@type": "duumbi:DbRowsLen",
        "@id": id("closed_rows_len_result"),
        "duumbi:operand": node_ref(&id("select_ada_rows")),
        "duumbi:resultType": "result<i64,string>"
    })]);
    ops.extend(print_result_err(
        "closed_rows_len",
        "closed_rows_len_result",
    ));
    ops.extend(close_conn_ops("mem", "mem_conn"));
    ops.extend(params_ops("after_close", &[]));
    ops.extend([
        const_string("after_close_sql", "select name from users"),
        json!({
            "@type": "duumbi:DbQuery",
            "@id": id("after_close_query_result"),
            "duumbi:operand": node_ref(&id("mem_conn")),
            "duumbi:left": node_ref(&id("after_close_sql")),
            "duumbi:right": node_ref(&id("after_close_params")),
            "duumbi:resultType": "result<db_rows,string>"
        }),
    ]);
    ops.extend(print_result_err(
        "after_close_query",
        "after_close_query_result",
    ));
    ops.extend(return_zero_ops());
    module_fixture(ops)
}

fn path_policy_fixture() -> String {
    let mut ops = Vec::new();
    ops.extend(db_open_ops("file", "data/demo.sqlite"));
    ops.extend(close_conn_ops("file", "file_conn"));
    ops.extend([
        const_string("outside_path", "../outside.sqlite"),
        json!({
            "@type": "duumbi:DbOpen",
            "@id": id("outside_open_result"),
            "duumbi:operand": node_ref(&id("outside_path")),
            "duumbi:resultType": "result<db_connection,string>"
        }),
    ]);
    ops.extend(print_result_err("outside_open", "outside_open_result"));
    ops.extend(return_zero_ops());
    module_fixture(ops)
}

#[test]
fn sqlite_memory_db_exec_query_params_and_resources_work() {
    let binary = compile_fixture(&memory_db_fixture(), "duumbi380_db_memory");
    let output = Command::new(&binary)
        .output()
        .expect("compiled binary must run");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().lines().collect();

    assert_eq!(lines[0], "true", "{stdout}");
    assert_eq!(lines[1], "0", "{stdout}");
    assert_eq!(lines[2], "1", "{stdout}");
    assert_eq!(lines[3], "1", "{stdout}");
    assert_eq!(lines[4], "1", "{stdout}");
    assert_eq!(lines[5], "Ada", "{stdout}");
    assert_eq!(lines[6], "1", "{stdout}");
    assert_eq!(lines[7], "Ada'); DROP TABLE users; --", "{stdout}");
    assert_eq!(lines[8], "0", "{stdout}");
    assert_eq!(lines[9], "false", "{stdout}");
    assert!(lines[10].contains("db_row"), "{stdout}");
    assert_eq!(lines[11], "false", "{stdout}");
    assert!(lines[12].contains("db_row"), "{stdout}");
    assert_eq!(lines[13], "false", "{stdout}");
    assert!(lines[14].contains("db_null"), "{stdout}");
    assert_eq!(lines[15], "0", "{stdout}");
    assert_eq!(lines[16], "false", "{stdout}");
    assert!(lines[17].contains("db_resource"), "{stdout}");
    assert_eq!(lines[18], "0", "{stdout}");
    assert_eq!(lines[19], "false", "{stdout}");
    assert!(lines[20].contains("db_resource"), "{stdout}");
    assert!(
        output.status.success(),
        "fixture exited unsuccessfully\nstdout:\n{}\nstderr:\n{}",
        stdout,
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = std::fs::remove_file(&binary);
}

#[test]
fn sqlite_file_paths_are_workspace_confined() {
    let binary = compile_fixture(&path_policy_fixture(), "duumbi380_db_path_policy");
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    std::fs::create_dir(workspace.path().join("data")).expect("data dir");
    let output = Command::new(&binary)
        .env("DUUMBI_WORKSPACE_ROOT", workspace.path())
        .output()
        .expect("compiled binary must run");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().lines().collect();

    assert_eq!(lines[0], "true", "{stdout}");
    assert_eq!(lines[1], "0", "{stdout}");
    assert_eq!(lines[2], "false", "{stdout}");
    assert!(lines[3].contains("db_path"), "{stdout}");
    assert!(
        output.status.success(),
        "fixture exited unsuccessfully\nstdout:\n{}\nstderr:\n{}",
        stdout,
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(workspace.path().join("data/demo.sqlite").exists());

    let _ = std::fs::remove_file(&binary);
}
