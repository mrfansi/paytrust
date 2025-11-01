# User Story 3: Installment Payment Configuration

This example demonstrates creating invoices with installment plans, customizing amounts, and handling sequential payments.

## Prerequisites

- Understanding of installment calculation rules
- Knowledge of proportional tax and service fee distribution
- Awareness of sequential payment enforcement

## Installment Calculation Rules

1. **Proportional Distribution**: Taxes and service fees distributed proportionally across installments
2. **Rounding Handling**: Last installment absorbs rounding differences
3. **Sequential Payment**: Must pay installment #1 before #2, etc.
4. **Overpayment Auto-Application**: Excess payment applied to next unpaid installment
5. **Adjustments**: Can modify unpaid installment amounts (paid ones are locked)

## Step 1: Create Invoice with Equal Installments

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
      }
    ],
    "installments": {
      "count": 3
    },
    "expires_at": "2025-12-31T23:59:59Z"
  }'
```

### Calculation Breakdown

```
Invoice Totals:
  - Subtotal: 10,000,000
  - Tax (10%): 1,000,000
  - Service Fee (2.9% + 2,000): 292,000
  - Total: 11,292,000

Equal Split (3 installments):
  - Per installment: 11,292,000 ÷ 3 = 3,764,000 each
  - Rounding adjustment: 0 (divides evenly)

Installment Breakdown:
  #1: 3,764,000
  #2: 3,764,000
  #3: 3,764,000
  Total: 11,292,000 ✓
```

### Response (201 Created)

```json
{
  "invoice_id": "inv_installment_001",
  "currency": "IDR",
  "status": "pending",
  "subtotal": "10000000",
  "tax_total": "1000000",
  "service_fee": "292000",
  "total_amount": "11292000",
  "installment_config": {
    "count": 3,
    "equal_split": true
  },
  "payment_urls": [
    {
      "installment_number": 1,
      "amount": "3764000",
      "status": "pending",
      "url": "https://checkout.xendit.co/web/inv_installment_001_inst_1"
    },
    {
      "installment_number": 2,
      "amount": "3764000",
      "status": "locked",
      "url": null,
      "note": "Available after installment 1 is paid"
    },
    {
      "installment_number": 3,
      "amount": "3764000",
      "status": "locked",
      "url": null,
      "note": "Available after installment 2 is paid"
    }
  ],
  "created_at": "2025-11-02T12:00:00Z"
}
```

## Step 2: Create Invoice with Custom Installment Amounts

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
      }
    ],
    "installments": {
      "count": 3,
      "custom_amounts": [
        "2000000",
        "3000000",
        "6292000"
      ]
    }
  }'
```

### Validation

```
Total amount: 11,292,000
Custom amounts sum: 2,000,000 + 3,000,000 + 6,292,000 = 11,292,000 ✓

PayTrust validates:
1. Sum of custom amounts equals total_amount
2. All amounts are positive
3. Count matches number of custom_amounts
```

### Response (201 Created)

```json
{
  "invoice_id": "inv_custom_inst_001",
  "currency": "IDR",
  "status": "pending",
  "total_amount": "11292000",
  "installment_config": {
    "count": 3,
    "equal_split": false,
    "custom_amounts": true
  },
  "payment_urls": [
    {
      "installment_number": 1,
      "amount": "2000000",
      "status": "pending",
      "url": "https://checkout.xendit.co/web/inv_custom_inst_001_inst_1"
    },
    {
      "installment_number": 2,
      "amount": "3000000",
      "status": "locked",
      "url": null
    },
    {
      "installment_number": 3,
      "amount": "6292000",
      "status": "locked",
      "url": null
    }
  ]
}
```

## Step 3: Retrieve Installment Schedule

### Request

```bash
curl -X GET https://api.paytrust.example.com/v1/invoices/inv_installment_001/installments \
  -H "X-API-Key: your_api_key_here"
```

### Response (200 OK)

```json
{
  "invoice_id": "inv_installment_001",
  "total_amount": "11292000",
  "amount_paid": "0",
  "installments": [
    {
      "installment_id": "inst_001_1",
      "installment_number": 1,
      "amount": "3764000",
      "status": "pending",
      "due_date": null,
      "payment_url": "https://checkout.xendit.co/web/inv_installment_001_inst_1",
      "paid_at": null,
      "transaction_id": null
    },
    {
      "installment_id": "inst_001_2",
      "installment_number": 2,
      "amount": "3764000",
      "status": "locked",
      "due_date": null,
      "payment_url": null,
      "paid_at": null,
      "transaction_id": null
    },
    {
      "installment_id": "inst_001_3",
      "installment_number": 3,
      "amount": "3764000",
      "status": "locked",
      "due_date": null,
      "payment_url": null,
      "paid_at": null,
      "transaction_id": null
    }
  ]
}
```

