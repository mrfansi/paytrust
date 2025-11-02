-- Seed test data for PayTrust development
-- Run this with: mysql -u root -p paytrust_dev < scripts/seed_test_data.sql

-- Insert test payment gateways
INSERT INTO payment_gateways (id, name, supported_currencies, fee_percentage, fee_fixed, api_key_encrypted, webhook_secret, webhook_url, is_active, environment, created_at, updated_at)
VALUES 
  ('gateway-xendit-idr', 'Xendit IDR', '[\"IDR\"]', 0.0290, 2000.0000, X'746573745f656e637279707465645f6b6579', 'xendit_test_webhook_secret', 'https://api.xendit.co/callback', true, 'sandbox', NOW(), NOW()),
  ('gateway-xendit-myr', 'Xendit MYR', '[\"MYR\"]', 0.0290, 1.5000, X'746573745f656e637279707465645f6b6579', 'xendit_test_webhook_secret', 'https://api.xendit.co/callback', true, 'sandbox', NOW(), NOW()),
  ('gateway-midtrans-idr', 'Midtrans IDR', '[\"IDR\"]', 0.0280, 1500.0000, X'746573745f656e637279707465645f6b6579', 'midtrans_test_webhook_secret', 'https://api.sandbox.midtrans.com/callback', true, 'sandbox', NOW(), NOW())
ON DUPLICATE KEY UPDATE 
  updated_at = NOW();

-- Insert test API key for development
-- Note: In production, use proper argon2 hashing
INSERT INTO api_keys (id, merchant_id, api_key_hash, rate_limit, is_active, created_at, updated_at)
VALUES 
  ('apikey-test-001', 'merchant-test-001', 'dev_test_key_bypass_auth_for_testing', 1000, true, NOW(), NOW())
ON DUPLICATE KEY UPDATE 
  updated_at = NOW();

SELECT 'Test data seeded successfully!' AS status;
