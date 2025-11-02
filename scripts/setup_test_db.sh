#!/usr/bin/env bash
# Setup test database for PayTrust integration tests
# This script creates the test database and runs migrations

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
TEST_DB_NAME="${TEST_DB_NAME:-paytrust_test}"
DB_HOST="${DB_HOST:-localhost}"
DB_PORT="${DB_PORT:-3306}"
DB_USER="${DB_USER:-root}"
DB_PASS="${DB_PASS:-password}"
DATABASE_URL="mysql://${DB_USER}:${DB_PASS}@${DB_HOST}:${DB_PORT}/${TEST_DB_NAME}"

echo -e "${GREEN}PayTrust Test Database Setup${NC}"
echo "=============================="
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

# Check if sqlx-cli is installed
echo -n "Checking sqlx-cli installation... "
if ! command -v sqlx &> /dev/null; then
    echo -e "${YELLOW}NOT FOUND${NC}"
    echo ""
    echo -e "${YELLOW}sqlx-cli is required for database migrations${NC}"
    echo "Install with: cargo install sqlx-cli --no-default-features --features mysql"
    echo ""
    read -p "Install sqlx-cli now? (y/n) " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        cargo install sqlx-cli --no-default-features --features mysql
    else
        echo -e "${RED}Aborting: sqlx-cli is required${NC}"
        exit 1
    fi
fi
echo -e "${GREEN}OK${NC}"

# Drop existing test database if it exists
echo -n "Dropping existing test database (if exists)... "
mysql -h"${DB_HOST}" -P"${DB_PORT}" -u"${DB_USER}" -p"${DB_PASS}" -e "DROP DATABASE IF EXISTS ${TEST_DB_NAME}" 2>/dev/null
echo -e "${GREEN}OK${NC}"

# Create test database
echo -n "Creating test database ${TEST_DB_NAME}... "
if mysql -h"${DB_HOST}" -P"${DB_PORT}" -u"${DB_USER}" -p"${DB_PASS}" -e "CREATE DATABASE ${TEST_DB_NAME} CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci" 2>/dev/null; then
    echo -e "${GREEN}OK${NC}"
else
    echo -e "${RED}FAILED${NC}"
    echo -e "${RED}Error: Could not create database ${TEST_DB_NAME}${NC}"
    exit 1
fi

# Run migrations
echo ""
echo "Running migrations..."
export DATABASE_URL="${DATABASE_URL}"
if sqlx migrate run; then
    echo -e "${GREEN}Migrations completed successfully${NC}"
else
    echo -e "${RED}Migration failed${NC}"
    exit 1
fi

# Seed initial test data (gateway configuration)
echo ""
echo -n "Seeding test gateway data... "
mysql -h"${DB_HOST}" -P"${DB_PORT}" -u"${DB_USER}" -p"${DB_PASS}" "${TEST_DB_NAME}" << EOF
-- Insert test payment gateways
INSERT INTO payment_gateways (id, name, gateway_type, api_key_id, is_active, created_at, updated_at)
VALUES 
    ('test-gateway-001', 'Test Xendit Gateway', 'xendit', 'test-xendit-key', true, NOW(), NOW()),
    ('test-gateway-002', 'Test Midtrans Gateway', 'midtrans', 'test-midtrans-key', true, NOW(), NOW())
ON DUPLICATE KEY UPDATE updated_at = NOW();

-- Insert test API keys (hashed with argon2)
INSERT INTO api_keys (id, name, key_hash, gateway_id, is_active, created_at, updated_at)
VALUES
    ('test-api-key-001', 'Test API Key', '\$argon2id\$v=19\$m=19456,t=2,p=1\$test_salt_here\$test_hash_here', 'test-gateway-001', true, NOW(), NOW())
ON DUPLICATE KEY UPDATE updated_at = NOW();
EOF

if [ $? -eq 0 ]; then
    echo -e "${GREEN}OK${NC}"
else
    echo -e "${YELLOW}Warning: Could not seed test data (tables may not exist yet)${NC}"
fi

echo ""
echo -e "${GREEN}✓ Test database setup complete!${NC}"
echo ""
echo "Database: ${TEST_DB_NAME}"
echo "Connection: ${DATABASE_URL}"
echo ""
echo "Next steps:"
echo "1. Copy config/.env.test.example to .env.test"
echo "2. Update .env.test with your test credentials"
echo "3. Run tests: cargo test"
echo ""