## Step 4: Payment Flow (Sequential Enforcement)

### 4.1: Pay First Installment

Customer pays via payment URL, gateway sends webhook:

**Webhook from Xendit:**

```json
{
  "external_id": "inv_installment_001_inst_1",
  "status": "PAID",
  "amount": 3764000,
  "paid_at": "2025-11-02T13:00:00Z"
}
```

**PayTrust Processing:**

1. Records transaction for installment #1
2. Updates installment #1 status to `paid`
3. Unlocks installment #2 (generates payment URL)
4. Updates invoice status to `partially_paid`

### 4.2: Check Updated Schedule

```bash
curl -X GET https://api.paytrust.example.com/v1/invoices/inv_installment_001/installments \
  -H "X-API-Key: your_api_key_here"
```

**Response:**

```json
{
  "invoice_id": "inv_installment_001",
  "total_amount": "11292000",
  "amount_paid": "3764000",
  "installments": [
    {
      "installment_number": 1,
      "amount": "3764000",
      "status": "paid",
      "paid_at": "2025-11-02T13:00:00Z",
      "transaction_id": "txn_inst_1_abc123"
    },
    {
      "installment_number": 2,
      "amount": "3764000",
      "status": "pending",
      "payment_url": "https://checkout.xendit.co/web/inv_installment_001_inst_2",
      "paid_at": null
    },
    {
      "installment_number": 3,
      "amount": "3764000",
      "status": "locked",
      "payment_url": null,
      "paid_at": null
    }
  ]
}
```

### 4.3: Attempt to Skip Installment (Will Fail)

If customer tries to pay installment #3 directly:

**Response (400 Bad Request):**

```json
{
  "error": {
    "code": "INSTALLMENT_OUT_OF_SEQUENCE",
    "message": "Cannot pay installment 3. Must pay installment 2 first.",
    "details": {
      "requested_installment": 3,
      "next_required_installment": 2,
      "paid_installments": [1],
      "unpaid_installments": [2, 3]
    }
  }
}
```

## Step 5: Adjust Unpaid Installment Amounts

After paying installment #1, adjust remaining installments:

### Request

```bash
curl -X PATCH https://api.paytrust.example.com/v1/invoices/inv_installment_001/installments \
  -H "X-API-Key: your_api_key_here" \
  -H "Content-Type: application/json" \
  -d '{
    "installments": [
      {
        "installment_number": 2,
        "new_amount": "4000000"
      },
      {
        "installment_number": 3,
        "new_amount": "3528000"
      }
    ]
  }'
```

### Validation

```
Paid amount: 3,764,000 (installment #1)
Remaining: 11,292,000 - 3,764,000 = 7,528,000

New amounts sum: 4,000,000 + 3,528,000 = 7,528,000 ✓

PayTrust validates:
1. Only unpaid installments can be adjusted
2. Sum of new amounts equals remaining balance
3. All new amounts are positive
```

### Response (200 OK)

```json
{
  "invoice_id": "inv_installment_001",
  "installments": [
    {
      "installment_number": 1,
      "amount": "3764000",
      "status": "paid",
      "note": "Cannot be modified (already paid)"
    },
    {
      "installment_number": 2,
      "amount": "4000000",
      "status": "pending",
      "payment_url": "https://checkout.xendit.co/web/inv_installment_001_inst_2",
      "note": "Amount updated"
    },
    {
      "installment_number": 3,
      "amount": "3528000",
      "status": "locked",
      "payment_url": null,
      "note": "Amount updated"
    }
  ],
  "updated_at": "2025-11-02T14:00:00Z"
}
```

## Step 6: Overpayment Handling

Customer pays 5,000,000 for installment #2 (expected: 4,000,000):

### Webhook from Gateway

```json
{
  "external_id": "inv_installment_001_inst_2",
  "status": "PAID",
  "amount": 5000000,
  "paid_at": "2025-11-03T10:00:00Z"
}
```

### PayTrust Auto-Application

```
Expected: 4,000,000
Actual: 5,000,000
Overpayment: 1,000,000

Auto-application:
1. Mark installment #2 as fully paid (4,000,000)
2. Apply 1,000,000 to installment #3
3. Remaining for installment #3: 3,528,000 - 1,000,000 = 2,528,000
```

### Updated Schedule

