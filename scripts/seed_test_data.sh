#!/bin/bash

# Seed test data for PayTrust development
# This script adds a test payment gateway and API key to the database

echo "🌱 Seeding PayTrust test data..."

# Database connection from .env
DB_URL="${DATABASE_URL:-mysql://root:password@localhost:3306/paytrust_dev}"

# Extract database name from URL
DB_NAME=$(echo $DB_URL | sed -n 's|.*//.*/.*/\([^?]*\).*|\1|p')
if [ -z "$DB_NAME" ]; then
    DB_NAME="paytrust_dev"
fi

# Extract host, user, password
DB_HOST=$(echo $DB_URL | sed -n 's|.*//.*@\([^:]*\):.*|\1|p')
DB_PORT=$(echo $DB_URL | sed -n 's|.*://.*@.*:\([0-9]*\)/.*|\1|p')
DB_USER=$(echo $DB_URL | sed -n 's|.*://\([^:]*\):.*|\1|p')
DB_PASS=$(echo $DB_URL | sed -n 's|.*://[^:]*:\([^@]*\)@.*|\1|p')

if [ -z "$DB_HOST" ]; then
    DB_HOST="localhost"
fi
if [ -z "$DB_PORT" ]; then
    DB_PORT="3306"
fi
if [ -z "$DB_USER" ]; then
    DB_USER="root"
fi
if [ -z "$DB_PASS" ]; then
    DB_PASS="password"
fi

echo "📊 Database: $DB_NAME on $DB_HOST:$DB_PORT"

# Create SQL for test data
SQL="
-- Insert test payment gateways
INSERT INTO payment_gateways (id, name, supported_currencies, fee_percentage, fee_fixed, api_key_encrypted, webhook_secret, webhook_url, is_active, environment, created_at, updated_at)
VALUES 
  ('gateway-xendit-idr', 'Xendit IDR', '[\"IDR\"]', 0.0290, 2000.0000, X'746573745f656e637279707465645f6b6579', 'xendit_test_webhook_secret', 'https://api.xendit.co/callback', true, 'sandbox', NOW(), NOW()),
  ('gateway-xendit-myr', 'Xendit MYR', '[\"MYR\"]', 0.0290, 1.5000, X'746573745f656e637279707465645f6b6579', 'xendit_test_webhook_secret', 'https://api.xendit.co/callback', true, 'sandbox', NOW(), NOW()),
  ('gateway-midtrans-idr', 'Midtrans IDR', '[\"IDR\"]', 0.0280, 1500.0000, X'746573745f656e637279707465645f6b6579', 'midtrans_test_webhook_secret', 'https://api.sandbox.midtrans.com/callback', true, 'sandbox', NOW(), NOW())
ON DUPLICATE KEY UPDATE 
  updated_at = NOW();

-- Insert test API key (hashed version of 'test_api_key_12345')
-- In production, this should be properly hashed with argon2
INSERT INTO api_keys (id, merchant_id, api_key_hash, rate_limit, is_active, created_at, updated_at)
VALUES 
  ('apikey-test-001', 'merchant-test-001', 'test_hash_placeholder_for_development', 1000, true, NOW(), NOW())
ON DUPLICATE KEY UPDATE 
  updated_at = NOW();

SELECT 'Seed data inserted successfully!' AS status;
"

# Execute SQL
echo "$SQL" | mysql -h"$DB_HOST" -P"$DB_PORT" -u"$DB_USER" -p"$DB_PASS" "$DB_NAME" 2>&1

if [ $? -eq 0 ]; then
    echo "✅ Test data seeded successfully!"
    echo ""
    echo "📝 Test Payment Gateways:"
    echo "  - gateway-xendit-idr (IDR, 2.9% + Rp2000)"
    echo "  - gateway-xendit-myr (MYR, 2.9% + RM1.50)"
    echo "  - gateway-midtrans-idr (IDR, 2.8% + Rp1500)"
    echo ""
    echo "🔑 Test API Key:"
    echo "  - Merchant ID: merchant-test-001"
    echo "  - API Key ID: apikey-test-001"
    echo ""
    echo "⚠️  Note: For actual API requests, you need to bypass auth middleware"
    echo "    or implement proper API key hashing in production"
else
    echo "❌ Failed to seed test data"
    exit 1
fi
