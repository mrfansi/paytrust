# User Story 4: Multi-Currency Payment Isolation

This example demonstrates handling multiple currencies with strict isolation and currency-specific decimal precision.

## Prerequisites

- Understanding of currency isolation rules
- Knowledge of decimal precision requirements
- Awareness of gateway currency support

## Currency Rules

### Supported Currencies

- **IDR (Indonesian Rupiah)**: Scale = 0 (whole numbers only)
- **MYR (Malaysian Ringgit)**: Scale = 2 (two decimal places)
- **USD (US Dollar)**: Scale = 2 (two decimal places)

### Isolation Requirements

1. **Single Currency per Invoice**: Each invoice must use only one currency
2. **No Mixing**: Cannot combine different currencies in calculations
3. **No Conversion**: Reports show separate totals per currency
4. **Gateway Validation**: Gateway must support the invoice currency

## Step 1: Create Invoice in IDR (Whole Numbers)

### Request

```bash
curl -X POST https://api.paytrust.example.com/v1/invoices \
  -H "X-API-Key: your_api_key_here" \
  -H "Content-Type: application/json" \
  -d '{
    "currency": "IDR",
    "gateway_id": "gateway-xendit-idr",
    "line_items": [
      {
        "product_name": "Premium Plan",
        "quantity": 1,
        "unit_price": "1500000",
        "tax_rate": "0.10"
      }
    ]
  }'
```

### IDR Calculation (No Decimals)

```
Subtotal: 1,500,000
Tax (10%): 150,000
Service Fee: (1,500,000 × 0.029) + 2,000 = 45,500
Total: 1,695,500 (whole number)
```

### Response (201 Created)

```json
{
  "invoice_id": "inv_idr_001",
  "currency": "IDR",
  "status": "pending",
  "subtotal": "1500000",
  "tax_total": "150000",
  "service_fee": "45500",
  "total_amount": "1695500",
  "decimal_precision": {
    "currency": "IDR",
    "scale": 0,
    "note": "Whole numbers only"
  }
}
```

## Step 2: Create Invoice in MYR (Two Decimals)

### Request

```bash
curl -X POST https://api.paytrust.example.com/v1/invoices \
  -H "X-API-Key: your_api_key_here" \
  -H "Content-Type: application/json" \
  -d '{
    "currency": "MYR",
    "gateway_id": "gateway-xendit-myr",
    "line_items": [
      {
        "product_name": "Premium Plan",
        "quantity": 1,
        "unit_price": "450.00",
        "tax_rate": "0.06"
      }
    ]
  }'
```

### MYR Calculation (2 Decimal Places)

```
Subtotal: 450.00
Tax (6%): 27.00
Service Fee: (450.00 × 0.029) + 0.50 = 13.55
Total: 490.55
```

### Response (201 Created)

```json
{
  "invoice_id": "inv_myr_001",
  "currency": "MYR",
  "status": "pending",
  "subtotal": "450.00",
  "tax_total": "27.00",
  "service_fee": "13.55",
  "total_amount": "490.55",
  "decimal_precision": {
    "currency": "MYR",
    "scale": 2,
    "note": "Two decimal places"
  }
}
```

## Step 3: Create Invoice in USD (Two Decimals)

### Request

```bash
curl -X POST https://api.paytrust.example.com/v1/invoices \
  -H "X-API-Key: your_api_key_here" \
  -H "Content-Type: application/json" \
  -d '{
    "currency": "USD",
    "gateway_id": "gateway-midtrans-usd",
    "line_items": [
      {
        "product_name": "Enterprise License",
        "quantity": 1,
        "unit_price": "1200.00",
        "tax_rate": "0.08"
      }
    ]
  }'
```

### USD Calculation

```
Subtotal: 1,200.00
Tax (8%): 96.00
Service Fee: (1,200.00 × 0.029) + 0.30 = 35.10
Total: 1,331.10
```

### Response (201 Created)

```json
{
  "invoice_id": "inv_usd_001",
  "currency": "USD",
  "status": "pending",
  "subtotal": "1200.00",
  "tax_total": "96.00",
  "service_fee": "35.10",
  "total_amount": "1331.10",
  "decimal_precision": {
    "currency": "USD",
    "scale": 2,
    "note": "Two decimal places"
  }
}
```

## Step 4: Currency-Specific Installment Rounding

### IDR - Whole Number Rounding

```bash
curl -X POST https://api.paytrust.example.com/v1/invoices \
  -H "X-API-Key: your_api_key_here" \
  -H "Content-Type: application/json" \
  -d '{
    "currency": "IDR",
    "gateway_id": "gateway-xendit-idr",
    "line_items": [
      {
        "product_name": "Laptop",
        "quantity": 1,
        "unit_price": "10000000",
        "tax_rate": "0.10"
      }
    ],
    "installments": {
      "count": 3
    }
  }'
```

### IDR Installment Calculation

