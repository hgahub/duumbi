#!/usr/bin/env bash
# Intent Create Quality Evaluation Harness
#
# Usage:
#   ./scripts/eval_intent.sh [--provider minimax|anthropic] [--filter E|M|H]
#
# Runs `duumbi intent create` for each corpus task, compares the generated
# intent against the gold standard, and produces a score report.
#
# Prerequisites:
#   - `duumbi` binary in PATH (or cargo build first)
#   - A configured LLM provider in .duumbi/config.toml
#   - yq (YAML processor) installed: brew install yq

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
CORPUS_DIR="$PROJECT_DIR/docs/e2e/corpus"
RESULTS_DIR="$PROJECT_DIR/docs/e2e/results"

# Parse args
PROVIDER="default"
FILTER=""
while [[ $# -gt 0 ]]; do
    case "$1" in
        --provider) PROVIDER="$2"; shift 2 ;;
        --filter)   FILTER="$2"; shift 2 ;;
        *)          echo "Unknown arg: $1"; exit 1 ;;
    esac
done

# Ensure results dir
mkdir -p "$RESULTS_DIR"

TIMESTAMP=$(date +%Y%m%d_%H%M%S)
REPORT="$RESULTS_DIR/${PROVIDER}_${TIMESTAMP}.json"

echo "=== Intent Create Eval ==="
echo "Provider: $PROVIDER"
echo "Filter:   ${FILTER:-all}"
echo "Report:   $REPORT"
echo ""

# Resolve duumbi binary: prefer cargo run if no binary in PATH
if command -v duumbi &>/dev/null; then
    DUUMBI="duumbi"
else
    echo "Building duumbi..."
    (cd "$PROJECT_DIR" && cargo build --quiet 2>/dev/null)
    DUUMBI="$PROJECT_DIR/target/debug/duumbi"
fi

# Init workspace if needed
if [[ ! -d "$PROJECT_DIR/.duumbi" ]]; then
    echo "Initializing workspace..."
    (cd "$PROJECT_DIR" && "$DUUMBI" init --yes 2>/dev/null || true)
fi

# Collect results
RESULTS="[]"
TOTAL=0
PASSED=0
FAILED=0
ERRORS=0

