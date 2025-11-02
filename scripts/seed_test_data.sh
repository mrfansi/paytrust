#!/usr/bin/env bash
# Seed test data for PayTrust integration tests
# Supports both manual seeding and environment variable configuration

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration from environment or defaults
TEST_DB_NAME="${TEST_DB_NAME:-paytrust_test}"
DB_HOST="${DB_HOST:-localhost}"
DB_PORT="${DB_PORT:-3306}"
DB_USER="${DB_USER:-root}"
DB_PASS="${DB_PASS:-password}"

# Gateway configuration from environment or defaults
XENDIT_GATEWAY_ID="${XENDIT_GATEWAY_ID:-xendit-test-001}"
MIDTRANS_GATEWAY_ID="${MIDTRANS_GATEWAY_ID:-midtrans-test-001}"
XENDIT_API_KEY="${XENDIT_TEST_API_KEY:-xnd_development_test_key}"
MIDTRANS_SERVER_KEY="${MIDTRANS_SERVER_KEY:-SB-Mid-server-test_key}"

echo -e "${GREEN}PayTrust Test Data Seeding${NC}"
echo "============================"
echo ""

# Check if MySQL is running
echo -n "Checking MySQL connection... "
if ! mysql -h"${DB_HOST}" -P"${DB_PORT}" -u"${DB_USER}" -p"${DB_PASS}" -e "SELECT 1" > /dev/null 2>&1; then
    echo -e "${RED}FAILED${NC}"
    echo -e "${RED}Error: Cannot connect to MySQL at ${DB_HOST}:${DB_PORT}${NC}"
    echo "Please ensure MySQL is running and credentials are correct"
    exit 1
fi
echo -e "${GREEN}OK${NC}"

# Check if database exists
echo -n "Checking database ${TEST_DB_NAME}... "
if ! mysql -h"${DB_HOST}" -P"${DB_PORT}" -u"${DB_USER}" -p"${DB_PASS}" -e "USE ${TEST_DB_NAME}" 2>/dev/null; then
    echo -e "${RED}NOT FOUND${NC}"
    echo -e "${RED}Error: Database ${TEST_DB_NAME} does not exist${NC}"
    echo "Run scripts/setup_test_db.sh first to create the database"
    exit 1
fi
echo -e "${GREEN}OK${NC}"

# Seed test gateway data
echo ""
echo -n "Seeding test gateway data... "
mysql -h"${DB_HOST}" -P"${DB_PORT}" -u"${DB_USER}" -p"${DB_PASS}" "${TEST_DB_NAME}" << EOF
-- Insert test payment gateways
INSERT INTO payment_gateways (id, name, provider, base_url, is_active, created_at, updated_at)
VALUES 
    ('${XENDIT_GATEWAY_ID}', 'Xendit Test', 'xendit', 'https://api.xendit.co', 1, NOW(), NOW()),
    ('${MIDTRANS_GATEWAY_ID}', 'Midtrans Test', 'midtrans', 'https://api.sandbox.midtrans.com', 1, NOW(), NOW())
ON DUPLICATE KEY UPDATE 
    name = VALUES(name),
    provider = VALUES(provider),
    base_url = VALUES(base_url),
    is_active = VALUES(is_active),
    updated_at = NOW();

-- Insert test API keys if api_keys table exists
INSERT IGNORE INTO api_keys (id, name, key_hash, gateway_id, is_active, created_at, updated_at)
SELECT 
    'test-api-key-xendit', 
    'Xendit Test API Key', 
    '\$argon2id\$v=19\$m=19456,t=2,p=1\$test_salt\$test_hash',
    '${XENDIT_GATEWAY_ID}',
    1,
    NOW(),
    NOW()
FROM DUAL
WHERE EXISTS (
    SELECT 1 FROM information_schema.tables 
    WHERE table_schema = '${TEST_DB_NAME}' 
    AND table_name = 'api_keys'
);

INSERT IGNORE INTO api_keys (id, name, key_hash, gateway_id, is_active, created_at, updated_at)
SELECT 
    'test-api-key-midtrans',
    'Midtrans Test API Key',
    '\$argon2id\$v=19\$m=19456,t=2,p=1\$test_salt\$test_hash',
    '${MIDTRANS_GATEWAY_ID}',
    1,
    NOW(),
    NOW()
FROM DUAL
WHERE EXISTS (
    SELECT 1 FROM information_schema.tables 
    WHERE table_schema = '${TEST_DB_NAME}' 
    AND table_name = 'api_keys'
);
EOF

if [ $? -eq 0 ]; then
    echo -e "${GREEN}OK${NC}"
else
    echo -e "${RED}FAILED${NC}"
    exit 1
fi

echo ""
echo -e "${GREEN}✓ Test data seeding complete!${NC}"
echo ""
echo "Seeded gateways:"
echo "  - ${XENDIT_GATEWAY_ID} (Xendit)"
echo "  - ${MIDTRANS_GATEWAY_ID} (Midtrans)"
echo ""
echo "Environment variables used:"
echo "  DB_HOST: ${DB_HOST}"
echo "  DB_PORT: ${DB_PORT}"
echo "  DB_USER: ${DB_USER}"
echo "  TEST_DB_NAME: ${TEST_DB_NAME}"
echo ""

if [ -n "${XENDIT_TEST_API_KEY:-}" ]; then
    echo -e "${GREEN}✓ XENDIT_TEST_API_KEY is set${NC}"
else
    echo -e "${YELLOW}⚠ XENDIT_TEST_API_KEY not set (gateway tests may fail)${NC}"
fi

if [ -n "${MIDTRANS_SERVER_KEY:-}" ]; then
    echo -e "${GREEN}✓ MIDTRANS_SERVER_KEY is set${NC}"
else
    echo -e "${YELLOW}⚠ MIDTRANS_SERVER_KEY not set (gateway tests may fail)${NC}"
fi

echo ""
