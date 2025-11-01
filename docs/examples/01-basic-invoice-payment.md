# User Story 1: Basic Invoice Creation and Payment

This example demonstrates creating a simple invoice with line items and processing a payment.

## Prerequisites

- API Key configured in environment
- Payment gateway (Xendit or Midtrans) registered
- MySQL database running

## Step 1: Create a Single Payment Invoice

### Request

```bash
curl -X POST https://api.paytrust.example.com/v1/invoices \
  -H "X-API-Key: your_api_key_here" \
  -H "Content-Type: application/json" \
  -d '{
    "currency": "IDR",
    "gateway_id": "gateway-xendit-123",
    "line_items": [
      {
        "product_name": "Premium Subscription",
        "quantity": 1,
        "unit_price": "1000000"
      },
      {
        "product_name": "Setup Fee",
        "quantity": 1,
        "unit_price": "250000"
      }
    ],
    "expires_at": "2025-11-03T10:00:00Z"
  }'
```

### Response (201 Created)

```json
{
  "invoice_id": "inv_7f8a9b0c1d2e3f4a",
  "currency": "IDR",
  "status": "pending",
  "line_items": [
    {
      "line_item_id": "li_1a2b3c4d",
      "product_name": "Premium Subscription",
      "quantity": 1,
      "unit_price": "1000000",
      "subtotal": "1000000"
    },
    {
      "line_item_id": "li_5e6f7g8h",
      "product_name": "Setup Fee",
      "quantity": 1,
      "unit_price": "250000",
      "subtotal": "250000"
    }
  ],
  "subtotal": "1250000",
  "tax_total": "0",
  "service_fee": "0",
  "total_amount": "1250000",
  "payment_urls": [
    {
      "installment_number": null,
      "url": "https://checkout.xendit.co/web/inv_7f8a9b0c1d2e3f4a"
    }
  ],
  "expires_at": "2025-11-03T10:00:00Z",
  "created_at": "2025-11-02T08:00:00Z"
}
```

## Step 2: Retrieve Invoice Details

### Request

```bash
curl -X GET https://api.paytrust.example.com/v1/invoices/inv_7f8a9b0c1d2e3f4a \
  -H "X-API-Key: your_api_key_here"
```

### Response (200 OK)

```json
{
  "invoice_id": "inv_7f8a9b0c1d2e3f4a",
  "currency": "IDR",
  "status": "pending",
  "gateway_id": "gateway-xendit-123",
  "subtotal": "1250000",
  "tax_total": "0",
  "service_fee": "0",
  "total_amount": "1250000",
  "amount_paid": "0",
  "line_items": [...],
  "payment_urls": [...],
  "expires_at": "2025-11-03T10:00:00Z",
  "created_at": "2025-11-02T08:00:00Z",
  "updated_at": "2025-11-02T08:00:00Z"
}
```

## Step 3: Payment Gateway Webhook (Automatic)

When the customer completes payment at the gateway, PayTrust receives a webhook:

### Webhook Payload (from Xendit)

```json
{
  "id": "ext_xendit_12345",
  "external_id": "inv_7f8a9b0c1d2e3f4a",
  "status": "PAID",
  "amount": 1250000,
  "paid_at": "2025-11-02T09:15:00Z",
  "payment_method": "BANK_TRANSFER"
}
```

### PayTrust Processing

1. Validates webhook signature
2. Records payment transaction
3. Updates invoice status to `fully_paid`
4. Logs transaction with idempotency key

## Step 4: Check Payment Transactions

### Request

```bash
curl -X GET https://api.paytrust.example.com/v1/invoices/inv_7f8a9b0c1d2e3f4a/transactions \
  -H "X-API-Key: your_api_key_here"
```

### Response (200 OK)

```json
{
  "invoice_id": "inv_7f8a9b0c1d2e3f4a",
  "transactions": [
    {
      "transaction_id": "txn_abc123def456",
      "external_transaction_id": "ext_xendit_12345",
      "amount": "1250000",
      "status": "success",
      "payment_method": "bank_transfer",
      "paid_at": "2025-11-02T09:15:00Z",
      "installment_number": null,
      "created_at": "2025-11-02T09:15:05Z"
    }
  ]
}
```

## Step 5: List All Invoices

### Request

```bash
curl -X GET "https://api.paytrust.example.com/v1/invoices?status=fully_paid&page=1&page_size=10" \
  -H "X-API-Key: your_api_key_here"
```

### Response (200 OK)

```json
{
  "data": [
    {
      "invoice_id": "inv_7f8a9b0c1d2e3f4a",
      "currency": "IDR",
      "status": "fully_paid",
      "total_amount": "1250000",
      "amount_paid": "1250000",
      "created_at": "2025-11-02T08:00:00Z"
    }
  ],
  "pagination": {
    "page": 1,
    "page_size": 10,
    "total_items": 1,
    "total_pages": 1
  }
}
```

## Error Handling Examples

### Invalid Currency

```bash
curl -X POST https://api.paytrust.example.com/v1/invoices \
  -H "X-API-Key: your_api_key_here" \
  -H "Content-Type: application/json" \
  -d '{
    "currency": "JPY",
    "gateway_id": "gateway-xendit-123",
    "line_items": [...]
  }'
```

**Response (400 Bad Request):**

```json
{
  "error": {
    "code": "INVALID_CURRENCY",
    "message": "Currency JPY is not supported. Allowed: IDR, MYR, USD",
    "details": {
      "field": "currency",
      "provided": "JPY",
      "allowed": ["IDR", "MYR", "USD"]
    }
  }
}
```

### Gateway Currency Mismatch

```bash
curl -X POST https://api.paytrust.example.com/v1/invoices \
  -H "X-API-Key: your_api_key_here" \
  -H "Content-Type: application/json" \
  -d '{
    "currency": "MYR",
    "gateway_id": "gateway-xendit-idr-only",
    "line_items": [...]
  }'
```

**Response (400 Bad Request):**

```json
{
  "error": {
    "code": "GATEWAY_CURRENCY_MISMATCH",
    "message": "Gateway gateway-xendit-idr-only does not support currency MYR",
    "details": {
      "gateway_id": "gateway-xendit-idr-only",
      "requested_currency": "MYR",
      "supported_currencies": ["IDR"]
    }
  }
}
```

### Expired Invoice

When retrieving an invoice past its expiration time:

**Response (200 OK with expired status):**

```json
{
  "invoice_id": "inv_7f8a9b0c1d2e3f4a",
  "status": "expired",
  "expires_at": "2025-11-03T10:00:00Z",
  "current_time": "2025-11-03T11:00:00Z",
  ...
}
```

## Testing with cURL

Save your API key as an environment variable:

```bash
export PAYTRUST_API_KEY="your_api_key_here"
```

Then use it in requests:

```bash
curl -X POST https://api.paytrust.example.com/v1/invoices \
  -H "X-API-Key: $PAYTRUST_API_KEY" \
  -H "Content-Type: application/json" \
  -d @invoice_payload.json
```

## Next Steps

- See [User Story 2](./02-taxes-and-service-fees.md) for adding taxes and service fees
- See [User Story 3](./03-installment-payments.md) for installment payment configuration
- See [User Story 4](./04-multi-currency-support.md) for multi-currency handling
