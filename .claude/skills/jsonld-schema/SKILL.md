# DUUMBI JSON-LD Schema Skill

ALWAYS use this skill when writing, editing, or validating `.jsonld` files
or `core.schema.json`.

---

## Every .jsonld file must start with

```json
{
  "@context": { "duumbi": "https://duumbi.dev/ns/core#" },
  "@type": "duumbi:Module",
  "@id": "duumbi:<module-name>",
  "duumbi:name": "<module-name>",
  "duumbi:functions": [...]
}
```

## nodeId format (strict)

`duumbi:<module>/<function>/<block>/<index>`

- module: the Module name (e.g. `main`)
- function: the Function name (e.g. `main`)
- block: the Block label (e.g. `entry`)
- index: zero-based integer within the block

Example: `duumbi:main/main/entry/0`

Indices must be sequential and unique within a block.
Every `{"@id": "..."}` reference must point to an existing nodeId.

## Phase 0 Op reference

**Const**
```json
{
  "@type": "duumbi:Const",
  "@id": "duumbi:main/main/entry/0",
  "duumbi:value": 3,
  "duumbi:resultType": "i64"
}
```

**Add / Sub / Mul / Div** (same structure)
```json
{
  "@type": "duumbi:Add",
  "@id": "duumbi:main/main/entry/2",
  "duumbi:left":  { "@id": "duumbi:main/main/entry/0" },
  "duumbi:right": { "@id": "duumbi:main/main/entry/1" },
  "duumbi:resultType": "i64"
}
```
Rule: left, right, and resultType must all be the same numeric type.

**Print**
```json
{
  "@type": "duumbi:Print",
  "@id": "duumbi:main/main/entry/3",
  "duumbi:operand": { "@id": "duumbi:main/main/entry/2" }
}
```

**Return**
```json
{
  "@type": "duumbi:Return",
  "@id": "duumbi:main/main/entry/4",
  "duumbi:operand": { "@id": "duumbi:main/main/entry/2" }
}
```

## Validation checklist (run mentally before saving)

- [ ] All `@id` values are unique within the file
- [ ] All `{"@id": "..."}` references point to existing nodes
- [ ] Ops within a block are ordered by index (0, 1, 2, ...)
- [ ] Every block ends with Return or Branch (no fall-through)
- [ ] A function named `main` exists at the top level
- [ ] resultType is present on every value-producing Op
- [ ] No cycles in data flow (a node's inputs must have lower indices)

## The canonical Phase 0 example (add.jsonld)

```json
{
  "@context": { "duumbi": "https://duumbi.dev/ns/core#" },
  "@type": "duumbi:Module",
  "@id": "duumbi:main",
  "duumbi:name": "main",
  "duumbi:functions": [
    {
      "@type": "duumbi:Function",
      "@id": "duumbi:main/main",
      "duumbi:name": "main",
      "duumbi:params": [],
      "duumbi:returnType": "i64",
      "duumbi:blocks": [
        {
          "@type": "duumbi:Block",
          "@id": "duumbi:main/main/entry",
          "duumbi:label": "entry",
          "duumbi:ops": [
            { "@type": "duumbi:Const",  "@id": "duumbi:main/main/entry/0", "duumbi:value": 3, "duumbi:resultType": "i64" },
            { "@type": "duumbi:Const",  "@id": "duumbi:main/main/entry/1", "duumbi:value": 5, "duumbi:resultType": "i64" },
            { "@type": "duumbi:Add",    "@id": "duumbi:main/main/entry/2", "duumbi:left": {"@id": "duumbi:main/main/entry/0"}, "duumbi:right": {"@id": "duumbi:main/main/entry/1"}, "duumbi:resultType": "i64" },
            { "@type": "duumbi:Print",  "@id": "duumbi:main/main/entry/3", "duumbi:operand": {"@id": "duumbi:main/main/entry/2"} },
            { "@type": "duumbi:Return", "@id": "duumbi:main/main/entry/4", "duumbi:operand": {"@id": "duumbi:main/main/entry/2"} }
          ]
        }
      ]
    }
  ]
}
```

Expected: binary prints `8`, exits with code 8.