```bash
curl -X GET https://api.paytrust.example.com/v1/invoices/inv_installment_001/installments \
  -H "X-API-Key: your_api_key_here"
```

**Response:**

```json
{
  "invoice_id": "inv_installment_001",
  "total_amount": "11292000",
  "amount_paid": "8764000",
  "installments": [
    {
      "installment_number": 1,
      "amount": "3764000",
      "status": "paid"
    },
    {
      "installment_number": 2,
      "amount": "4000000",
      "status": "paid",
      "paid_at": "2025-11-03T10:00:00Z",
      "transaction_id": "txn_inst_2_def456",
      "note": "Overpayment of 1,000,000 applied to installment 3"
    },
    {
      "installment_number": 3,
      "amount": "2528000",
      "status": "pending",
      "payment_url": "https://checkout.xendit.co/web/inv_installment_001_inst_3",
      "note": "Adjusted due to overpayment on installment 2"
    }
  ]
}
```

## Step 7: Complete Final Installment

Customer pays final installment:

### After Final Payment

```json
{
  "invoice_id": "inv_installment_001",
  "status": "fully_paid",
  "total_amount": "11292000",
  "amount_paid": "11292000",
  "installments": [
    {
      "installment_number": 1,
      "amount": "3764000",
      "status": "paid",
      "paid_at": "2025-11-02T13:00:00Z"
    },
    {
      "installment_number": 2,
      "amount": "4000000",
      "status": "paid",
      "paid_at": "2025-11-03T10:00:00Z"
    },
    {
      "installment_number": 3,
      "amount": "2528000",
      "status": "paid",
      "paid_at": "2025-11-04T09:00:00Z"
    }
  ],
  "completed_at": "2025-11-04T09:00:05Z"
}
```

## Step 8: Supplementary Invoices

For configuration changes after first payment:

### Request

```bash
curl -X POST https://api.paytrust.example.com/v1/invoices \
  -H "X-API-Key: your_api_key_here" \
  -H "Content-Type: application/json" \
  -d '{
    "original_invoice_id": "inv_installment_001",
    "currency": "IDR",
    "gateway_id": "gateway-xendit-123",
    "line_items": [
      {
        "product_name": "Additional Service",
        "quantity": 1,
        "unit_price": "1000000",
        "tax_rate": "0.10"
      }
    ],
    "note": "Supplementary charge for original invoice"
  }'
```

### Response

```json
{
  "invoice_id": "inv_supp_001",
  "original_invoice_id": "inv_installment_001",
  "type": "supplementary",
  "currency": "IDR",
  "status": "pending",
  "total_amount": "1132000",
  "payment_urls": [
    {
      "url": "https://checkout.xendit.co/web/inv_supp_001"
    }
  ],
  "note": "This is a separate invoice. Does not affect original installment schedule."
}
```

## Error Scenarios

### Invalid Custom Amounts (Sum Mismatch)

```bash
curl -X POST https://api.paytrust.example.com/v1/invoices \
  -H "X-API-Key: your_api_key_here" \
  -d '{
    "total_amount": "11292000",
    "installments": {
      "count": 3,
      "custom_amounts": ["2000000", "3000000", "5000000"]
    }
  }'
```

**Response (400 Bad Request):**

```json
{
  "error": {
    "code": "INSTALLMENT_SUM_MISMATCH",
    "message": "Sum of custom installment amounts must equal total amount",
    "details": {
      "total_amount": "11292000",
      "custom_amounts_sum": "10000000",
      "difference": "1292000"
    }
  }
}
```

### Adjust Paid Installment (Not Allowed)

```bash
curl -X PATCH https://api.paytrust.example.com/v1/invoices/inv_installment_001/installments \
  -H "X-API-Key: your_api_key_here" \
  -d '{
    "installments": [
      {
        "installment_number": 1,
        "new_amount": "5000000"
      }
    ]
  }'
```

**Response (400 Bad Request):**

```json
{
  "error": {
    "code": "INSTALLMENT_ALREADY_PAID",
    "message": "Cannot adjust installment 1 because it has already been paid",
    "details": {
      "installment_number": 1,
      "status": "paid",
      "paid_at": "2025-11-02T13:00:00Z"
    }
  }
}
```

## Best Practices

1. **Sequential Enforcement**: Always pay installments in order
2. **Adjustment Timing**: Adjust amounts before customers pay
3. **Overpayment Handling**: System auto-applies excess to next installment
4. **Custom Amounts**: Validate sum equals total before submitting
5. **Supplementary Invoices**: Use for post-payment changes

## Next Steps

- See [User Story 4](./04-multi-currency-support.md) for currency-specific rounding in installments
