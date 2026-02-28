# JSON-LD Format Overview

DUUMBI programs are stored as JSON-LD documents in `.duumbi/graph/`.

## Namespace

All DUUMBI terms use the namespace `https://duumbi.dev/ns/core#` with the
prefix `duumbi:`.

```json
{
  "@context": {
    "duumbi": "https://duumbi.dev/ns/core#"
  }
}
```

## nodeId format

Every node has a unique `@id`:

```
duumbi:<module>/<function>/<block>/<index>
```

Example: `duumbi:main/main/entry/2`

## Minimal program

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
      "duumbi:returnType": "void",
      "duumbi:params": [],
      "duumbi:blocks": [
        {
          "@type": "duumbi:Block",
          "@id": "duumbi:main/main/entry",
          "duumbi:label": "entry",
          "duumbi:ops": [
            {
              "@type": "duumbi:Const",
              "@id": "duumbi:main/main/entry/0",
              "duumbi:value": 42,
              "duumbi:resultType": "i64"
            },
            {
              "@type": "duumbi:Print",
              "@id": "duumbi:main/main/entry/1",
              "duumbi:operand": { "@id": "duumbi:main/main/entry/0" }
            },
            {
              "@type": "duumbi:Return",
              "@id": "duumbi:main/main/entry/2"
            }
          ]
        }
      ]
    }
  ]
}
```

See [Op Reference](ops.md) for all available operations.
