# User Story 2: Additional Charges Management (Taxes & Service Fees)

This example demonstrates creating invoices with taxes and service fees, and generating financial reports.

## Prerequisites

- API Key configured
- Gateway registered with fee structure
- Understanding of tax calculation rules (per-line-item, applied to subtotal only)

## Tax Calculation Rules

- **Per-Line-Item**: Tax rate applied to each line item's subtotal (quantity × unit_price)
- **Tax on Subtotal Only**: Service fees are NOT taxed
- **Locked Rates**: Tax rates locked at invoice creation (immutable)
- **Formula**: `total = subtotal + tax_total + service_fee`

## Service Fee Calculation

- **Gateway-Specific**: Each gateway has its own fee structure
- **Formula**: `percentage_fee + fixed_fee`
- **Example**: Xendit IDR = 2.9% + Rp 2,000
- **Applied After Tax**: Service fee is not subject to tax

## Step 1: Create Invoice with Taxes

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
        "product_name": "Laptop",
        "quantity": 1,
        "unit_price": "10000000",
        "tax_rate": "0.10"
      },
      {
        "product_name": "Mouse",
        "quantity": 2,
        "unit_price": "150000",
        "tax_rate": "0.10"
      }
    ],
    "expires_at": "2025-11-05T10:00:00Z"
  }'
```

### Calculation Breakdown

```
Line Item 1 (Laptop):
  - Subtotal: 1 × 10,000,000 = 10,000,000
  - Tax (10%): 10,000,000 × 0.10 = 1,000,000

Line Item 2 (Mouse):
  - Subtotal: 2 × 150,000 = 300,000
  - Tax (10%): 300,000 × 0.10 = 30,000

Invoice Totals:
  - Subtotal: 10,000,000 + 300,000 = 10,300,000
  - Tax Total: 1,000,000 + 30,000 = 1,030,000
  - Service Fee (2.9% + 2,000): (10,300,000 × 0.029) + 2,000 = 300,700
  - Total: 10,300,000 + 1,030,000 + 300,700 = 11,630,700
```

### Response (201 Created)

```json
{
  "invoice_id": "inv_tax_example_001",
  "currency": "IDR",
  "status": "pending",
  "gateway_id": "gateway-xendit-123",
  "line_items": [
    {
      "line_item_id": "li_laptop_001",
      "product_name": "Laptop",
      "quantity": 1,
      "unit_price": "10000000",
      "subtotal": "10000000",
      "tax_rate": "0.10",
      "tax_amount": "1000000"
    },
    {
      "line_item_id": "li_mouse_001",
      "product_name": "Mouse",
      "quantity": 2,
      "unit_price": "150000",
      "subtotal": "300000",
      "tax_rate": "0.10",
      "tax_amount": "30000"
    }
  ],
  "subtotal": "10300000",
  "tax_total": "1030000",
  "service_fee": "300700",
  "total_amount": "11630700",
  "payment_urls": [
    {
      "installment_number": null,
      "url": "https://checkout.xendit.co/web/inv_tax_example_001"
    }
  ],
  "created_at": "2025-11-02T10:00:00Z"
}
```

## Step 2: Mixed Tax Rates

Different products can have different tax rates:

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
        "product_name": "Electronics (VAT 10%)",
        "quantity": 1,
        "unit_price": "5000000",
        "tax_rate": "0.10"
      },
      {
        "product_name": "Books (VAT 0%)",
        "quantity": 3,
        "unit_price": "100000",
        "tax_rate": "0.00"
      },
      {
        "product_name": "Luxury Item (VAT 15%)",
        "quantity": 1,
        "unit_price": "2000000",
        "tax_rate": "0.15"
      }
    ]
  }'
```

### Calculation Breakdown

```
Electronics:
  - Subtotal: 5,000,000
  - Tax (10%): 500,000

Books:
  - Subtotal: 300,000
  - Tax (0%): 0

Luxury Item:
  - Subtotal: 2,000,000
  - Tax (15%): 300,000

Totals:
  - Subtotal: 7,300,000
  - Tax Total: 800,000
  - Service Fee: (7,300,000 × 0.029) + 2,000 = 213,700
  - Total: 8,313,700
```

## Step 3: Generate Financial Report

Retrieve aggregated financial data with tax and fee breakdown:

### Request

```bash
curl -X GET "https://api.paytrust.example.com/v1/reports/financial?start_date=2025-11-01&end_date=2025-11-30&currency=IDR" \
  -H "X-API-Key: your_api_key_here"
```

### Response (200 OK)

```json
{
  "period": {
    "start_date": "2025-11-01",
    "end_date": "2025-11-30"
  },
  "currency": "IDR",
  "summary": {
    "total_invoices": 125,
    "total_revenue": "450000000",
    "total_taxes_collected": "45000000",
    "total_service_fees": "13050000",
    "net_revenue": "391950000"
  },
  "tax_breakdown": [
    {
      "tax_rate": "0.00",
      "invoice_count": 15,
      "subtotal": "25000000",
      "tax_collected": "0"
    },
    {
      "tax_rate": "0.10",
      "invoice_count": 95,
      "subtotal": "400000000",
      "tax_collected": "40000000"
    },
    {
      "tax_rate": "0.15",
      "invoice_count": 15,
      "subtotal": "33333333",
      "tax_collected": "5000000"
    }
  ],
  "service_fee_breakdown": [
    {
      "gateway_id": "gateway-xendit-123",
      "gateway_name": "Xendit IDR",
      "transaction_count": 80,
      "total_fees": "8500000"
    },
    {
      "gateway_id": "gateway-midtrans-001",
      "gateway_name": "Midtrans IDR",
      "transaction_count": 45,
      "total_fees": "4550000"
    }
  ],
  "daily_summary": [
    {
      "date": "2025-11-01",
      "invoices": 5,
      "revenue": "15000000",
      "taxes": "1500000",
      "fees": "435000"
    },
    {
      "date": "2025-11-02",
      "invoices": 8,
      "revenue": "23000000",
      "taxes": "2300000",
      "fees": "667000"
    }
  ]
}
```

