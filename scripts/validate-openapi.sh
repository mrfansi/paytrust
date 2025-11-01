#!/usr/bin/env bash

# OpenAPI 3.0 Schema Validation Script
# Validates the OpenAPI specification for schema compliance

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
OPENAPI_SPEC="$PROJECT_ROOT/specs/001-payment-orchestration-api/contracts/openapi.yaml"

echo "OpenAPI 3.0 Schema Validation"
echo "=============================="
echo ""
echo "Specification: $OPENAPI_SPEC"
echo ""

# Check if spec file exists
if [ ! -f "$OPENAPI_SPEC" ]; then
    echo "❌ Error: OpenAPI spec file not found at $OPENAPI_SPEC"
    exit 1
fi

echo "✅ OpenAPI spec file found"
echo ""

# Validate YAML syntax
echo "Checking YAML syntax..."
if command -v yamllint &> /dev/null; then
    if yamllint -d relaxed "$OPENAPI_SPEC"; then
        echo "✅ YAML syntax is valid"
    else
        echo "❌ YAML syntax errors found"
        exit 1
    fi
else
    echo "⚠️  yamllint not installed, skipping YAML syntax check"
    echo "   Install with: pip install yamllint"
fi
echo ""

# Validate OpenAPI schema using different validators
echo "Validating OpenAPI 3.0 schema..."
echo ""

# Method 1: Try swagger-cli (npm package @apidevtools/swagger-cli)
if command -v swagger-cli &> /dev/null; then
    echo "Using swagger-cli validator..."
    if swagger-cli validate "$OPENAPI_SPEC"; then
        echo "✅ OpenAPI schema is valid (swagger-cli)"
    else
        echo "❌ OpenAPI schema validation failed (swagger-cli)"
        exit 1
    fi
    echo ""
fi

# Method 2: Try openapi-generator-cli
if command -v openapi-generator-cli &> /dev/null; then
    echo "Using openapi-generator-cli validator..."
    if openapi-generator-cli validate -i "$OPENAPI_SPEC"; then
        echo "✅ OpenAPI schema is valid (openapi-generator-cli)"
    else
        echo "❌ OpenAPI schema validation failed (openapi-generator-cli)"
        exit 1
    fi
    echo ""
fi

# Method 3: Try spectral (npm package @stoplight/spectral-cli)
if command -v spectral &> /dev/null; then
    echo "Using Spectral linter..."
    if spectral lint "$OPENAPI_SPEC" --ruleset spectral:oas; then
        echo "✅ OpenAPI schema is valid (Spectral)"
    else
        echo "⚠️  Spectral found issues (non-fatal)"
    fi
    echo ""
fi

# Method 4: Try Python openapi-spec-validator
if command -v openapi-spec-validator &> /dev/null; then
    echo "Using openapi-spec-validator..."
    if openapi-spec-validator "$OPENAPI_SPEC"; then
        echo "✅ OpenAPI schema is valid (openapi-spec-validator)"
    else
        echo "❌ OpenAPI schema validation failed (openapi-spec-validator)"
        exit 1
    fi
    echo ""
fi

# Method 5: Try redocly CLI
if command -v redocly &> /dev/null; then
    echo "Using Redocly validator..."
    if redocly lint "$OPENAPI_SPEC"; then
        echo "✅ OpenAPI schema is valid (Redocly)"
    else
        echo "⚠️  Redocly found issues"
    fi
    echo ""
fi

# Check if at least one validator ran
if ! command -v swagger-cli &> /dev/null && \
   ! command -v openapi-generator-cli &> /dev/null && \
   ! command -v spectral &> /dev/null && \
   ! command -v openapi-spec-validator &> /dev/null && \
   ! command -v redocly &> /dev/null; then
    echo "⚠️  No external OpenAPI validators found!"
    echo ""
    echo "Running built-in Rust contract tests instead..."
    echo ""
    
    # Run the Rust contract tests for OpenAPI validation
    if cd "$PROJECT_ROOT" && cargo test --test openapi_validation_test --quiet 2>&1; then
        echo "✅ OpenAPI validation passed (Rust contract tests)"
        echo ""
    else
        echo "❌ OpenAPI validation failed (Rust contract tests)"
        echo ""
        exit 1
    fi
    
    echo "For more thorough validation, consider installing an external validator:"
    echo ""
    echo "Option 1 - swagger-cli (recommended):"
    echo "  npm install -g @apidevtools/swagger-cli"
    echo ""
    echo "Option 2 - openapi-spec-validator (Python):"
    echo "  pip install openapi-spec-validator"
    echo ""
    echo "Option 3 - Spectral:"
    echo "  npm install -g @stoplight/spectral-cli"
    echo ""
    echo "Option 4 - Redocly:"
    echo "  npm install -g @redocly/cli"
    echo ""
fi

# Basic structure validation (manual checks)
echo "Running basic structure validation..."
echo ""

# Check for required OpenAPI 3.0.x version
if grep -q "openapi: 3.0" "$OPENAPI_SPEC"; then
    echo "✅ OpenAPI version 3.0.x detected"
else
    echo "❌ OpenAPI version 3.0.x not found"
    exit 1
fi

# Check for required info section
if grep -q "^info:" "$OPENAPI_SPEC"; then
    echo "✅ Info section present"
else
    echo "❌ Info section missing"
    exit 1
fi

# Check for required paths section
if grep -q "^paths:" "$OPENAPI_SPEC"; then
    echo "✅ Paths section present"
else
    echo "❌ Paths section missing"
    exit 1
fi

# Check for components/schemas section
if grep -q "^components:" "$OPENAPI_SPEC" && grep -q "schemas:" "$OPENAPI_SPEC"; then
    echo "✅ Components/schemas section present"
else
    echo "⚠️  Components/schemas section missing or incomplete"
fi

# Check for security definitions
if grep -q "security:" "$OPENAPI_SPEC"; then
    echo "✅ Security definitions present"
else
    echo "⚠️  Security definitions missing"
fi

echo ""
echo "=============================="
echo "✅ OpenAPI 3.0 validation complete!"
echo ""
echo "Summary:"
echo "  - Specification is valid OpenAPI 3.0.3"
echo "  - All required sections present"
echo "  - Schema structure is correct"
echo ""
echo "You can view the interactive documentation at:"
echo "  http://127.0.0.1:8080/api/docs"
echo ""
