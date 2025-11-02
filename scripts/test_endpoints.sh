#!/bin/bash

# PayTrust API Endpoint Testing Script
# Tests all implemented endpoints with sample requests

BASE_URL="http://127.0.0.1:8080"
API_KEY="test_api_key_bypass"  # For testing - bypass auth in dev

echo "🧪 PayTrust API Endpoint Testing"
echo "=================================="
echo ""

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test counter
PASS=0
FAIL=0

# Function to test endpoint
test_endpoint() {
    local name="$1"
    local method="$2"
    local endpoint="$3"
    local data="$4"
    local expected_status="$5"
    
    echo -n "Testing: $name... "
    
    if [ "$method" = "GET" ]; then
        response=$(curl -s -w "\n%{http_code}" "$BASE_URL$endpoint")
    else
        response=$(curl -s -w "\n%{http_code}" -X "$method" "$BASE_URL$endpoint" \
            -H "Content-Type: application/json" \
            -H "X-API-Key: $API_KEY" \
            -d "$data")
    fi
    
    status=$(echo "$response" | tail -n1)
    body=$(echo "$response" | sed '$d')
    
    if [ "$status" = "$expected_status" ]; then
        echo -e "${GREEN}✓ PASS${NC} (HTTP $status)"
        PASS=$((PASS + 1))
        if [ -n "$body" ] && [ "$body" != "null" ]; then
            echo "$body" | jq '.' 2>/dev/null || echo "$body"
        fi
    else
        echo -e "${RED}✗ FAIL${NC} (Expected $expected_status, got $status)"
        FAIL=$((FAIL + 1))
        echo "$body"
    fi
    echo ""
}

# 1. Health Check
echo "📋 Health & Status Endpoints"
echo "----------------------------"
test_endpoint "Health Check" "GET" "/health" "" "200"
test_endpoint "Ready Check" "GET" "/ready" "" "200"
test_endpoint "Root Endpoint" "GET" "/" "" "200"
test_endpoint "Metrics" "GET" "/metrics" "" "200"

# 2. Invoice Endpoints
echo ""
echo "📄 Invoice Endpoints"
echo "----------------------------"

# Create invoice (will fail without proper auth/gateway setup, but tests routing)
INVOICE_DATA='{
  "external_id": "TEST-INV-001",
  "gateway_id": "gateway-xendit-idr",
  "currency": "IDR",
  "line_items": [
    {
      "description": "Test Product",
      "quantity": 1,
      "unit_price": "100000",
      "tax_rate": "0.10"
    }
  ]
}'

test_endpoint "Create Invoice" "POST" "/v1/invoices" "$INVOICE_DATA" "201"

# List invoices
test_endpoint "List Invoices" "GET" "/v1/invoices?limit=10&offset=0" "" "200"

# Get specific invoice (will 404 if not exists)
test_endpoint "Get Invoice by ID" "GET" "/v1/invoices/test-id" "" "404"

# 3. Installment Endpoints
echo ""
echo "💳 Installment Endpoints"
echo "----------------------------"
test_endpoint "Get Installments" "GET" "/v1/installments/test-invoice-id" "" "404"

# 4. Reports Endpoints
echo ""
echo "📊 Reports Endpoints"
echo "----------------------------"
test_endpoint "Financial Report" "GET" "/v1/reports/financial?start_date=2025-01-01&end_date=2025-12-31" "" "200"

# 5. Gateway Endpoints
echo ""
echo "🔌 Gateway Endpoints"
echo "----------------------------"
test_endpoint "List Gateways" "GET" "/v1/gateways" "" "200"

# 6. Tax Endpoints
echo ""
echo "💰 Tax Endpoints"
echo "----------------------------"
test_endpoint "List Active Taxes" "GET" "/v1/taxes" "" "200"
test_endpoint "Get Tax by ID" "GET" "/v1/taxes/test-id" "" "404"

# Summary
echo ""
echo "=================================="
echo "📊 Test Summary"
echo "=================================="
echo -e "Passed: ${GREEN}$PASS${NC}"
echo -e "Failed: ${RED}$FAIL${NC}"
echo -e "Total:  $((PASS + FAIL))"

if [ $FAIL -eq 0 ]; then
    echo -e "\n${GREEN}✓ All tests passed!${NC}"
    exit 0
else
    echo -e "\n${YELLOW}⚠ Some tests failed${NC}"
    exit 1
fi
