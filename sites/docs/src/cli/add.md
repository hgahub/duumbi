# duumbi add

AI-driven graph mutation. Translates a natural language request into a
validated JSON-LD graph patch, shows a diff, and asks for confirmation.

## Usage

```bash
duumbi add "<request>"
duumbi add --yes "<request>"   # Skip confirmation prompt
```

## Options

| Flag | Description |
|------|-------------|
| `--yes` / `-y` | Apply the patch without asking for confirmation |

## How it works

1. Reads `.duumbi/config.toml` → connects to LLM (Anthropic or OpenAI)
2. Sends the current graph + your request to the model with 6 structured tools
3. LLM responds with `PatchOp` calls (AddFunction, AddBlock, AddOp, ModifyOp, RemoveNode, SetEdge)
4. Patch is applied atomically (all-or-nothing)
5. Result is validated against the schema
6. On failure: retries once with the error message
7. Shows a diff summary and asks for confirmation
8. Saves a snapshot to `.duumbi/history/` and writes the new graph

## Example

```bash
duumbi add "add a function that computes the factorial of an i64"
```

```
+ Added function: factorial(n: i64) -> i64
  + Block: entry
    + Const 1 (base case)
    + Compare n <= 1
    + Branch → base_case / recursive_case
  + Block: base_case
    + Return 1
  + Block: recursive_case
    + Call factorial(n - 1)
    + Mul n * result
    + Return result

Apply? [y/N]
```

## Configuration

LLM provider and model are set in `.duumbi/config.toml`:

```toml
[llm]
provider = "anthropic"
model = "claude-sonnet-4-6"
api_key_env = "ANTHROPIC_API_KEY"
```
