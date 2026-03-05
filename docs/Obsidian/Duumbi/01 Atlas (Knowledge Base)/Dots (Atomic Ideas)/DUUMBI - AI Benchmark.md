---
tags:
  - project/duumbi
  - doc/testing
status: draft
created: 2026-02-27
updated: 2026-02-27
related_maps:
  - "[[DUUMBI - MVP Specification]]"
  - "[[DUUMBI - Task List]]"
---
# DUUMBI — Phase 2 AI Benchmark (20 Commands)

> Kill criterion: >70% correct mutations (≥14 of 20 commands).
> A mutation is **correct** if: (1) `duumbi check` passes, (2) `duumbi build` succeeds, (3) graph diff structurally matches gold standard (ignoring `@id` values).

## Starting State

Each command starts from a clean workspace with the skeleton `main.jsonld` (the `add(3,5)` program from `duumbi init`).

## Commands

### Category A: Add Constants & Prints (4 commands)

| # | Command | Expected Mutation |
|---|---------|-------------------|
| 1 | `"add a constant 42 and print it"` | Add Const(42) + Print node in main/entry block |
| 2 | `"add a float constant 3.14 and print it"` | Add ConstF64(3.14) + Print node (tests f64 type) |
| 3 | `"add a boolean true and print it"` | Add ConstBool(true) + Print node (tests bool type) |
| 4 | `"print the number 100"` | Add Const(100) + Print node (natural phrasing) |

### Category B: Modify Existing Nodes (4 commands)

| # | Command | Expected Mutation |
|---|---------|-------------------|
| 5 | `"change the first constant from 3 to 10"` | Modify Const(3) → Const(10) |
| 6 | `"change the addition to subtraction"` | Modify Op::Add → Op::Sub |
| 7 | `"remove the print statement"` | Remove the Print node and its edge |
| 8 | `"change the return value to 0"` | Add Const(0), redirect Return operand edge |

### Category C: Add Functions (4 commands)

| # | Command | Expected Mutation |
|---|---------|-------------------|
| 9 | `"add a function called double that takes an i64 and returns it multiplied by 2"` | New function with param, Mul(Load(n), Const(2)), Return |
| 10 | `"add a function called add_one that takes an i64 and returns n+1"` | New function with param, Add(Load(n), Const(1)), Return |
| 11 | `"add a function called is_positive that returns true if input > 0"` | New function with Compare(Gt), Return |
| 12 | `"add a max function that returns the larger of two i64 parameters"` | New function with Compare, Branch, Return in each branch |

### Category D: Control Flow (4 commands)

| # | Command | Expected Mutation |
|---|---------|-------------------|
| 13 | `"add an if: if the result is greater than 5, print it"` | Compare + Branch + new blocks with/without Print |
| 14 | `"add a branch: if the result equals 8, return 1, otherwise return 0"` | Compare(Eq) + Branch + two return blocks |
| 15 | `"add a function that computes absolute value of an i64"` | Function with Compare(Lt, n, 0) + Branch + Sub(0, n) |
| 16 | `"add a function that returns 1 if input is even, 0 if odd"` | Function with Div + Mul + Compare + Branch |

### Category E: Integration / Complex (4 commands)

| # | Command | Expected Mutation |
|---|---------|-------------------|
| 17 | `"call the double function on the add result and print it"` | Add Call(double, [add_result]) + Print (requires #9 first — run on modified graph) |
| 18 | `"add a function that computes factorial recursively"` | Recursive function with Compare, Branch, Call(self), Mul |
| 19 | `"replace the entire main function body: compute 10*5, print, return"` | Clear main ops, add Const(10), Const(5), Mul, Print, Return |
| 20 | `"add a function that sums integers from 1 to n using recursion"` | Recursive sum with Compare(n ≤ 1), Branch, Add(n, Call(sum, n-1)) |

## Scoring

- **Pass**: All 3 criteria met (check passes, build succeeds, structural diff match)
- **Partial**: Check passes but diff doesn't match exactly (correct semantics, different structure)
- **Fail**: Check fails or build fails

Target: ≥14/20 Pass (70%)

## Notes

- Commands 17 requires command 9 to have been applied first (integration test)
- Commands 18 and 20 test recursion (the hardest category)
- Gold standard diffs stored in `tests/ai_benchmark/` as `.jsonld` files
- CI tests use mock LLM responses — never call live APIs