for txt_file in "$CORPUS_DIR"/*.txt; do
    task_id=$(basename "$txt_file" .txt)

    # Apply filter
    if [[ -n "$FILTER" ]] && [[ "$task_id" != ${FILTER}* ]]; then
        continue
    fi

    gold_file="$CORPUS_DIR/${task_id}.gold.yaml"
    if [[ ! -f "$gold_file" ]]; then
        echo "SKIP $task_id (no gold standard)"
        continue
    fi

    description=$(cat "$txt_file")
    TOTAL=$((TOTAL + 1))

    echo -n "[$TOTAL] $task_id ... "

    # Snapshot existing intents before this run
    BEFORE_FILES=$(ls "$PROJECT_DIR/.duumbi/intents/"*.yaml 2>/dev/null | sort || true)

    # Run intent create
    GENERATED=""
    if output=$(cd "$PROJECT_DIR" && "$DUUMBI" intent create "$description" --yes 2>&1); then
        # Find the newly created file (diff against snapshot)
        AFTER_FILES=$(ls "$PROJECT_DIR/.duumbi/intents/"*.yaml 2>/dev/null | sort || true)
        GENERATED=$(comm -13 <(echo "$BEFORE_FILES") <(echo "$AFTER_FILES") | head -1)

        # Fallback: if comm finds nothing, use the most recently modified file
        if [[ -z "$GENERATED" ]]; then
            GENERATED=$(ls -t "$PROJECT_DIR/.duumbi/intents/"*.yaml 2>/dev/null | head -1)
        fi
    fi

    if [[ -z "$GENERATED" ]] || [[ ! -f "$GENERATED" ]]; then
        echo "ERROR (no output)"
        ERRORS=$((ERRORS + 1))
        RESULTS=$(echo "$RESULTS" | jq --arg id "$task_id" --arg status "error" \
            '. + [{"task": $id, "status": $status, "score": 0}]')
        continue
    fi

    # --- Scoring ---
    score_json=10
    score_modules=0
    score_tests=0
    score_edge=0
    score_criteria=0

    # 1. JSON validity (10 pts) — if we got here, it parsed
    # Already 10

    # 2. Module structure (15 pts)
    gold_modules_create=$(yq -r '.modules.create[]' "$gold_file" 2>/dev/null | sort)
    gen_modules_create=$(yq -r '.modules.create[]' "$GENERATED" 2>/dev/null | sort)
    gold_modules_modify=$(yq -r '.modules.modify[]' "$gold_file" 2>/dev/null | sort)
    gen_modules_modify=$(yq -r '.modules.modify[]' "$GENERATED" 2>/dev/null | sort)

    if [[ "$gold_modules_create" == "$gen_modules_create" ]] && [[ "$gold_modules_modify" == "$gen_modules_modify" ]]; then
        score_modules=15
    elif [[ $(echo "$gold_modules_create" | wc -l) -eq $(echo "$gen_modules_create" | wc -l) ]]; then
        score_modules=10
    elif [[ -n "$gen_modules_create" ]]; then
        score_modules=5
    fi

    # 3. Test coverage (30 pts) — fuzzy function matching by arg count + expected_return
    #    Strategy: match each gold function to a generated function by comparing
    #    test signatures (arg count + expected_return values). This handles
    #    name differences like "sum" vs "add" or "max_of_two" vs "max".
    gold_func_count=$(yq -r '.test_cases[].function' "$gold_file" 2>/dev/null | sort -u | grep -c . || true)
    gen_func_count=$(yq -r '.test_cases[].function' "$GENERATED" 2>/dev/null | sort -u | grep -c . || true)

    if [[ $gold_func_count -gt 0 ]] && [[ $gen_func_count -gt 0 ]]; then
        # Build signature sets: for each unique function, collect "argcount:expected_return" pairs
        gold_sigs=$(yq -r '.test_cases[] | "\(.function)|\(.args | length)|\(.expected_return)"' "$gold_file" 2>/dev/null | sort)
        gen_sigs=$(yq -r '.test_cases[] | "\(.function)|\(.args | length)|\(.expected_return)"' "$GENERATED" 2>/dev/null | sort)

        # For each gold function, try to find a matching generated function
        # by checking if their test signatures (argcount + return) overlap
        matched_funcs=0
        used_gen_funcs=""
        for gold_func in $(yq -r '.test_cases[].function' "$gold_file" 2>/dev/null | sort -u); do
            # Get gold signatures for this function: "argcount:return" per test
            gold_func_sigs=$(echo "$gold_sigs" | grep "^${gold_func}|" | sed "s/^[^|]*|//" | sort)
            gold_sig_count=$(echo "$gold_func_sigs" | grep -c . || true)

            best_match=""
            best_overlap=0
            for gen_func in $(yq -r '.test_cases[].function' "$GENERATED" 2>/dev/null | sort -u); do
                # Skip already matched
                if echo "$used_gen_funcs" | grep -q "^${gen_func}$"; then continue; fi

                gen_func_sigs=$(echo "$gen_sigs" | grep "^${gen_func}|" | sed "s/^[^|]*|//" | sort)
                # Count overlapping signatures
                overlap=$(comm -12 <(echo "$gold_func_sigs") <(echo "$gen_func_sigs") | grep -c . || true)
                if [[ $overlap -gt $best_overlap ]]; then
                    best_overlap=$overlap
                    best_match=$gen_func
                fi
            done

            # Accept match if at least 1 test signature overlaps
            if [[ $best_overlap -gt 0 ]]; then
                matched_funcs=$((matched_funcs + 1))
                used_gen_funcs=$(printf '%s\n%s' "$used_gen_funcs" "$best_match")
            fi
        done

        score_tests=$((30 * matched_funcs / gold_func_count))
    fi

    # 4. Edge cases (25 pts) — count test cases with zero or negative args
    gold_edge_count=$(yq -r '.test_cases[] | select(.args[] == 0 or .args[] < 0) | .name' "$gold_file" 2>/dev/null | sort -u | grep -c . || true)
    gen_edge_count=$(yq -r '.test_cases[] | select(.args[] == 0 or .args[] < 0) | .name' "$GENERATED" 2>/dev/null | sort -u | grep -c . || true)

    if [[ $gold_edge_count -gt 0 ]]; then
        if [[ $gen_edge_count -ge $gold_edge_count ]]; then
            score_edge=25
        else
            score_edge=$((25 * gen_edge_count / gold_edge_count))
        fi
    else
        score_edge=25  # No edge cases expected
    fi

    # 5. Acceptance criteria (20 pts) — count overlap
    gold_criteria_count=$(yq -r '.acceptance_criteria | length' "$gold_file" 2>/dev/null)
    gen_criteria_count=$(yq -r '.acceptance_criteria | length' "$GENERATED" 2>/dev/null)

    if [[ $gold_criteria_count -gt 0 ]] && [[ $gen_criteria_count -gt 0 ]]; then
        # Simple heuristic: score by ratio of generated vs gold criteria count
        if [[ $gen_criteria_count -ge $gold_criteria_count ]]; then
            score_criteria=20
        else
            score_criteria=$((20 * gen_criteria_count / gold_criteria_count))
        fi
    fi

    total_score=$((score_json + score_modules + score_tests + score_edge + score_criteria))

    if [[ $total_score -ge 70 ]]; then
        PASSED=$((PASSED + 1))
        status="pass"
    else
        FAILED=$((FAILED + 1))
        status="fail"
    fi

    echo "$status ($total_score/100) [json=$score_json mod=$score_modules test=$score_tests edge=$score_edge crit=$score_criteria]"

    RESULTS=$(echo "$RESULTS" | jq \
        --arg id "$task_id" \
        --arg status "$status" \
        --argjson score "$total_score" \
        --argjson json "$score_json" \
        --argjson mod "$score_modules" \
        --argjson test "$score_tests" \
        --argjson edge "$score_edge" \
        --argjson crit "$score_criteria" \
        '. + [{"task": $id, "status": $status, "score": $score, "json": $json, "modules": $mod, "tests": $test, "edge_cases": $edge, "criteria": $crit}]')
done

# Summary
echo ""
echo "=== Summary ==="
echo "Total:  $TOTAL"
echo "Passed: $PASSED (>= 70)"
echo "Failed: $FAILED"
echo "Errors: $ERRORS"

if [[ $TOTAL -gt 0 ]]; then
    AVG=$(echo "$RESULTS" | jq '[.[].score] | add / length | floor')
    echo "Average: $AVG/100"
    PASS_RATE=$((100 * PASSED / TOTAL))
    echo "Pass rate: $PASS_RATE%"
fi

# Write report
echo "$RESULTS" | jq '{
    provider: "'"$PROVIDER"'",
    timestamp: "'"$TIMESTAMP"'",
    total: '"$TOTAL"',
    passed: '"$PASSED"',
    failed: '"$FAILED"',
    errors: '"$ERRORS"',
    results: .
}' > "$REPORT"

echo ""
echo "Report saved: $REPORT"
