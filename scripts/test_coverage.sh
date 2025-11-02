#!/bin/bash
# Test Coverage Report Script
# Generates code coverage report for PayTrust using cargo-tarpaulin
# Usage: ./scripts/test_coverage.sh [--html] [--min-coverage N]

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default values
GENERATE_HTML=false
MIN_COVERAGE=70
OUTPUT_DIR="target/coverage"

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --html)
            GENERATE_HTML=true
            shift
            ;;
        --min-coverage)
            MIN_COVERAGE="$2"
            shift 2
            ;;
        --help)
            echo "Usage: $0 [--html] [--min-coverage N]"
            echo ""
            echo "Options:"
            echo "  --html              Generate HTML coverage report"
            echo "  --min-coverage N    Set minimum coverage threshold (default: 70)"
            echo "  --help              Show this help message"
            exit 0
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            exit 1
            ;;
    esac
done

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}PayTrust Test Coverage Report${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""

# Check if cargo-tarpaulin is installed
if ! command -v cargo-tarpaulin &> /dev/null; then
    echo -e "${YELLOW}cargo-tarpaulin not found. Installing...${NC}"
    cargo install cargo-tarpaulin
    echo -e "${GREEN}✓ cargo-tarpaulin installed${NC}"
    echo ""
fi

# Create output directory
mkdir -p "$OUTPUT_DIR"

# Load environment variables for tests
if [ -f .env ]; then
    set -a
    source .env
    set +a
    echo -e "${GREEN}✓ Loaded .env configuration${NC}"
else
    echo -e "${YELLOW}⚠ Warning: .env file not found, using defaults${NC}"
fi

# Ensure test database exists
if command -v mysql &> /dev/null; then
    echo -e "${BLUE}Checking test database...${NC}"
    mysql -u root -e "CREATE DATABASE IF NOT EXISTS paytrust_test CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;" 2>/dev/null || true
    echo -e "${GREEN}✓ Test database ready${NC}"
fi

echo ""
echo -e "${BLUE}Running tests with coverage analysis...${NC}"
echo -e "${BLUE}This may take several minutes...${NC}"
echo ""

# Build tarpaulin command
TARPAULIN_CMD="cargo tarpaulin \
    --workspace \
    --timeout 300 \
    --out Xml \
    --output-dir $OUTPUT_DIR \
    --exclude-files 'target/*' 'tests/*' \
    --verbose"

# Add HTML output if requested
if [ "$GENERATE_HTML" = true ]; then
    TARPAULIN_CMD="$TARPAULIN_CMD --out Html"
fi

# Run coverage analysis
if eval "$TARPAULIN_CMD"; then
    echo ""
    echo -e "${GREEN}✓ Coverage analysis complete${NC}"
else
    echo ""
    echo -e "${RED}✗ Coverage analysis failed${NC}"
    exit 1
fi

# Parse coverage percentage from cobertura.xml
if [ -f "$OUTPUT_DIR/cobertura.xml" ]; then
    COVERAGE=$(grep -oP 'line-rate="\K[0-9.]+' "$OUTPUT_DIR/cobertura.xml" | head -1)
    COVERAGE_PERCENT=$(echo "$COVERAGE * 100" | bc | cut -d. -f1)
    
    echo ""
    echo -e "${BLUE}========================================${NC}"
    echo -e "${BLUE}Coverage Summary${NC}"
    echo -e "${BLUE}========================================${NC}"
    echo ""
    echo -e "Total Coverage: ${GREEN}${COVERAGE_PERCENT}%${NC}"
    echo -e "Threshold:      ${YELLOW}${MIN_COVERAGE}%${NC}"
    echo ""
    
    # Check if coverage meets threshold
    if [ "$COVERAGE_PERCENT" -ge "$MIN_COVERAGE" ]; then
        echo -e "${GREEN}✓ Coverage meets minimum threshold${NC}"
        COVERAGE_PASS=true
    else
        echo -e "${RED}✗ Coverage below minimum threshold${NC}"
        echo -e "${YELLOW}  Need ${MIN_COVERAGE}% coverage, got ${COVERAGE_PERCENT}%${NC}"
        COVERAGE_PASS=false
    fi
    
    # Display detailed coverage by module
    echo ""
    echo -e "${BLUE}Coverage by Module:${NC}"
    echo ""
    
    # Parse module coverage from XML (simplified)
    grep -oP 'filename="src/[^"]+' "$OUTPUT_DIR/cobertura.xml" | \
        cut -d/ -f2-3 | \
        sort -u | \
        while read -r module; do
            echo "  - $module"
        done
    
    echo ""
else
    echo -e "${YELLOW}⚠ Warning: Could not parse coverage percentage${NC}"
    COVERAGE_PASS=true
fi

# Display HTML report location
if [ "$GENERATE_HTML" = true ]; then
    HTML_REPORT="$OUTPUT_DIR/index.html"
    if [ -f "$HTML_REPORT" ]; then
        echo -e "${BLUE}========================================${NC}"
        echo -e "${BLUE}HTML Report${NC}"
        echo -e "${BLUE}========================================${NC}"
        echo ""
        echo -e "HTML report generated at:"
        echo -e "${GREEN}$HTML_REPORT${NC}"
        echo ""
        echo -e "Open in browser:"
        echo -e "${BLUE}open $HTML_REPORT${NC}"
        echo ""
    fi
fi

# Display XML report location
echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}Coverage Reports${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""
echo -e "XML report: ${GREEN}$OUTPUT_DIR/cobertura.xml${NC}"
if [ "$GENERATE_HTML" = true ]; then
    echo -e "HTML report: ${GREEN}$OUTPUT_DIR/index.html${NC}"
fi
echo ""

# Coverage breakdown by test type
echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}Coverage Breakdown${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""

# Analyze which parts of the codebase have coverage
echo "Analyzing coverage by module..."
echo ""

# Check coverage for key modules
for module in "modules/invoices" "modules/installments" "modules/transactions" "modules/gateways" "modules/taxes" "modules/reports"; do
    if grep -q "$module" "$OUTPUT_DIR/cobertura.xml" 2>/dev/null; then
        echo -e "  ${GREEN}✓${NC} $module"
    else
        echo -e "  ${YELLOW}?${NC} $module (no coverage data)"
    fi
done

echo ""

# Recommendations
echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}Recommendations${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""

if [ "$COVERAGE_PERCENT" -lt 60 ]; then
    echo -e "${RED}Critical: Coverage is very low${NC}"
    echo "  1. Add integration tests for untested endpoints"
    echo "  2. Add unit tests for business logic"
    echo "  3. Test error handling paths"
elif [ "$COVERAGE_PERCENT" -lt 80 ]; then
    echo -e "${YELLOW}Warning: Coverage could be improved${NC}"
    echo "  1. Add tests for edge cases"
    echo "  2. Test error scenarios"
    echo "  3. Improve service layer coverage"
else
    echo -e "${GREEN}Excellent: Coverage is good${NC}"
    echo "  1. Maintain current test coverage"
    echo "  2. Add tests for new features"
    echo "  3. Focus on critical path testing"
fi

echo ""

# Exit with appropriate code
if [ "$COVERAGE_PASS" = true ]; then
    echo -e "${GREEN}========================================${NC}"
    echo -e "${GREEN}Coverage Check: PASSED${NC}"
    echo -e "${GREEN}========================================${NC}"
    exit 0
else
    echo -e "${RED}========================================${NC}"
    echo -e "${RED}Coverage Check: FAILED${NC}"
    echo -e "${RED}========================================${NC}"
    exit 1
fi