## Step 4: Multi-Currency Report

Get separate totals per currency (no conversion):

### Request

```bash
curl -X GET "https://api.paytrust.example.com/v1/reports/financial?start_date=2025-11-01&end_date=2025-11-30" \
  -H "X-API-Key: your_api_key_here"
```

### Response (200 OK)

```json
{
  "period": {
    "start_date": "2025-11-01",
    "end_date": "2025-11-30"
  },
  "by_currency": [
    {
      "currency": "IDR",
      "total_invoices": 125,
      "total_revenue": "450000000",
      "total_taxes_collected": "45000000",
      "total_service_fees": "13050000",
      "net_revenue": "391950000"
    },
    {
      "currency": "MYR",
      "total_invoices": 45,
      "total_revenue": "125000.00",
      "total_taxes_collected": "7500.00",
      "total_service_fees": "3625.00",
      "net_revenue": "113875.00"
    },
    {
      "currency": "USD",
      "total_invoices": 12,
      "total_revenue": "35000.00",
      "total_taxes_collected": "3500.00",
      "total_service_fees": "1015.00",
      "net_revenue": "30485.00"
    }
  ],
  "note": "Currencies are not converted. Each currency shows separate totals."
}
```

## Step 5: Tax Rate Locked at Creation

Once an invoice is created, tax rates cannot be changed:

### Attempt to Modify (Will Fail)

```bash
curl -X PATCH https://api.paytrust.example.com/v1/invoices/inv_tax_example_001 \
  -H "X-API-Key: your_api_key_here" \
  -H "Content-Type: application/json" \
  -d '{
    "line_items": [
      {
        "line_item_id": "li_laptop_001",
        "tax_rate": "0.12"
      }
    ]
  }'
```

**Response (400 Bad Request):**

```json
{
  "error": {
    "code": "INVOICE_IMMUTABLE",
    "message": "Invoice cannot be modified after creation. Tax rates are locked.",
    "details": {
      "invoice_id": "inv_tax_example_001",
      "immutable_fields": ["tax_rate", "unit_price", "quantity"],
      "reason": "Financial compliance requires tax rate locking"
    }
  }
}
```

## Gateway-Specific Service Fees

### Xendit Fee Structure (IDR)

```json
{
  "gateway_id": "gateway-xendit-idr",
  "fee_structure": {
    "percentage": "0.029",
    "fixed_amount": "2000",
    "currency": "IDR"
  }
}
```

**Calculation**: `(subtotal × 0.029) + 2000`

### Midtrans Fee Structure (IDR)

```json
{
  "gateway_id": "gateway-midtrans-idr",
  "fee_structure": {
    "percentage": "0.025",
    "fixed_amount": "0",
    "currency": "IDR"
  }
}
```

**Calculation**: `subtotal × 0.025`

## Testing Scenarios

### Scenario 1: Zero Tax Rate

```bash
curl -X POST https://api.paytrust.example.com/v1/invoices \
  -H "X-API-Key: your_api_key_here" \
  -H "Content-Type: application/json" \
  -d '{
    "currency": "IDR",
    "gateway_id": "gateway-xendit-123",
    "line_items": [
      {
        "product_name": "Tax-Exempt Product",
        "quantity": 1,
        "unit_price": "1000000",
        "tax_rate": "0.00"
      }
    ]
  }'
```

**Result**: Tax total = 0, service fee still applies

### Scenario 2: High-Value Transaction

```bash
curl -X POST https://api.paytrust.example.com/v1/invoices \
  -H "X-API-Key: your_api_key_here" \
  -H "Content-Type: application/json" \
  -d '{
    "currency": "IDR",
    "gateway_id": "gateway-xendit-123",
    "line_items": [
      {
        "product_name": "Enterprise License",
        "quantity": 1,
        "unit_price": "100000000",
        "tax_rate": "0.10"
      }
    ]
  }'
```

**Calculation**:

- Subtotal: 100,000,000
- Tax (10%): 10,000,000
- Service Fee: (100,000,000 × 0.029) + 2,000 = 2,902,000
- Total: 112,902,000

## Best Practices

1. **Always specify tax_rate**: Even if 0%, explicitly set it
2. **Validate tax rates**: Check local tax regulations before setting rates
3. **Monitor reports regularly**: Use financial reports for reconciliation
4. **Consider gateway fees**: Factor service fees into pricing strategy
5. **Use currency-specific precision**: IDR = whole numbers, MYR/USD = 2 decimals

## Next Steps

- See [User Story 3](./03-installment-payments.md) for proportional tax distribution in installments
- See [User Story 4](./04-multi-currency-support.md) for currency-specific tax handling
