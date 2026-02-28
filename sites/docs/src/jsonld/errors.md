# Error Codes

All validation and compilation errors use structured JSONL output to stdout.

## Format

```json
{
  "level": "error",
  "code": "E001",
  "message": "Type mismatch: Add expects matching operand types",
  "nodeId": "duumbi:main/main/entry/2",
  "file": "graph/main.jsonld",
  "details": { "expected": "i64", "found": "f64", "field": "duumbi:left" }
}
```

## Error code reference

| Code | Name | Description |
|------|------|-------------|
| E001 | Type mismatch | Operand types don't match the operation's requirements |
| E002 | Unknown Op | `@type` is not a recognized `duumbi:` operation |
| E003 | Missing field | A required field for this Op type is absent |
| E004 | Orphan reference | An `@id` reference points to a non-existent node |
| E005 | Duplicate `@id` | Two nodes share the same `@id` |
| E006 | No entry function | No `main` function found in the module |
| E007 | Cycle | Data-flow graph contains a cycle |
| E008 | Link failed | The linker (`cc`) returned a non-zero exit code |
| E009 | Schema invalid | The JSON-LD document fails JSON Schema validation |
| E010 | Unresolved cross-module reference | A `Call` op targets a function not exported by any module |
