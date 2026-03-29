# Phase 15: Studio Workflow — End-to-End Test Protocol

**Version:** 2.0
**Date:** 2026-03-29
**Milestone:** Phase 15 — Studio Workflow Redesign
**Issues:** #478–#489 (12 issues)

---

## Prerequisites

- [ ] Rust toolchain: `rustup show` → stable
- [ ] C compiler on PATH: `cc --version` (needed for linking compiled binaries)
- [ ] LLM provider API key in environment: `ANTHROPIC_API_KEY` (or `OPENAI_API_KEY`, etc.)

---

## Build Setup

Build both the `duumbi` CLI and the `studio` SSR server from the workspace root:

```bash
# 1. Build the main CLI binary
cargo build

# 2. Build the Studio SSR server (separate binary, requires "ssr" feature)
cargo build -p duumbi-studio --features ssr

# 3. Verify both binaries exist
ls -la target/debug/duumbi target/debug/studio

# 4. Export the CLI path for convenience
export DUUMBI="$(pwd)/target/debug/duumbi"

# 5. Run the test suite to confirm a clean baseline
cargo test --all
# Expected: 1722+ tests green, 0 failed

# 6. Clippy clean
cargo clippy --all-targets -- -D warnings
# Expected: 0 warnings
```

## Create a Test Workspace

```bash
# Create an isolated test directory
mkdir -p /tmp/duumbi-p15-test
cd /tmp/duumbi-p15-test

# Initialize a fresh duumbi workspace
$DUUMBI init .
# Expected: ".duumbi/ workspace created with main module"

# Configure an LLM provider (if not already in config)
cat >> .duumbi/config.toml << 'TOML'

[[providers]]
provider = "Anthropic"
role = "Primary"
model = "claude-sonnet-4-6"
api_key_env = "ANTHROPIC_API_KEY"
TOML

# Verify the workspace is ready
$DUUMBI check
# Expected: no errors (empty project validates fine)
```

> **Note:** Each sample task below assumes a **fresh workspace**. Either
> re-run `$DUUMBI init .` in a new directory per sample, or clean up
> between samples with `rm -rf .duumbi && $DUUMBI init .`

---

## Sample 1: Calculator (Simple)

**Intent:** "Build a calculator with add, subtract, multiply, divide functions that work on i64 numbers"
**Expected:** 1 module (calculator/ops), 4 functions, main modified, binary prints results
**Time target:** <10 minutes (CLI), <12 minutes (Studio)

### CLI REPL Walkthrough

```bash
# 1. Create fresh workspace
mkdir -p /tmp/duumbi-p15-calculator && cd /tmp/duumbi-p15-calculator
$DUUMBI init .
# Expected: ".duumbi/ workspace created with main module"

# 2. Launch REPL (no arguments = interactive mode)
$DUUMBI

# 3. In REPL — create intent
/intent create "Build a calculator with add, subtract, multiply, divide functions that work on i64 numbers"
# Expected output:
#   Intent: "Build a calculator..."
#   Acceptance criteria:
#     - add(a, b) returns a + b
#     - sub(a, b) returns a - b
#     - mul(a, b) returns a * b
#     - div(a, b) returns a / b (0 on division by zero)
#   Modules: create [calculator/ops], modify [app/main]
#   Test cases: 4 (add 3+5=8, sub 10-3=7, mul 4*6=24, div 10/2=5)
#   Save intent? [Y/n]
y

# 4. Execute intent
/intent execute calculator
# Expected: 4 tasks execute sequentially
#   [1/4] Create module calculator/ops... ✓
#   [2/4] Implement arithmetic functions... ✓
#   [3/4] Modify main to demo... ✓
#   [4/4] Verify test cases... ✓
#   Intent 'calculator' completed. 4/4 tasks, 4/4 tests passed.

# 5. Inspect the graph
/describe
# Expected: shows pseudocode for add, sub, mul, div functions

# 6. Build and run
/build
# Expected: "Build successful: .duumbi/build/output"
/run
# Expected: prints calculation results

# 7. Iterate — add a new function via chat
/add "Add a power(base, exp) function to calculator/ops that computes base^exp using a loop"
# Expected: proposes graph mutation, asks for confirmation
y
/build
/run
# Expected: power function result also printed
```

### Studio Walkthrough