```
Total: 11,292,000
Per installment: 11,292,000 ÷ 3 = 3,764,000 each

Installment breakdown:
  #1: 3,764,000
  #2: 3,764,000
  #3: 3,764,000
  Sum: 11,292,000 ✓ (no rounding needed)
```

### MYR - Decimal Rounding

```bash
curl -X POST https://api.paytrust.example.com/v1/invoices \
  -H "X-API-Key: your_api_key_here" \
  -H "Content-Type: application/json" \
  -d '{
    "currency": "MYR",
    "gateway_id": "gateway-xendit-myr",
    "line_items": [
      {
        "product_name": "Premium Subscription",
        "quantity": 1,
        "unit_price": "1000.00",
        "tax_rate": "0.06"
      }
    ],
    "installments": {
      "count": 3
    }
  }'
```

### MYR Installment Calculation

```
Total: 1,090.90
Per installment: 1,090.90 ÷ 3 = 363.63333...

With 2 decimal rounding:
  #1: 363.63
  #2: 363.63
  #3: 363.64 (absorbs rounding difference of 0.01)
  Sum: 1,090.90 ✓
```

## Step 5: Currency Mismatch Prevention

### Attempt Mixed Currency Payment

```bash
curl -X POST https://api.paytrust.example.com/v1/invoices/inv_idr_001/payments \
  -H "X-API-Key: your_api_key_here" \
  -H "Content-Type: application/json" \
  -d '{
    "amount": "100.00",
    "currency": "USD"
  }'
```

**Response (400 Bad Request):**

```json
{
  "error": {
    "code": "CURRENCY_MISMATCH",
    "message": "Payment currency must match invoice currency",
    "details": {
      "invoice_id": "inv_idr_001",
      "invoice_currency": "IDR",
      "payment_currency": "USD",
      "reason": "Currency mixing not allowed"
    }
  }
}
```

### Gateway Currency Support Validation

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
    "code": "GATEWAY_CURRENCY_NOT_SUPPORTED",
    "message": "Gateway does not support the requested currency",
    "details": {
      "gateway_id": "gateway-xendit-idr-only",
      "requested_currency": "MYR",
      "supported_currencies": ["IDR"]
    }
  }
}
```

## Step 6: Multi-Currency Financial Report

### Request All Currencies

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
  "currencies": ["IDR", "MYR", "USD"],
  "by_currency": [
    {
      "currency": "IDR",
      "decimal_scale": 0,
      "summary": {
        "total_invoices": 125,
        "total_revenue": "450000000",
        "total_taxes_collected": "45000000",
        "total_service_fees": "13050000",
        "net_revenue": "391950000"
      },
      "daily_breakdown": [
        {
          "date": "2025-11-01",
          "invoices": 5,
          "revenue": "15000000",
          "taxes": "1500000",
          "fees": "435000"
        }
      ]
    },
    {
      "currency": "MYR",
      "decimal_scale": 2,
      "summary": {
        "total_invoices": 45,
        "total_revenue": "125000.00",
        "total_taxes_collected": "7500.00",
        "total_service_fees": "3625.00",
        "net_revenue": "113875.00"
      },
      "daily_breakdown": [
        {
          "date": "2025-11-01",
          "invoices": 2,
          "revenue": "5000.00",
          "taxes": "300.00",
          "fees": "145.00"
        }
      ]
    },
    {
      "currency": "USD",
      "decimal_scale": 2,
      "summary": {
        "total_invoices": 12,
        "total_revenue": "35000.00",
        "total_taxes_collected": "2800.00",
        "total_service_fees": "1015.00",
        "net_revenue": "31185.00"
      },
      "daily_breakdown": [
        {
          "date": "2025-11-01",
          "invoices": 1,
          "revenue": "1200.00",
          "taxes": "96.00",
          "fees": "35.10"
        }
      ]
    }
  ],
  "note": "Currencies are NOT converted. Each currency shows separate totals with appropriate decimal precision."
}
```

## Step 7: Filter Report by Single Currency

### Request

```bash
curl -X GET "https://api.paytrust.example.com/v1/reports/financial?currency=IDR&start_date=2025-11-01&end_date=2025-11-30" \
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
  "decimal_scale": 0,
  "summary": {
    "total_invoices": 125,
    "total_revenue": "450000000",
    "total_taxes_collected": "45000000",
    "total_service_fees": "13050000",
    "net_revenue": "391950000"
  },
  "tax_breakdown": [
    {
      "tax_rate": "0.10",
      "invoice_count": 95,
      "subtotal": "400000000",
      "tax_collected": "40000000"
    },
    {
      "tax_rate": "0.11",
      "invoice_count": 30,
      "subtotal": "45454545",
      "tax_collected": "5000000"
    }
  ],
  "service_fee_breakdown": [
    {
      "gateway_id": "gateway-xendit-idr",
      "transaction_count": 80,
      "total_fees": "8500000"
    },
    {
      "gateway_id": "gateway-midtrans-idr",
      "transaction_count": 45,
      "total_fees": "4550000"
    }
  ]
}
```

