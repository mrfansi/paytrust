#!/bin/bash
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}=== PayTrust Test Suite with Docker ===${NC}\n"

# Check if docker-compose is available
if ! command -v docker-compose &> /dev/null && ! command -v docker &> /dev/null; then
    echo -e "${RED}Error: docker-compose or docker not found${NC}"
    echo "Please install Docker Desktop or Docker Engine with Compose plugin"
    exit 1
fi

# Determine compose command
if command -v docker-compose &> /dev/null; then
    COMPOSE_CMD="docker-compose"
else
    COMPOSE_CMD="docker compose"
fi

# Function to cleanup on exit
cleanup() {
    echo -e "\n${YELLOW}Cleaning up...${NC}"
    $COMPOSE_CMD -f docker-compose.test.yml down
}

trap cleanup EXIT

# Start MySQL test container
echo -e "${GREEN}Starting MySQL test container...${NC}"
$COMPOSE_CMD -f docker-compose.test.yml up -d

# Wait for MySQL to be ready
echo -e "${YELLOW}Waiting for MySQL to be ready...${NC}"
for i in {1..30}; do
    if docker exec paytrust-mysql-test mysqladmin ping -h localhost -u root -ptest_password --silent 2>/dev/null; then
        echo -e "${GREEN}MySQL is ready!${NC}"
        break
    fi
    if [ $i -eq 30 ]; then
        echo -e "${RED}Error: MySQL failed to start within 30 seconds${NC}"
        exit 1
    fi
    echo "Waiting... ($i/30)"
    sleep 1
done

# Set test environment variables
export TEST_DATABASE_URL="mysql://root:test_password@127.0.0.1:3307/paytrust_test"
export DATABASE_URL="mysql://root:test_password@127.0.0.1:3307/paytrust_test"
export SERVER_HOST="127.0.0.1"
export SERVER_PORT="8080"
export RUST_LOG="${RUST_LOG:-info}"
export TEST_CLEANUP_ENABLED="true"

# Check for required environment variables
if [ -z "$XENDIT_TEST_API_KEY" ]; then
    echo -e "${YELLOW}Warning: XENDIT_TEST_API_KEY not set. Gateway tests may fail.${NC}"
fi

if [ -z "$MIDTRANS_SERVER_KEY" ]; then
    echo -e "${YELLOW}Warning: MIDTRANS_SERVER_KEY not set. Gateway tests may fail.${NC}"
fi

# Run migrations
echo -e "\n${GREEN}Running database migrations...${NC}"
if ! cargo install sqlx-cli --no-default-features --features mysql 2>/dev/null; then
    echo -e "${YELLOW}sqlx-cli already installed or installation failed${NC}"
fi
sqlx migrate run

# Seed test data
echo -e "\n${GREEN}Seeding test data...${NC}"
docker exec -i paytrust-mysql-test mysql -u root -ptest_password paytrust_test <<EOF
INSERT INTO payment_gateways (id, name, provider, base_url, is_active, created_at, updated_at)
VALUES 
  ('xendit-test-001', 'Xendit Test', 'xendit', 'https://api.xendit.co', 1, NOW(), NOW()),
  ('midtrans-test-001', 'Midtrans Test', 'midtrans', 'https://api.sandbox.midtrans.com', 1, NOW(), NOW())
ON DUPLICATE KEY UPDATE updated_at = NOW();
EOF

# Run tests based on argument
TEST_TYPE="${1:-all}"

case "$TEST_TYPE" in
    unit)
        echo -e "\n${GREEN}Running unit tests...${NC}"
        cargo test --lib --verbose
        ;;
    integration)
        echo -e "\n${GREEN}Running integration tests...${NC}"
        cargo test --test '*' --verbose
        ;;
    contract)
        echo -e "\n${GREEN}Running contract tests...${NC}"
        cargo test --test '*api_test' --verbose
        ;;
    all)
        echo -e "\n${GREEN}Running all tests...${NC}"
        echo -e "${YELLOW}1. Unit tests${NC}"
        cargo test --lib --verbose
        
        echo -e "\n${YELLOW}2. Integration tests${NC}"
        cargo test --test '*' --verbose
        
        echo -e "\n${YELLOW}3. Contract tests${NC}"
        cargo test --test '*api_test' --verbose
        ;;
    *)
        echo -e "${RED}Error: Invalid test type '$TEST_TYPE'${NC}"
        echo "Usage: $0 [unit|integration|contract|all]"
        exit 1
        ;;
esac

echo -e "\n${GREEN}=== Tests completed successfully! ===${NC}"
