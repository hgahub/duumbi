//! DUUMBI-378 Cycle 1 acceptance coverage:
//! parser, validator, result-safety, and stdlib metadata.

use duumbi::errors::codes;
use duumbi::graph::builder::{build_graph, build_graph_no_call_check};
use duumbi::graph::validator::validate;
use duumbi::parser::parse_jsonld;
use duumbi::types::Op;

fn diagnostics_for(jsonld: &str) -> Vec<duumbi::errors::Diagnostic> {
    let module = parse_jsonld(jsonld).expect("fixture must parse");
    let graph = build_graph(&module).expect("fixture must build");
    validate(&graph)
}

#[test]
fn stdlib_io_metadata_includes_line_result_apis() {
    let module = parse_jsonld(include_str!("../stdlib/io.jsonld")).expect("stdlib io must parse");
    assert!(module.exports.contains(&"read_line".to_string()));
    assert!(module.exports.contains(&"print_ln".to_string()));

    let graph = build_graph_no_call_check(&module).expect("stdlib io must build");
    let diagnostics = validate(&graph);
    assert!(
        diagnostics.is_empty(),
        "stdlib io must validate cleanly, got {diagnostics:?}"
    );
}

#[test]
fn stdlib_file_metadata_validates() {
    let module =
        parse_jsonld(include_str!("../stdlib/file.jsonld")).expect("stdlib file must parse");
    assert_eq!(
        module.exports,
        vec![
            "read_file".to_string(),
            "write_file".to_string(),
            "file_exists".to_string(),
            "list_dir".to_string(),
            "path_join".to_string()
        ]
    );

    let graph = build_graph_no_call_check(&module).expect("stdlib file must build");
    let diagnostics = validate(&graph);
    assert!(
        diagnostics.is_empty(),
        "stdlib file must validate cleanly, got {diagnostics:?}"
    );
}

#[test]
fn read_file_parses_path_and_max_bytes_fields() {
    let module =
        parse_jsonld(include_str!("../stdlib/file.jsonld")).expect("stdlib file must parse");
    let read_file = module
        .functions
        .iter()
        .find(|function| function.name.0 == "read_file")
        .expect("read_file function must exist");
    let op = &read_file.blocks[0].ops[2];

    assert!(matches!(op.op, Op::ReadFile));
    assert_eq!(
        op.left.as_ref().expect("path ref").id.0,
        "duumbi:file/read_file/entry/0"
    );
    assert_eq!(
        op.right.as_ref().expect("maxBytes ref").id.0,
        "duumbi:file/read_file/entry/1"
    );
}

#[test]
fn read_file_rejects_non_i64_max_bytes() {
    let diagnostics = diagnostics_for(
        r#"{
          "@context": {"duumbi": "https://duumbi.dev/ns/core#"},
          "@type": "duumbi:Module",
          "@id": "duumbi:test",
          "duumbi:name": "test",
          "duumbi:functions": [{
            "@type": "duumbi:Function",
            "@id": "duumbi:test/main",
            "duumbi:name": "main",
            "duumbi:returnType": "result<string,string>",
            "duumbi:blocks": [{
              "@type": "duumbi:Block",
              "@id": "duumbi:test/main/entry",
              "duumbi:label": "entry",
              "duumbi:ops": [
                {"@type": "duumbi:Const", "@id": "duumbi:test/main/entry/0",
                  "duumbi:value": "notes.txt", "duumbi:resultType": "string"},
                {"@type": "duumbi:Const", "@id": "duumbi:test/main/entry/1",
                  "duumbi:value": "bad", "duumbi:resultType": "string"},
                {"@type": "duumbi:ReadFile", "@id": "duumbi:test/main/entry/2",
                  "duumbi:path": {"@id": "duumbi:test/main/entry/0"},
                  "duumbi:maxBytes": {"@id": "duumbi:test/main/entry/1"},
                  "duumbi:resultType": "result<string,string>"},
                {"@type": "duumbi:Return", "@id": "duumbi:test/main/entry/3",
                  "duumbi:operand": {"@id": "duumbi:test/main/entry/2"}}
              ]
            }]
          }]
        }"#,
    );

    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == codes::E001_TYPE_MISMATCH
                && diagnostic.message.contains("maxBytes")),
        "ReadFile maxBytes must reject non-i64 input, got {diagnostics:?}"
    );
}

#[test]
fn ignored_direct_result_producer_is_rejected() {
    let diagnostics = diagnostics_for(
        r#"{
          "@context": {"duumbi": "https://duumbi.dev/ns/core#"},
          "@type": "duumbi:Module",
          "@id": "duumbi:test",
          "duumbi:name": "test",
          "duumbi:functions": [{
            "@type": "duumbi:Function",
            "@id": "duumbi:test/main",
            "duumbi:name": "main",
            "duumbi:returnType": "i64",
            "duumbi:blocks": [{
              "@type": "duumbi:Block",
              "@id": "duumbi:test/main/entry",
              "duumbi:label": "entry",
              "duumbi:ops": [
                {"@type": "duumbi:ReadLine", "@id": "duumbi:test/main/entry/0",
                  "duumbi:resultType": "result<string,string>"},
                {"@type": "duumbi:Const", "@id": "duumbi:test/main/entry/1",
                  "duumbi:value": 0, "duumbi:resultType": "i64"},
                {"@type": "duumbi:Return", "@id": "duumbi:test/main/entry/2",
                  "duumbi:operand": {"@id": "duumbi:test/main/entry/1"}}
              ]
            }]
          }]
        }"#,
    );

    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == codes::E030_UNHANDLED_RESULT),
        "ignored ReadLine Result must be rejected, got {diagnostics:?}"
    );
}

#[test]
fn returned_direct_result_producer_is_accepted() {
    let diagnostics = diagnostics_for(
        r#"{
          "@context": {"duumbi": "https://duumbi.dev/ns/core#"},
          "@type": "duumbi:Module",
          "@id": "duumbi:test",
          "duumbi:name": "test",
          "duumbi:functions": [{
            "@type": "duumbi:Function",
            "@id": "duumbi:test/main",
            "duumbi:name": "main",
            "duumbi:returnType": "result<string,string>",
            "duumbi:blocks": [{
              "@type": "duumbi:Block",
              "@id": "duumbi:test/main/entry",
              "duumbi:label": "entry",
              "duumbi:ops": [
                {"@type": "duumbi:ReadLine", "@id": "duumbi:test/main/entry/0",
                  "duumbi:resultType": "result<string,string>"},
                {"@type": "duumbi:Return", "@id": "duumbi:test/main/entry/1",
                  "duumbi:operand": {"@id": "duumbi:test/main/entry/0"}}
              ]
            }]
          }]
        }"#,
    );

    assert!(
        diagnostics.is_empty(),
        "returned ReadLine Result should be accepted, got {diagnostics:?}"
    );
}
