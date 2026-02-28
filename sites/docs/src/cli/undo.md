# duumbi undo

Restore the previous graph snapshot. Snapshots are created automatically by
`duumbi add` before each mutation.

## Usage

```bash
duumbi undo
```

## How it works

- Reads the latest snapshot from `.duumbi/history/` (LIFO stack, `{N:06}.jsonld` naming)
- Copies it back to `.duumbi/graph/main.jsonld`
- Removes the snapshot file

Calling `duumbi undo` repeatedly walks back through the entire history.

## Example

```bash
duumbi add "add a buggy function"
# Something went wrong
duumbi undo
# Graph restored to state before the add
```
