# PayTrust API Quick Start Guide

Welcome to PayTrust! This guide shows you how to integrate the payment orchestration API into your application in 5 minutes.

---

## Prerequisites

- API key (64-character hex string provided by PayTrust)
- Payment gateway account (Xendit or Midtrans)
- Basic understanding of REST APIs and HTTP

---

## 1. Your First Invoice (Single Payment)

Create an invoice and get a payment URL in one call:

### Request
```bash
curl -X POST https://api.paytrust.local/invoices \
  -H "X-API-Key: your_api_key_here" \
  -H "Content-Type: application/json" \
  -d '{
    "gateway_id": 1,
    "currency_code": "IDR",
    "expires_at": "2025-11-10T00:00:00Z",
    "line_items": [
      {
        "product_name": "Laptop",
        "quantity": 1,
        "unit_price": 15000000,
        "tax_rate": 0.10,
        "tax_category": "VAT"
      }
    ]
  }'
```

### Response (201 Created)
```json
{
  "id": 12345,
  "external_id": null,
  "status": "draft",
  "currency_code": "IDR",
  "subtotal_amount": 15000000,
  "total_tax_amount": 1500000,
  "total_service_fee_amount": 435000,
  "total_amount": 16935000,
  "payment_url": "https://checkout.xendit.co/invoice/xyz123",
  "selected_gateway": "Xendit",
  "created_at": "2025-11-03T12:00:00Z",
  "expires_at": "2025-11-10T00:00:00Z",
  "line_items": [
    {
      "id": 1,
      "product_name": "Laptop",
      "quantity": 1,
      "unit_price": 15000000,
      "tax_rate": 0.10,
      "tax_category": "VAT",
      "country_code": null,
      "subtotal_amount": 15000000,
      "tax_amount": 1500000
    }
  ]
}
```

**Next**: Copy the `payment_url` and redirect your customer there to complete payment. PayTrust will receive a webhook notification when payment completes.

---

## 2. Installment Payments (Multiple Payments)

Create an invoice with 2 installments (customer pays in two parts):

### Request
```bash
curl -X POST https://api.paytrust.local/invoices \
  -H "X-API-Key: your_api_key_here" \
  -H "Content-Type: application/json" \
  -d '{
    "gateway_id": 2,
    "currency_code": "MYR",
    "expires_at": "2025-12-31T23:59:59Z",
    "line_items": [
      {
        "product_name": "iPhone 15",
        "quantity": 2,
        "unit_price": 999.99,
        "tax_rate": 0.06
      }
    ],
    "installments": {
      "count": 2
    }
  }'
```

### Response
```json
{
  "id": 12346,
  "status": "draft",
  "currency_code": "MYR",
  "subtotal_amount": 199998,
  "total_tax_amount": 12000,
  "total_service_fee_amount": 6000,
  "total_amount": 217998,
  "payment_url": null,
  "selected_gateway": "Midtrans",
  "created_at": "2025-11-03T12:05:00Z",
  "expires_at": "2025-12-31T23:59:59Z",
  "installment_schedule": [
    {
      "number": 1,
      "due_date": "2025-11-17T00:00:00Z",
      "amount": 99999,
      "tax": 6000,
      "service_fee": 3000,
      "total": 108999,
      "status": "unpaid",
      "paid_at": null,
      "payment_url": "https://app.midtrans.com/charge/xyz456"
    },
    {
      "number": 2,
      "due_date": "2025-12-01T00:00:00Z",
      "amount": 99999,
      "tax": 6000,
      "service_fee": 3000,
      "total": 108999,
      "status": "unpaid",
      "paid_at": null,
      "payment_url": null
    }
  ],
  "line_items": [...]
}
```