```
# 1. Launch Studio (in the calculator workspace directory)
cd /tmp/duumbi-p15-calculator
$DUUMBI studio
# Expected: "DUUMBI Studio running at http://localhost:8421"
# Open http://localhost:8421 in your browser

# 2. Intents panel (default view)
# → Click "+" button
# → Type intent: "Build a calculator with add, subtract, multiply, divide..."
# → Wait for LLM to generate spec (~5 seconds)
# → Review: criteria, modules, test cases shown inline
# → Click "Save"
# → Click "Execute"
# Expected: task list shows progress [✓] [✓] [✓] [✓]

# 3. Graph panel (click "Graph" in footer)
# → C4 Context: shows software system node
# → Click to drill into Container
# → See: calculator/ops module + main module
# → Click calculator/ops → Component level
# → See: add, sub, mul, div functions
# → Click any function → Code level → see ops graph

# 4. Chat (right panel in Graph view)
# → Type: "Add a modulo function that returns a % b"
# → LLM streams response, proposes mutation
# → Click "Apply" or confirm
# → Graph refreshes — new modulo function appears

# 5. Build panel (click "Build" in footer)
# → Click "Build" → wait for compilation
# → "Build successful" → click "Run"
# → Output shown in terminal panel
```

---

## Sample 2: String Utilities (Moderate)

**Intent:** "Create a string utility library with functions: reverse a string, count vowels, check if palindrome. Demo all three in main."
**Expected:** 1 module (string/utils), 3 functions, main modified
**Time target:** <15 minutes (CLI), <18 minutes (Studio)

### CLI REPL Walkthrough

```bash
mkdir -p /tmp/duumbi-p15-strings && cd /tmp/duumbi-p15-strings
$DUUMBI init .
$DUUMBI

/intent create "Create a string utility library with functions: reverse a string, count vowels, check if palindrome. Demo all three in main."
y

/intent execute string-utility
# Expected: Planner → Coder → Tester pipeline
#   Plan: 3 tasks
#   [1/3] Create string/utils module with reverse, count_vowels, is_palindrome
#   [2/3] Implement string functions using StringLength, StringEquals ops
#   [3/3] Modify main to call all 3 and print results
#   Verifier: 3/3 tests passed

/build
/run
# Expected: prints reversed string, vowel count, palindrome check results
```

### Studio Walkthrough

Same as Sample 1, but observe:
- Agent panel (via ⌘K → "Agent templates") shows Planner + Coder + Tester were used
- Graph shows string/utils module with 3 functions
- Code level shows StringLength, StringEquals, StringConcat ops

---

## Sample 3: Math Library (Moderate — Cross-Module)

**Intent:** "Build a math library with: factorial (recursive), fibonacci (iterative), and is_prime functions. The main function should compute factorial(10), fibonacci(15), and check if 97 is prime."
**Expected:** 1 module (math/lib), 3 functions (one recursive, one with loops), main modified
**Time target:** <15 minutes (CLI), <18 minutes (Studio)

### CLI REPL Walkthrough

```bash
mkdir -p /tmp/duumbi-p15-math && cd /tmp/duumbi-p15-math
$DUUMBI init .
$DUUMBI

/intent create "Build a math library with: factorial (recursive), fibonacci (iterative), and is_prime functions. The main function should compute factorial(10), fibonacci(15), and check if 97 is prime."
y

/intent execute math-library
# Expected: Planner → Coder → Tester pipeline
#   [1/3] Create math/lib with factorial, fibonacci, is_prime
#   [2/3] Implement: factorial uses Call (recursion), fibonacci uses Branch (loop)
#   [3/3] Modify main: import math/lib, call all 3, print results
#   Verifier: 3/3 tests passed
#     factorial(10) = 3628800 ✓
#     fibonacci(15) = 610 ✓
#     is_prime(97) = 1 ✓

/describe
# Shows: factorial calls itself (recursive Call op)
#        fibonacci has Branch + multiple blocks (iterative)
#        is_prime has modulo check + early return

/build
/run
# Expected output:
#   factorial(10) = 3628800
#   fibonacci(15) = 610
#   is_prime(97) = 1
```

### Key Verification Points for Sample 3