## Step 8: List Available Gateways by Currency

### Request

```bash
curl -X GET "https://api.paytrust.example.com/v1/gateways?currency=MYR" \
  -H "X-API-Key: your_api_key_here"
```

### Response (200 OK)

```json
{
  "currency": "MYR",
  "available_gateways": [
    {
      "gateway_id": "gateway-xendit-myr",
      "name": "Xendit Malaysia",
      "supported_currencies": ["MYR"],
      "fee_structure": {
        "percentage": "0.029",
        "fixed_amount": "0.50",
        "currency": "MYR"
      },
      "status": "active"
    },
    {
      "gateway_id": "gateway-midtrans-myr",
      "name": "Midtrans Malaysia",
      "supported_currencies": ["MYR"],
      "fee_structure": {
        "percentage": "0.025",
        "fixed_amount": "0.00",
        "currency": "MYR"
      },
      "status": "active"
    }
  ]
}
```

## Currency Precision Examples

### IDR: Rounding to Whole Numbers

```javascript
// Service fee calculation
const subtotal = 1500000; // IDR
const percentage = 0.029;
const fixed = 2000;

const fee = subtotal * percentage + fixed;
// fee = 43500 + 2000 = 45500 (already whole number)

// If there were decimals:
const fee_with_decimals = 45500.67;
const rounded = Math.round(fee_with_decimals);
// rounded = 45501 (round to nearest whole number)
```

### MYR/USD: Rounding to 2 Decimals

```javascript
// Service fee calculation
const subtotal = 450.0; // MYR
const percentage = 0.029;
const fixed = 0.5;

const fee = subtotal * percentage + fixed;
// fee = 13.05 + 0.50 = 13.55

// If more decimals:
const fee_with_extra = 13.556;
const rounded = Math.round(fee_with_extra * 100) / 100;
// rounded = 13.56 (round to 2 decimals)
```

## Error Scenarios

### Invalid Currency

```bash
curl -X POST https://api.paytrust.example.com/v1/invoices \
  -H "X-API-Key: your_api_key_here" \
  -d '{
    "currency": "EUR",
    "gateway_id": "gateway-xendit-eur",
    "line_items": [...]
  }'
```

**Response (400 Bad Request):**

```json
{
  "error": {
    "code": "UNSUPPORTED_CURRENCY",
    "message": "Currency EUR is not supported",
    "details": {
      "provided": "EUR",
      "supported": ["IDR", "MYR", "USD"]
    }
  }
}
```

### Invalid Decimal Precision for IDR

```bash
curl -X POST https://api.paytrust.example.com/v1/invoices \
  -H "X-API-Key: your_api_key_here" \
  -d '{
    "currency": "IDR",
    "gateway_id": "gateway-xendit-idr",
    "line_items": [
      {
        "product_name": "Product",
        "quantity": 1,
        "unit_price": "1000000.50"
      }
    ]
  }'
```

**Response (400 Bad Request):**

```json
{
  "error": {
    "code": "INVALID_CURRENCY_PRECISION",
    "message": "IDR does not support decimal values",
    "details": {
      "currency": "IDR",
      "required_scale": 0,
      "provided_value": "1000000.50",
      "note": "IDR amounts must be whole numbers"
    }
  }
}
```

### Too Many Decimals for MYR/USD

```bash
curl -X POST https://api.paytrust.example.com/v1/invoices \
  -H "X-API-Key: your_api_key_here" \
  -d '{
    "currency": "MYR",
    "gateway_id": "gateway-xendit-myr",
    "line_items": [
      {
        "product_name": "Product",
        "quantity": 1,
        "unit_price": "450.12345"
      }
    ]
  }'
```

**Response (400 Bad Request):**

```json
{
  "error": {
    "code": "INVALID_CURRENCY_PRECISION",
    "message": "MYR supports maximum 2 decimal places",
    "details": {
      "currency": "MYR",
      "required_scale": 2,
      "provided_value": "450.12345",
      "provided_scale": 5
    }
  }
}
```

## Best Practices

1. **Validate Currency**: Always check gateway supports the currency
2. **Respect Precision**: Use whole numbers for IDR, 2 decimals for MYR/USD
3. **No Conversion**: Never convert between currencies in calculations
4. **Separate Reports**: Generate reports per currency or view all separately
5. **Rounding Rules**: Last installment absorbs rounding differences
6. **Gateway Selection**: Choose gateways that support your currency

## Testing Checklist

- [ ] Create invoice in each currency (IDR, MYR, USD)
- [ ] Verify decimal precision enforcement
- [ ] Test currency mismatch rejection
- [ ] Test gateway currency validation
- [ ] Generate multi-currency reports
- [ ] Test installment rounding per currency
- [ ] Verify no currency mixing in calculations

## Next Steps

- Review [API Reference](../api-reference.md) for complete endpoint documentation
- Check [Security Guide](../security.md) for API key management
- See [Deployment Guide](../deployment.md) for production setup
