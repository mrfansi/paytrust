#!/usr/bin/env bash
# Parallel test validation script for PayTrust
# Validates test data isolation and repeatability by running tests multiple times in parallel

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
ITERATIONS="${1:-10}"  # Default: 10 iterations
TEST_THREADS="${TEST_THREADS:-4}"
LOG_DIR="/tmp/paytrust-test-parallel-$(date +%s)"

echo -e "${GREEN}=== PayTrust Parallel Test Validation ===${NC}"
echo -e "${BLUE}Iterations: ${ITERATIONS}${NC}"
echo -e "${BLUE}Test Threads: ${TEST_THREADS}${NC}"
echo -e "${BLUE}Log Directory: ${LOG_DIR}${NC}"
echo ""

# Create log directory
mkdir -p "$LOG_DIR"

# Function to run tests
run_test_iteration() {
    local iteration=$1
    local log_file="$LOG_DIR/iteration_${iteration}.log"
    
    echo -e "${YELLOW}Running iteration ${iteration}...${NC}"
    
    if cargo test --test-threads="$TEST_THREADS" > "$log_file" 2>&1; then
        echo -e "${GREEN}✓ Iteration ${iteration} PASSED${NC}"
        return 0
    else
        echo -e "${RED}✗ Iteration ${iteration} FAILED${NC}"
        echo -e "${RED}  See log: ${log_file}${NC}"
        return 1
    fi
}

# Track results
TOTAL=0
PASSED=0
FAILED=0
declare -a FAILED_ITERATIONS

# Run iterations
for i in $(seq 1 "$ITERATIONS"); do
    ((TOTAL++)) || true
    
    if run_test_iteration "$i"; then
        ((PASSED++)) || true
    else
        ((FAILED++)) || true
        FAILED_ITERATIONS+=("$i")
    fi
    
    echo ""
done

# Calculate pass rate
PASS_RATE=$(awk "BEGIN {printf \"%.1f\", ($PASSED / $TOTAL) * 100}")

# Summary
echo -e "${BLUE}=== Test Validation Summary ===${NC}"
echo -e "Total Iterations:  ${TOTAL}"
echo -e "Passed:           ${GREEN}${PASSED}${NC}"
echo -e "Failed:           ${RED}${FAILED}${NC}"
echo -e "Pass Rate:        ${PASS_RATE}%"
echo ""

# Show failed iterations
if [ ${#FAILED_ITERATIONS[@]} -gt 0 ]; then
    echo -e "${RED}Failed Iterations:${NC}"
    for iter in "${FAILED_ITERATIONS[@]}"; do
        echo -e "  - Iteration ${iter}: ${LOG_DIR}/iteration_${iter}.log"
    done
    echo ""
fi

# Check for data conflicts
echo -e "${BLUE}Checking for common test data conflicts...${NC}"
CONFLICT_COUNT=0

for log_file in "$LOG_DIR"/*.log; do
    if grep -q "Duplicate entry" "$log_file" 2>/dev/null; then
        echo -e "${RED}  ✗ Duplicate entry conflict found in $(basename "$log_file")${NC}"
        ((CONFLICT_COUNT++)) || true
    fi
    
    if grep -q "UNIQUE constraint failed" "$log_file" 2>/dev/null; then
        echo -e "${RED}  ✗ Unique constraint violation in $(basename "$log_file")${NC}"
        ((CONFLICT_COUNT++)) || true
    fi
    
    if grep -q "deadlock" "$log_file" 2>/dev/null; then
        echo -e "${RED}  ✗ Database deadlock in $(basename "$log_file")${NC}"
        ((CONFLICT_COUNT++)) || true
    fi
done

if [ "$CONFLICT_COUNT" -eq 0 ]; then
    echo -e "${GREEN}  ✓ No data conflicts detected${NC}"
else
    echo -e "${RED}  Found ${CONFLICT_COUNT} potential data conflicts${NC}"
fi
echo ""

# Performance check
echo -e "${BLUE}Analyzing test performance...${NC}"
TOTAL_TIME=0
for log_file in "$LOG_DIR"/*.log; do
    # Extract test duration (assumes cargo test reports "finished in X.XXs")
    if TIME=$(grep -oP 'finished in \K[0-9.]+' "$log_file" 2>/dev/null | head -1); then
        TOTAL_TIME=$(awk "BEGIN {print $TOTAL_TIME + $TIME}")
    fi
done

if [ "$TOTAL_TIME" != "0" ]; then
    AVG_TIME=$(awk "BEGIN {printf \"%.2f\", $TOTAL_TIME / $TOTAL}")
    echo -e "Average Test Duration: ${AVG_TIME}s per iteration"
    
    if (( $(awk "BEGIN {print ($AVG_TIME > 60) ? 1 : 0}") )); then
        echo -e "${YELLOW}  ⚠ Warning: Average duration exceeds 60s target${NC}"
    else
        echo -e "${GREEN}  ✓ Performance within target (<60s)${NC}"
    fi
else
    echo -e "${YELLOW}  ⚠ Could not extract timing information${NC}"
fi
echo ""

# Final verdict
echo -e "${BLUE}=== Final Verdict ===${NC}"

if [ "$FAILED" -eq 0 ] && [ "$CONFLICT_COUNT" -eq 0 ]; then
    echo -e "${GREEN}✓✓✓ ALL TESTS PASSED WITH 100% REPEATABILITY ✓✓✓${NC}"
    echo -e "${GREEN}Tests are ready for parallel execution in CI/CD${NC}"
    exit 0
elif [ "$PASS_RATE" = "100.0" ] && [ "$CONFLICT_COUNT" -gt 0 ]; then
    echo -e "${YELLOW}⚠ Tests passed but data conflicts detected${NC}"
    echo -e "${YELLOW}Review conflict logs to improve test isolation${NC}"
    exit 1
else
    echo -e "${RED}✗ TEST VALIDATION FAILED${NC}"
    echo -e "${RED}Pass rate: ${PASS_RATE}% (target: 100%)${NC}"
    echo -e "${RED}Review failed iteration logs in ${LOG_DIR}${NC}"
    exit 1
fi