- [ ] `math/lib` module has `duumbi:exports: ["factorial", "fibonacci", "is_prime"]`
- [ ] `main.jsonld` has `duumbi:imports: [{module: "math/lib"}]`
- [ ] factorial uses `duumbi:Call` with `duumbi:function: "factorial"` (self-reference)
- [ ] fibonacci uses `duumbi:Branch` with `trueBlock`/`falseBlock` for loop control
- [ ] is_prime uses `duumbi:Modulo` and `duumbi:Compare`

---

## Phase 15 Feature Verification

Before running sample tasks, verify the new Phase 15 features work:

### Context-Aware Chat (#478)
1. Open Studio → Graph panel → navigate to Context level
2. Send a chat message → observe the prompt is short (workspace overview only)
3. Drill down to Code level → send another message → prompt includes full ops
4. **Pass criteria:** Context-level chat uses less tokens than Code-level

### Live Graph Refresh (#479)
1. Open Studio → Graph panel at Component level
2. Chat: "Add a helper function called greet that prints hello"
3. After mutation succeeds, graph should reload automatically
4. New node appears with a fade-in animation
5. **Pass criteria:** No manual page refresh needed; new nodes animate in

### Panel Wiring (#480–#483)
1. **Footer:** Exactly 3 items — Intents, Graph, Build. No Plans/Agents/Registry
2. **Intents panel:** Type a description → click "Create & Plan" → intent appears in left list
3. **Intents panel:** Select intent → click "Execute" → status message updates
4. **Graph panel:** Breadcrumb shows `workspace > module > function > block` trail
5. **Build panel:** Click "Build" → output appears. Click "Run" → binary stdout shown
6. **Pass criteria:** All 3 panels fully functional without JS console errors

### Command Palette (#484–#485)
1. Press `⌘K` → type "agent" → "Agent Templates" item appears
2. Click → popup shows 5 seed templates (Planner, Coder, Reviewer, Tester, Repair)
3. Each card: name, role badge, tool count, system prompt preview
4. Press `⌘K` → type "provider" → "Configure Providers" opens settings popup
5. **Pass criteria:** All items accessible via ⌘K

---

## Troubleshooting

| Problem | Likely Cause | Fix |
|---------|-------------|-----|
| "No provider configured" | Missing `[[providers]]` in config.toml | `duumbi provider add anthropic` or edit `.duumbi/config.toml` |
| "LLM returned no tool calls" | Model doesn't support tool use | Use claude-sonnet-4-6 or gpt-4o |
| Intent execution hangs | API rate limit or timeout | Check `ANTHROPIC_API_KEY`, try again |
| Build fails with E006 | No main function | Ensure intent includes "modify main" |
| Build fails with E010 | Cross-module import missing | Check `duumbi:imports` in main.jsonld |
| Studio doesn't open | Port 8421 in use | `duumbi studio --port 8422` |
| Chat returns mock responses | WebSocket not connected | Check browser console for WS errors; ensure `/ws/chat` route is reachable |
| Graph doesn't refresh after chat | WS `result` frame not received | Check browser console for WS `result` frame with `refresh: true`; verify `studio.js` StudioWS.onResult handler |

---

## Regression Checklist

After all 3 samples pass, verify:

```bash
# All existing tests still pass
cargo test --all
# Expected: 1722+ tests green

# Clippy clean
cargo clippy --all-targets -- -D warnings

# Format check
cargo fmt --check
```

| # | Check | Expected |
|---|-------|----------|
| 1 | `cargo build` | 0 warnings |
| 2 | `cargo test --all` | All green |
| 3 | Sample 1 CLI | Completes <10 min |
| 4 | Sample 1 Studio | Completes, graph shows calculator |
| 5 | Sample 2 CLI | String ops work |
| 6 | Sample 2 Studio | Planner+Coder+Tester visible |
| 7 | Sample 3 CLI | Recursion + loops work |
| 8 | Sample 3 Studio | Cross-module imports visible in graph |
| 9 | Studio chat → LLM | Real streaming responses |
| 10 | ⌘K settings | Provider config accessible |
| 11 | ⌘K agent templates | 5 seed templates viewable |
| 12 | Graph refresh | Nodes animate after chat mutation |
| 13 | Breadcrumb | Shows drill-down path, clickable back-nav |
| 14 | Build panel | Build + Run buttons produce output |
| 15 | Intents panel | Create + Execute buttons wired |
