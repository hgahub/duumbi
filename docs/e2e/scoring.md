# Intent Create Quality Scoring

## Overview

Each generated intent is scored 0–100 against its gold standard across 5 dimensions.

## Dimensions

| Dimension | Weight | What it measures | Scoring |
|-----------|--------|-----------------|---------|
| **JSON validity** | 10 | Parseable JSON with all required fields | 10 = valid with all fields, 5 = valid but missing optional fields, 0 = invalid JSON or missing required fields |
| **Module structure** | 15 | modules_create and modules_modify match gold | 15 = exact match, 10 = correct count but different names, 5 = partially correct, 0 = wrong structure |
| **Test coverage** | 30 | Test cases: correct functions, args, expected_return | Per function: (matched_tests / gold_tests) * weight. A test matches if function name matches and expected_return is correct for given args |
| **Edge cases** | 25 | Presence of boundary/edge test cases | Score = (edge_tests_present / gold_edge_tests) * 25. Edge tests: zero args, negative args, boundary values, equal inputs |
| **Acceptance criteria** | 20 | Semantic coverage of gold criteria | Score = (covered_criteria / gold_criteria) * 20. A criterion is "covered" if the generated criteria semantically describe the same requirement |

## Required fields (JSON validity)

- `acceptance_criteria` (array of strings, non-empty)
- `modules_create` (array of strings)
- `modules_modify` (array of strings, should contain "app/main")
- `test_cases` (array of objects with name, function, args, expected_return)
- `dependencies` (array, usually empty)

## Test case matching rules

A generated test case **matches** a gold test case when:
1. `function` name is identical (case-sensitive)
2. `args` array has the same length
3. `expected_return` equals the gold value

Function name aliases are NOT accepted (e.g., "sub" != "subtract").
The generated intent should derive appropriate function names from the natural language description.

## Edge case classification

A test case is an "edge case" if any of these apply:
- Any arg is 0
- Any arg is negative
- Args are equal to each other
- Args are at documented boundaries (e.g., clamp at lo or hi)
- Input is a known special case (e.g., factorial(0), fib(0), fib(1))

## Automation

**Fully automatable:** JSON validity, module structure, test case function/args/return matching
**Semi-automatable:** Edge case classification (pattern matching on args)
**Manual/LLM-eval:** Acceptance criteria semantic coverage

## Scoring formula

```
total = json_validity + module_structure + test_coverage + edge_cases + acceptance_criteria
```

## Aggregation

- **Per-difficulty average:** mean score across Easy/Medium/Hard groups
- **Overall average:** mean score across all 30 tasks
- **Pass rate:** percentage of tasks scoring >= 70
