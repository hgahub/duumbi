//! DUUMBI-378 Cycle 2 acceptance coverage:
//! runtime-op lowering for stdin/stdout and workspace file APIs.

use duumbi::compiler::lowering;
use duumbi::graph::builder::build_graph;
use duumbi::parser::parse_jsonld;

#[test]
fn new_io_and_file_ops_lower_to_native_object() {
    let module = parse_jsonld(
        r#"{
          "@context": {"duumbi": "https://duumbi.dev/ns/core#"},
          "@type": "duumbi:Module",
          "@id": "duumbi:test",
          "duumbi:name": "test",
          "duumbi:functions": [{
            "@type": "duumbi:Function",
            "@id": "duumbi:test/main",
            "duumbi:name": "main",
            "duumbi:returnType": "result<array<string>,string>",
            "duumbi:blocks": [{
              "@type": "duumbi:Block",
              "@id": "duumbi:test/main/entry",
              "duumbi:label": "entry",
              "duumbi:ops": [
                {"@type": "duumbi:Const", "@id": "duumbi:test/main/entry/0",
                  "duumbi:value": "data", "duumbi:resultType": "string"},
                {"@type": "duumbi:Const", "@id": "duumbi:test/main/entry/1",
                  "duumbi:value": "notes.txt", "duumbi:resultType": "string"},
                {"@type": "duumbi:PathJoin", "@id": "duumbi:test/main/entry/2",
                  "duumbi:left": {"@id": "duumbi:test/main/entry/0"},
                  "duumbi:right": {"@id": "duumbi:test/main/entry/1"},
                  "duumbi:resultType": "result<string,string>"},
                {"@type": "duumbi:ResultUnwrap", "@id": "duumbi:test/main/entry/3",
                  "duumbi:operand": {"@id": "duumbi:test/main/entry/2"},
                  "duumbi:resultType": "string"},
                {"@type": "duumbi:ReadLine", "@id": "duumbi:test/main/entry/4",
                  "duumbi:resultType": "result<string,string>"},
                {"@type": "duumbi:ResultUnwrap", "@id": "duumbi:test/main/entry/5",
                  "duumbi:operand": {"@id": "duumbi:test/main/entry/4"},
                  "duumbi:resultType": "string"},
                {"@type": "duumbi:PrintLn", "@id": "duumbi:test/main/entry/6",
                  "duumbi:operand": {"@id": "duumbi:test/main/entry/5"},
                  "duumbi:resultType": "result<i64,string>"},
                {"@type": "duumbi:ResultUnwrap", "@id": "duumbi:test/main/entry/7",
                  "duumbi:operand": {"@id": "duumbi:test/main/entry/6"},
                  "duumbi:resultType": "i64"},
                {"@type": "duumbi:WriteFile", "@id": "duumbi:test/main/entry/8",
                  "duumbi:path": {"@id": "duumbi:test/main/entry/3"},
                  "duumbi:contents": {"@id": "duumbi:test/main/entry/5"},
                  "duumbi:resultType": "result<i64,string>"},
                {"@type": "duumbi:ResultUnwrap", "@id": "duumbi:test/main/entry/9",
                  "duumbi:operand": {"@id": "duumbi:test/main/entry/8"},
                  "duumbi:resultType": "i64"},
                {"@type": "duumbi:Const", "@id": "duumbi:test/main/entry/10",
                  "duumbi:value": 1024, "duumbi:resultType": "i64"},
                {"@type": "duumbi:ReadFile", "@id": "duumbi:test/main/entry/11",
                  "duumbi:path": {"@id": "duumbi:test/main/entry/3"},
                  "duumbi:maxBytes": {"@id": "duumbi:test/main/entry/10"},
                  "duumbi:resultType": "result<string,string>"},
                {"@type": "duumbi:ResultUnwrap", "@id": "duumbi:test/main/entry/12",
                  "duumbi:operand": {"@id": "duumbi:test/main/entry/11"},
                  "duumbi:resultType": "string"},
                {"@type": "duumbi:FileExists", "@id": "duumbi:test/main/entry/13",
                  "duumbi:path": {"@id": "duumbi:test/main/entry/3"},
                  "duumbi:resultType": "result<bool,string>"},
                {"@type": "duumbi:ResultUnwrap", "@id": "duumbi:test/main/entry/14",
                  "duumbi:operand": {"@id": "duumbi:test/main/entry/13"},
                  "duumbi:resultType": "bool"},
                {"@type": "duumbi:ListDir", "@id": "duumbi:test/main/entry/15",
                  "duumbi:path": {"@id": "duumbi:test/main/entry/0"},
                  "duumbi:resultType": "result<array<string>,string>"},
                {"@type": "duumbi:Return", "@id": "duumbi:test/main/entry/16",
                  "duumbi:operand": {"@id": "duumbi:test/main/entry/15"}}
              ]
            }]
          }]
        }"#,
    )
    .expect("fixture must parse");

    let graph = build_graph(&module).expect("fixture must build");
    let object = lowering::compile_to_object(&graph).expect("new runtime ops must lower");
    assert!(!object.is_empty(), "compiled object must not be empty");
}
