# PayTrust API Usage Examples

This directory contains comprehensive examples demonstrating how to use the PayTrust Payment Orchestration API for each user story.

## 📚 Available Examples

### [01 - Basic Invoice Creation and Payment](./01-basic-invoice-payment.md)

**User Story 1 (P1 - MVP)**

Learn how to:

- Create simple invoices with line items
- Process single payments through gateways
- Handle webhook notifications
- Query payment transactions
- Manage invoice lifecycle

**Perfect for**: Getting started with PayTrust, understanding the basic payment flow

---

### [02 - Additional Charges Management](./02-taxes-and-service-fees.md)

**User Story 2 (P2)**

Learn how to:

- Apply per-line-item tax calculations
- Configure different tax rates per product
- Calculate gateway-specific service fees
- Generate financial reports with tax breakdowns
- Understand tax locking rules

**Perfect for**: E-commerce platforms, businesses requiring tax compliance

---

### [03 - Installment Payment Configuration](./03-installment-payments.md)

**User Story 3 (P3)**

Learn how to:

- Create invoices with equal installment splits
- Customize installment amounts
- Handle sequential payment enforcement
- Adjust unpaid installment amounts
- Manage overpayment auto-application
- Create supplementary invoices

**Perfect for**: Subscription services, high-value purchases, flexible payment plans

---

### [04 - Multi-Currency Payment Isolation](./04-multi-currency-support.md)

**User Story 4 (P4)**

Learn how to:

- Process payments in IDR, MYR, USD
- Handle currency-specific decimal precision
- Prevent currency mixing errors
- Generate multi-currency financial reports
- Validate gateway currency support
- Apply currency-specific rounding

**Perfect for**: International businesses, multi-region operations

---

## 🚀 Quick Start

### Prerequisites

1. **PayTrust API running locally**:

   ```bash
   cd /path/to/paytrust
   cargo run
   ```

2. **API Key**: Generate via admin endpoint or use test key

   ```bash
   export API_KEY="pk_dev_your_key_here"
   ```

3. **Payment Gateway**: Configure Xendit or Midtrans credentials in `.env`

### Making Your First Request

```bash
# Create a simple invoice
curl -X POST http://127.0.0.1:8080/v1/invoices \
  -H "X-API-Key: $API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "currency": "IDR",
    "gateway_id": "gateway-xendit-123",
    "line_items": [
      {
        "product_name": "Test Product",
        "quantity": 1,
        "unit_price": "100000"
      }
    ]
  }' | jq
```

---

## 📖 Learning Path

### For New Developers

1. **Start with Example 01**: Understand basic invoice creation and payment flow
2. **Add Example 02**: Learn about taxes and service fees
3. **Progress to Example 03**: Master installment payments
4. **Complete with Example 04**: Handle multi-currency scenarios

### For Integration Teams

1. **Review all examples**: Understand capabilities
2. **Identify your use case**: Match to appropriate user story
3. **Adapt examples**: Customize for your business logic
4. **Test thoroughly**: Use property-based tests for edge cases

---

## 🔍 Example Structure

Each example follows this consistent structure:

1. **Prerequisites**: What you need before starting
2. **Step-by-Step Guide**: Detailed walkthrough with requests/responses
3. **Calculation Breakdown**: Financial math explained
4. **Error Handling**: Common errors and how to handle them
5. **Best Practices**: Recommendations for production use
6. **Testing Scenarios**: Edge cases to consider

---

## 🛠️ Tools & Utilities

### cURL Examples

All examples use cURL for simplicity. Save your API key:

```bash
export API_KEY="your_api_key_here"
```

Then copy-paste examples directly.

### JSON Formatting

Install `jq` for readable JSON output:

```bash
# macOS
brew install jq

# Ubuntu
sudo apt install jq

# Usage
curl ... | jq
```

### Postman Collection

Import the OpenAPI spec into Postman:

1. Open Postman
2. Import → Link → `http://127.0.0.1:8080/api/docs/openapi.json`
3. Generate collection from OpenAPI spec

---

## 📊 API Documentation

### Interactive Documentation

Visit the Swagger UI for interactive API testing:

```
http://127.0.0.1:8080/api/docs
```

### OpenAPI Specification

- **YAML format**: `http://127.0.0.1:8080/api/docs/openapi.yaml`
- **JSON format**: `http://127.0.0.1:8080/api/docs/openapi.json`

---

## 🧪 Testing Examples

### Unit Tests

Examples are backed by comprehensive unit tests:

```bash
# Run all unit tests
cargo test --lib

# Run specific test
cargo test test_invoice_calculation
```

### Integration Tests

Examples demonstrate flows tested in integration tests:

```bash
# Run all integration tests
cargo test --test '*'

# Run specific integration test
cargo test --test payment_flow_test
```

### Contract Tests

Examples conform to OpenAPI contract:

```bash
# Run contract tests
cargo test --test invoice_api_test
```

---

## 🔐 Security Notes

### API Key Management

- **Development**: Use `pk_dev_` prefixed keys
- **Production**: Use `pk_live_` prefixed keys
- **Never commit**: Add `.env` to `.gitignore`
- **Rotate regularly**: Change keys every 90 days

### Webhook Signature Verification

All webhook examples include signature verification:

```bash
# Xendit webhook signature
X-Callback-Token: your_webhook_secret

# Midtrans webhook signature
X-Signature: hmac_sha512_hash
```

---

## 💡 Tips & Tricks

### 1. Idempotency

All payment operations support idempotency keys:

```bash
curl -X POST ... \
  -H "Idempotency-Key: unique-request-id-123"
```

### 2. Pagination

List endpoints support pagination:

```bash
curl "http://127.0.0.1:8080/v1/invoices?page=1&page_size=20"
```

### 3. Filtering

Filter results by status, currency, date:

```bash
curl "http://127.0.0.1:8080/v1/invoices?status=paid&currency=IDR"
```

### 4. Rate Limiting

Default: 1000 requests/minute per API key. Headers show limits:

```
X-RateLimit-Limit: 1000
X-RateLimit-Remaining: 985
X-RateLimit-Reset: 1698876000
```

---

## 🆘 Troubleshooting

### Common Issues

**401 Unauthorized**

- Check API key is correct
- Verify `X-API-Key` header is set
- Ensure key hasn't expired

**400 Bad Request**

- Validate JSON payload format
- Check required fields are present
- Verify data types match schema

**429 Too Many Requests**

- Wait for rate limit reset
- Use exponential backoff
- Consider increasing rate limit

**500 Internal Server Error**

- Check server logs
- Verify database connection
- Ensure migrations are applied

### Getting Help

1. **Check logs**: `RUST_LOG=debug cargo run`
2. **Review spec**: `specs/001-payment-orchestration-api/spec.md`
3. **Read data model**: `specs/001-payment-orchestration-api/data-model.md`
4. **Test examples**: Run integration tests

---

## 📚 Additional Resources

- **Developer Quickstart**: [../quickstart.md](../quickstart.md)
- **Deployment Guide**: [../deployment.md](../deployment.md)
- **OpenAPI Spec**: [../../specs/001-payment-orchestration-api/contracts/openapi.yaml](../../specs/001-payment-orchestration-api/contracts/openapi.yaml)
- **Data Model**: [../../specs/001-payment-orchestration-api/data-model.md](../../specs/001-payment-orchestration-api/data-model.md)

---

## 🤝 Contributing

Found an issue or want to add an example?

1. Check existing examples for patterns
2. Follow the same structure and format
3. Include request/response examples
4. Add error handling scenarios
5. Document edge cases
6. Test all code snippets

---

## 📄 License

These examples are part of the PayTrust Payment Orchestration Platform documentation.

---

**Happy Integrating!** 🎉

For questions or support, review the specification documents in `specs/001-payment-orchestration-api/`.
