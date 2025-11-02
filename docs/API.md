# PayTrust API Documentation

**Version**: 0.1.0  
**Base URL**: `http://127.0.0.1:8080` (development)  
**Last Updated**: November 2, 2025

## Table of Contents

1. [Authentication](#authentication)
2. [Health & Status](#health--status)
3. [Invoices](#invoices)
4. [Installments](#installments)
5. [Reports](#reports)
6. [Gateways](#gateways)
7. [Taxes](#taxes)
8. [Error Handling](#error-handling)

---

## Authentication

All API endpoints (except health checks) require an API key passed in the header.

### Headers

```
X-API-Key: your_api_key_here
Content-Type: application/json
```

### Rate Limiting

- Default: 1000 requests per minute per API key
- Rate limit info returned in headers:
  - `X-RateLimit-Limit`
  - `X-RateLimit-Remaining`
  - `X-RateLimit-Reset`

---

## Health & Status

### GET /health

Health check endpoint (no auth required).

**Response**:

```json
{
  "status": "healthy",
  "database": "connected",
  "timestamp": "2025-11-02T10:00:00Z"
}
```

### GET /ready

Readiness check for deployments (no auth required).

**Response**:

```json
{
  "ready": true,
  "checks": {
    "database": "ok",
    "migrations": "ok"
  }
}
```

### GET /metrics

Prometheus-style metrics for monitoring (no auth required).

**Response**: Plain text metrics format

---

## Invoices

### POST /v1/invoices

Create a new invoice with line items.

**Request Body**:

```json
{
  "external_id": "ORDER-12345",
  "gateway_id": "gateway-xendit-idr",
  "currency": "IDR",
  "line_items": [
    {
      "description": "Premium Subscription",
      "quantity": 1,
      "unit_price": "1000000",
      "tax_rate": "0.10",
      "tax_category": "VAT"
    },
    {
      "description": "Setup Fee",
      "quantity": 1,
      "unit_price": "500000"
    }
  ],
  "installment_config": {
    "count": 2,
    "amounts": ["600000", "900000"]
  }
}
```

**Field Descriptions**:

- `external_id` (string, required): Your unique reference ID
- `gateway_id` (string, required): Payment gateway to use
- `currency` (enum, required): `"IDR"`, `"MYR"`, or `"USD"`
- `line_items` (array, required): At least one line item
  - `description` (string): Product/service description
  - `quantity` (number): Quantity purchased
  - `unit_price` (string/decimal): Price per unit
  - `tax_rate` (string/decimal, optional): Tax rate (0.10 = 10%)
  - `tax_category` (string, optional): Tax category label
- `installment_config` (object, optional): Installment payment setup
  - `count` (number): Number of installments (2-12)
  - `amounts` (array, optional): Custom amounts for each installment

**Response** (201 Created):

```json
{
  "id": "inv_7f8a9b0c1d2e3f4a",
  "external_id": "ORDER-12345",
  "gateway_id": "gateway-xendit-idr",
  "currency": "IDR",
  "subtotal": "1500000",
  "tax_total": "100000",
  "service_fee": "45350",
  "total": "1645350",
  "status": "pending",
  "line_items": [
    {
      "id": "li_1a2b3c4d",
      "description": "Premium Subscription",
      "quantity": 1,
      "unit_price": "1000000",
      "currency": "IDR",
      "subtotal": "1000000",
      "tax_rate": "0.10",
      "tax_category": "VAT",
      "tax_amount": "100000"
    },
    {
      "id": "li_5e6f7g8h",
      "description": "Setup Fee",
      "quantity": 1,
      "unit_price": "500000",
      "currency": "IDR",
      "subtotal": "500000"
    }
  ],
  "expires_at": "2025-11-03T10:00:00Z",
  "created_at": "2025-11-02T10:00:00Z",
  "updated_at": "2025-11-02T10:00:00Z"
}
```

**Calculation Logic**:

1. `subtotal` = Sum of all line item subtotals (quantity × unit_price)
2. `tax_total` = Sum of all line item taxes (subtotal × tax_rate)
3. `service_fee` = (subtotal × gateway_fee_percentage) + gateway_fee_fixed
4. `total` = subtotal + tax_total + service_fee

**Error Responses**:

- `400 Bad Request`: Invalid input data
- `404 Not Found`: Gateway not found
- `409 Conflict`: Duplicate external_id
- `422 Unprocessable Entity`: Validation errors

### GET /v1/invoices/{id}

Retrieve a specific invoice by ID.

**Path Parameters**:

- `id` (string): Invoice ID

**Response** (200 OK):

```json
{
  "id": "inv_7f8a9b0c1d2e3f4a",
  "external_id": "ORDER-12345",
  "gateway_id": "gateway-xendit-idr",
  "currency": "IDR",
  "subtotal": "1500000",
  "tax_total": "100000",
  "service_fee": "45350",
  "total": "1645350",
  "status": "pending",
  "line_items": [...],
  "expires_at": "2025-11-03T10:00:00Z",
  "created_at": "2025-11-02T10:00:00Z",
  "updated_at": "2025-11-02T10:00:00Z"
}
```

**Error Responses**:

- `404 Not Found`: Invoice not found

### GET /v1/invoices

List invoices with pagination.

**Query Parameters**:

- `limit` (number, optional): Results per page (default: 20, max: 100)
- `offset` (number, optional): Results to skip (default: 0)

**Example**:

```bash
curl -X GET "http://127.0.0.1:8080/v1/invoices?limit=10&offset=0" \
  -H "X-API-Key: your_api_key"
```

**Response** (200 OK):

```json
[
  {
    "id": "inv_7f8a9b0c1d2e3f4a",
    "external_id": "ORDER-12345",
    "status": "pending",
    "total": "1645350",
    "currency": "IDR",
    ...
  },
  ...
]
```

### POST /v1/invoices/{id}/supplementary

Create a supplementary invoice for excess overpayment.

**Path Parameters**:

- `id` (string): Original invoice ID

**Request Body**:

```json
{
  "excess_amount": "50000",
  "description": "Excess payment credit"
}
```

**Response** (200 OK):

```json
{
  "id": "inv_supp_9i0j1k2l",
  "external_id": "ORDER-12345-SUPP",
  "status": "paid",
  "total": "50000",
  ...
}
```

---

## Installments

### GET /v1/installments/{invoice_id}

Get all installments for an invoice.

**Path Parameters**:

- `invoice_id` (string): Invoice ID

**Response** (200 OK):

```json
{
  "invoice_id": "inv_7f8a9b0c1d2e3f4a",
  "installments": [
    {
      "id": "inst_1a2b3c4d",
      "installment_number": 1,
      "amount": "600000",
      "tax_amount": "40000",
      "service_fee_amount": "18140",
      "due_date": "2025-11-15",
      "status": "unpaid",
      "payment_url": "https://checkout.xendit.co/web/inst_1a2b3c4d"
    },
    {
      "id": "inst_5e6f7g8h",
      "installment_number": 2,
      "amount": "900000",
      "tax_amount": "60000",
      "service_fee_amount": "27210",
      "due_date": "2025-12-15",
      "status": "unpaid",
      "payment_url": null
    }
  ]
}
```

### POST /v1/installments/{invoice_id}/adjust

Adjust unpaid installment amounts.

**Path Parameters**:

- `invoice_id` (string): Invoice ID

**Request Body**:

```json
{
  "adjustments": [
    {
      "installment_number": 2,
      "new_amount": "1045350"
    }
  ]
}
```

**Response** (200 OK):

```json
{
  "invoice_id": "inv_7f8a9b0c1d2e3f4a",
  "updated_installments": [...]
}
```

---

## Reports

### GET /v1/reports/financial

Generate financial report with service fees and taxes breakdown.

**Query Parameters**:

- `start_date` (string, required): Start date (YYYY-MM-DD)
- `end_date` (string, required): End date (YYYY-MM-DD)
- `currency` (string, optional): Filter by currency (IDR, MYR, USD)

**Example**:

```bash
curl -X GET "http://127.0.0.1:8080/v1/reports/financial?start_date=2025-01-01&end_date=2025-12-31&currency=IDR" \
  -H "X-API-Key: your_api_key"
```

**Response** (200 OK):

```json
{
  "period": {
    "start_date": "2025-01-01",
    "end_date": "2025-12-31"
  },
  "service_fees": [
    {
      "gateway_id": "gateway-xendit-idr",
      "currency": "IDR",
      "total_amount": "453500",
      "transaction_count": 10
    }
  ],
  "taxes": [
    {
      "tax_rate": "0.10",
      "currency": "IDR",
      "total_amount": "1000000",
      "transaction_count": 10
    }
  ],
  "summary": {
    "total_service_fees": "453500",
    "total_taxes": "1000000",
    "total_revenue": "15000000"
  }
}
```

---

## Gateways

### GET /v1/gateways

List all active payment gateways.

**Response** (200 OK):

```json
[
  {
    "id": "gateway-xendit-idr",
    "name": "Xendit IDR",
    "supported_currencies": ["IDR"],
    "fee_percentage": "0.0290",
    "fee_fixed": "2000.0000",
    "is_active": true,
    "environment": "sandbox"
  },
  {
    "id": "gateway-midtrans-idr",
    "name": "Midtrans IDR",
    "supported_currencies": ["IDR"],
    "fee_percentage": "0.0280",
    "fee_fixed": "1500.0000",
    "is_active": true,
    "environment": "sandbox"
  }
]
```

---

## Taxes

### GET /v1/taxes

List all active tax rates.

**Response** (200 OK):

```json
[
  {
    "id": "tax_001",
    "name": "VAT 10%",
    "rate": "0.10",
    "currency": "IDR",
    "category": "VAT",
    "is_active": true
  }
]
```

### GET /v1/taxes/{id}

Get a specific tax rate by ID.

**Path Parameters**:

- `id` (string): Tax ID

**Response** (200 OK):

```json
{
  "id": "tax_001",
  "name": "VAT 10%",
  "rate": "0.10",
  "currency": "IDR",
  "category": "VAT",
  "is_active": true
}
```

---

## Error Handling

All errors return a consistent JSON structure:

```json
{
  "error": {
    "code": "INVALID_REQUEST",
    "message": "Validation failed: currency must be one of IDR, MYR, USD",
    "details": {
      "field": "currency",
      "value": "XXX"
    }
  }
}
```

### Error Codes

| HTTP Status | Error Code            | Description                      |
| ----------- | --------------------- | -------------------------------- |
| 400         | `INVALID_REQUEST`     | Invalid input data               |
| 401         | `UNAUTHORIZED`        | Missing or invalid API key       |
| 404         | `NOT_FOUND`           | Resource not found               |
| 409         | `CONFLICT`            | Resource conflict (duplicate ID) |
| 422         | `VALIDATION_ERROR`    | Input validation failed          |
| 429         | `RATE_LIMIT_EXCEEDED` | Too many requests                |
| 500         | `INTERNAL_ERROR`      | Server error                     |
| 503         | `SERVICE_UNAVAILABLE` | Database or gateway unavailable  |

---

## Complete Example: Creating an Invoice

```bash
#!/bin/bash

# Set your API key
API_KEY="your_api_key_here"
BASE_URL="http://127.0.0.1:8080"

# Create an invoice with 2 installments
curl -X POST "$BASE_URL/v1/invoices" \
  -H "X-API-Key: $API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "external_id": "ORDER-'$(date +%s)'",
    "gateway_id": "gateway-xendit-idr",
    "currency": "IDR",
    "line_items": [
      {
        "description": "Premium Subscription",
        "quantity": 1,
        "unit_price": "1000000",
        "tax_rate": "0.10"
      }
    ],
    "installment_config": {
      "count": 2
    }
  }' | jq '.'
```

Expected output:

```json
{
  "id": "inv_abc123",
  "external_id": "ORDER-1730548800",
  "currency": "IDR",
  "subtotal": "1000000",
  "tax_total": "100000",
  "service_fee": "31000",
  "total": "1131000",
  "status": "pending",
  ...
}
```

---

## Development Notes

### Database Requirements

- MySQL 8.0+
- Tables: `payment_gateways`, `api_keys`, `invoices`, `line_items`, `installment_schedules`, `payment_transactions`
- Run migrations: `sqlx migrate run`

### Environment Variables

See `.env.example` for configuration options.

### Testing

Run endpoint tests:

```bash
./scripts/test_endpoints.sh
```

---

**Need help?** Check the quickstart guide at `docs/quickstart.md`