**Key Points**:
- Only the first installment has a `payment_url` (customer pays installment #1 first)
- After first payment completes via webhook, the second `payment_url` becomes available
- Customer must pay installments in order (1 → 2)

---

## 3. Check Invoice Status

Query invoice details anytime:

### Request
```bash
curl https://api.paytrust.local/invoices/12345 \
  -H "X-API-Key: your_api_key_here"
```

### Response
```json
{
  "id": 12345,
  "status": "fully_paid",
  "currency_code": "IDR",
  "total_amount": 16935000,
  ...
}
```

---

## 4. Financial Reporting

Get revenue breakdown by currency and tax rate:

### Request
```bash
curl "https://api.paytrust.local/reports/financial?start_date=2025-11-01T00:00:00Z&end_date=2025-11-30T23:59:59Z" \
  -H "X-API-Key: your_api_key_here"
```

### Response
```json
{
  "period": {
    "start_date": "2025-11-01T00:00:00Z",
    "end_date": "2025-11-30T23:59:59Z"
  },
  "by_currency": [
    {
      "currency_code": "IDR",
      "transaction_count": 42,
      "subtotal": 630000000,
      "tax": 63000000,
      "service_fees": 18270000,
      "amount": 711270000,
      "tax_breakdown": [
        {
          "tax_rate": 0.10,
          "amount": 63000000,
          "transaction_count": 42
        }
      ],
      "gateway_breakdown": [
        {
          "gateway_name": "Xendit",
          "service_fees": 10000000,
          "transaction_count": 25
        },
        {
          "gateway_name": "Midtrans",
          "service_fees": 8270000,
          "transaction_count": 17
        }
      ]
    }
  ]
}
```

---

## 5. Handle Webhooks

When payment completes, PayTrust sends a webhook notification to your system. Verify the signature and update your invoice status:

### Webhook Payload Example
```json
{
  "invoice_id": 12345,
  "status": "paid",
  "amount_paid": 16935000,
  "gateway_transaction_id": "xyz789",
  "timestamp": "2025-11-03T12:15:30Z"
}
```

---

## Common Integration Patterns

### Pattern 1: Simple Payment Flow
```
1. User clicks "Pay Now"
2. Your backend: POST /invoices → get payment_url
3. Redirect user to payment_url
4. User completes payment on gateway
5. PayTrust sends webhook to your backend
6. Your backend: GET /invoices/{id} to confirm status
7. Update order status in your system
```

### Pattern 2: Installment Payment Flow
```
1. User selects "Pay in 2 installments"
2. Your backend: POST /invoices with installments: {count: 2}
3. Display invoice.installment_schedule to user with due dates
4. Installment #1: Redirect to first payment_url
5. After Installment #1 webhook: Display Installment #2 payment_url
6. Installment #2: Redirect to second payment_url
7. After final webhook: Mark order complete
```

### Pattern 3: Add Items Mid-Payment
```
1. User has paid for initial order (Invoice #1 partially paid)
2. User requests to add items
3. Your backend: POST /invoices/123/supplementary
4. Creates new Invoice #2 linked to #1
5. Proceed with payment for Invoice #2
```

---

## Error Handling

### Rate Limiting (429)
```json
{
  "error": "rate_limit_exceeded",
  "message": "Request limit exceeded: 1000 requests per minute",
  "details": {
    "retry_after_seconds": 45
  }
}
```

**Solution**: Wait `retry_after_seconds` before retrying.

### Validation Error (400)
```json
{
  "error": "invalid_currency",
  "message": "Gateway Xendit does not support currency EUR"
}
```

**Solution**: Use supported currencies (IDR, MYR, USD) and verify gateway supports your currency.

### Gateway Unavailable (500)
```json
{
  "error": "gateway_error",
  "message": "Payment gateway temporarily unavailable"
}
```

**Solution**: Retry with idempotency key (same external_id) after waiting.

---

## Best Practices

### 1. Use External IDs for Idempotency
```bash
curl -X POST https://api.paytrust.local/invoices \
  -H "X-API-Key: your_api_key_here" \
  -d '{
    "external_id": "order_12345_v1",  # Your order ID + version
    ...
  }'
```

If the request fails and you retry with the same `external_id`, you'll get the same invoice back (safe idempotency).

### 2. Check Installment Schedule Before Displaying Payments
```bash
curl https://api.paytrust.local/invoices/12345/installments \
  -H "X-API-Key: your_api_key_here"
```

Use this to show customers their payment schedule and due dates.

### 3. Handle Overpayments
```bash
curl https://api.paytrust.local/invoices/12345/overpayment \
  -H "X-API-Key: your_api_key_here"
```

If `overpayment_amount > 0`, inform customer about excess payment. Customer initiates refund through Xendit/Midtrans dashboard.

### 4. Generate ISO 20022 Compliance Documents
```bash
curl https://api.paytrust.local/invoices/12345/payment-initiation \
  -H "X-API-Key: your_api_key_here" \
  -H "Accept: application/xml"
```

Returns pain.001 XML document for audit/compliance records.

---

## API Key Management (Admin)

### Generate New API Key
```bash
curl -X POST https://api.paytrust.local/api-keys \
  -H "X-Admin-Key: your_admin_key" \
  -d '{
    "description": "Production API key for merchant XYZ"
  }'
```

**Response**: Full API key returned (save securely, never retrievable again)

### Rotate API Key
```bash
curl -X PUT https://api.paytrust.local/api-keys/1/rotate \
  -H "X-Admin-Key: your_admin_key"
```

Old key becomes inactive, new key issued for continued service.

### Query Audit Log
```bash
curl "https://api.paytrust.local/api-keys/audit?start_date=2025-11-01T00:00:00Z&end_date=2025-11-30T23:59:59Z" \
  -H "X-Admin-Key: your_admin_key"
```

View all API key operations (created, rotated, revoked, used) for security auditing.

---

## Testing with Different Currencies

### IDR (Indonesian Rupiah - No Decimals)
```json
"unit_price": 1000000   # 1 million rupiah
"total_amount": 1100000 # 1.1 million rupiah
```

### MYR/USD (2 Decimal Places)
```json
"unit_price": 99.99      # 99.99 MYR/USD
"total_amount": 109.98   # 109.98 MYR/USD
```

---

## Next Steps

- **Read the full API specification**: OpenAPI spec at `/contracts/openapi.yaml`
- **Implement webhook handling**: Subscribe to payment status updates
- **Set up financial reporting**: Weekly/monthly revenue reports
- **Test installment flows**: Validate multi-payment scenarios
- **Implement error recovery**: Handle rate limits and gateway failures gracefully

---

## Support

- API issues: Check error response message and HTTP status code
- Gateway-specific questions: Refer to Xendit/Midtrans documentation
- Feature requests: Contact PayTrust support

---

**Happy integrating!** 🚀
