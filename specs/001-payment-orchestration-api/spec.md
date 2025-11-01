# Feature Specification: PayTrust Payment Orchestration Platform

**Feature Branch**: `001-payment-orchestration-api`  
**Created**: 2025-11-01  
**Status**: Draft  
**Input**: User description: "Paytrust is payment orchestration to unify multiple payment gateway. The goals is to help developer to integrates their platform into their payment gateway using this platform effortless by consume the API. All transaction architectures handled by this codebase. This platform can create invoice with the items, can add additional charge like service fee (its charges from payment gateway) and tax, can handling installment payment by dynamic setup their installment, can adjust their payment like example if their payment is Rp1000.000 and has 2 installments so they can adjust for the first one is Rp200.000 and the last one is Rp800.000. For now the third party payment gateway support only Xendit and Midtrans (Indonesia Only). The currency support is IDR (Indonesia), MYR (Malaysia) and USD (Global). This platform must support for get total additional charge like service fee (its charges from payment gateway) and tax to help their finance. This platform only backend API. Consider to make the payment isolation between different currency or region to avoid mismatch calculation"

## User Scenarios & Testing _(mandatory)_

### User Story 1 - Basic Invoice Creation and Payment (Priority: P1)

A developer integrates PayTrust API into their e-commerce platform to create invoices with line items and process payments through either Xendit or Midtrans. The system handles the payment gateway selection and returns the payment status.

**Why this priority**: This is the core functionality - without the ability to create invoices and process payments, no other features matter. This delivers immediate value by enabling basic payment processing.

**Independent Test**: Can be fully tested by creating an invoice with multiple line items, submitting it to a payment gateway, and receiving payment confirmation. Success is measured by receiving a valid payment response and transaction record.

**Acceptance Scenarios**:

1. **Given** a developer has API credentials, **When** they submit an invoice with line items (product name, quantity, price) and currency (IDR/MYR/USD), **Then** the system creates an invoice and returns a unique invoice ID and payment URL
2. **Given** an invoice is created, **When** payment is made through Xendit or Midtrans, **Then** the system receives webhook notification and updates invoice status to "paid"
3. **Given** an invoice with IDR currency, **When** developer requests payment, **Then** system routes to appropriate gateway (Xendit or Midtrans) for Indonesian payments
4. **Given** multiple line items with quantities, **When** invoice is created, **Then** system calculates correct total amount based on (quantity × price) for each item

---

### User Story 2 - Additional Charges Management (Priority: P2)

A developer needs to add service fees (charged by payment gateway) and taxes to invoices to reflect the actual amount customers must pay, and generate financial reports showing these additional charges.

**Why this priority**: Additional charges are essential for accurate financial reporting and compliance with tax regulations. Without this, merchants cannot properly account for gateway fees and taxes.

**Independent Test**: Can be tested by creating invoices with tax and service fee configurations, verifying the total amount includes these charges, and generating reports that show breakdown of fees and taxes across all transactions.

**Acceptance Scenarios**:

1. **Given** an invoice is being created, **When** developer specifies tax percentage and service fee rules, **Then** system calculates and adds these charges to the subtotal
2. **Given** a payment gateway charges 2.9% + fixed fee, **When** invoice is created, **Then** system calculates service fee based on gateway-specific rules and includes it in the total
3. **Given** multiple completed transactions, **When** developer requests financial report, **Then** system returns total service fees and taxes collected, broken down by currency and gateway
4. **Given** different tax rates for different regions (Indonesia vs Malaysia vs Global), **When** invoice is created, **Then** system applies correct tax rate based on currency/region

---

### User Story 3 - Installment Payment Configuration (Priority: P3)

A developer enables customers to pay invoices in installments with flexible payment schedules, allowing custom amounts for each installment period while ensuring the total equals the invoice amount.

**Why this priority**: Installment payments increase conversion rates for high-value transactions and provide payment flexibility to customers, but the platform can function without this feature initially.

**Independent Test**: Can be tested by creating an invoice with installment configuration (e.g., 2 installments), customizing the amount for each installment, processing each installment payment separately, and verifying the invoice is marked complete only after all installments are paid.

**Acceptance Scenarios**:

1. **Given** an invoice of Rp 1,000,000, **When** developer configures 2 installments, **Then** system creates installment schedule with default equal splits (Rp 500,000 each)
2. **Given** an installment schedule exists, **When** developer adjusts first installment to Rp 200,000, **Then** system automatically adjusts remaining installments to total Rp 800,000
3. **Given** an invoice with 3 installments, **When** first installment is paid, **Then** system updates invoice status to "partially paid" and tracks remaining installments
4. **Given** all installments are paid, **When** final installment payment is confirmed, **Then** system marks invoice as "fully paid" and closes the payment cycle
5. **Given** an installment payment is overdue, **When** developer queries invoice status, **Then** system returns installment schedule with overdue indicators

---

### User Story 4 - Multi-Currency Payment Isolation (Priority: P4)

A developer processes transactions in multiple currencies (IDR, MYR, USD) with proper isolation to prevent currency mismatch errors and ensure accurate financial calculations for each region.

**Why this priority**: Currency isolation is critical for financial accuracy and compliance, but can be implemented after basic payment flows are working. This prevents catastrophic calculation errors.

**Independent Test**: Can be tested by creating invoices in different currencies simultaneously, processing payments, and verifying that calculations, reports, and transaction records never mix currencies and maintain separate totals per currency.

**Acceptance Scenarios**:

1. **Given** invoices in different currencies (IDR, MYR, USD), **When** system processes payments, **Then** each transaction is isolated by currency with separate accounting
2. **Given** a financial report request, **When** developer queries totals, **Then** system returns separate totals for each currency (no mixing or conversion)
3. **Given** an invoice in MYR, **When** developer attempts to add IDR payment, **Then** system rejects the payment with currency mismatch error
4. **Given** service fees calculated in different currencies, **When** system generates report, **Then** fees are grouped by currency without automatic conversion

---

### Edge Cases

- What happens when a payment gateway (Xendit or Midtrans) is temporarily unavailable or returns an error?
- How does the system handle webhook failures or delayed webhook notifications from payment gateways?
- What happens when a customer partially pays an invoice amount (underpayment or overpayment)?
- How does the system handle installment payment when a customer skips an installment?
- What happens when a developer tries to modify an invoice after payment has started?
- How does the system handle refunds or payment reversals from the gateway?
- What happens when timezone differences cause payment timestamp discrepancies?
- How does the system handle concurrent payment attempts for the same invoice?
- What happens when tax or service fee rules change mid-transaction?
- How does the system handle currency-specific formatting and decimal places (IDR has no decimals, USD/MYR have 2)?

## Requirements _(mandatory)_

### Functional Requirements

#### Core Payment Processing

- **FR-001**: System MUST create invoices with multiple line items, each containing product name, quantity, unit price, and subtotal
- **FR-002**: System MUST support three currencies: IDR (Indonesian Rupiah), MYR (Malaysian Ringgit), and USD (US Dollar)
- **FR-003**: System MUST integrate with Xendit and Midtrans payment gateways for payment processing
- **FR-004**: System MUST generate unique invoice IDs for tracking and reference
- **FR-005**: System MUST calculate invoice subtotal by summing all line item totals (quantity × unit price)
- **FR-006**: System MUST provide RESTful API endpoints for all payment operations
- **FR-007**: System MUST route IDR currency payments to Indonesian-supported gateways (Xendit or Midtrans)

#### Additional Charges

- **FR-008**: System MUST allow configuring tax rates as percentages applied to invoice subtotal
- **FR-009**: System MUST calculate service fees based on payment gateway fee structures (percentage + fixed amount)
- **FR-010**: System MUST add tax and service fees to subtotal to calculate final payable amount
- **FR-011**: System MUST track and store tax amounts separately from service fees for reporting
- **FR-012**: System MUST generate financial reports showing total service fees collected by gateway and currency
- **FR-013**: System MUST generate financial reports showing total taxes collected by currency

#### Installment Payments

- **FR-014**: System MUST allow configuring installment plans with specified number of installments (2-12 installments)
- **FR-015**: System MUST create default installment schedules with equal payment amounts
- **FR-016**: System MUST allow adjusting individual installment amounts while maintaining total invoice amount
- **FR-017**: System MUST validate that sum of all installment amounts equals total invoice amount
- **FR-018**: System MUST track payment status for each installment separately
- **FR-019**: System MUST update invoice status to "partially paid" after first installment payment
- **FR-020**: System MUST mark invoice as "fully paid" only after all installments are completed
- **FR-021**: System MUST store installment due dates and payment history

#### Currency Isolation

- **FR-022**: System MUST maintain separate transaction records for each currency
- **FR-023**: System MUST prevent mixing currencies within a single invoice
- **FR-024**: System MUST reject payment attempts in different currency than invoice currency
- **FR-025**: System MUST calculate and report financial totals separately for each currency
- **FR-026**: System MUST handle currency-specific formatting (IDR: no decimals, MYR/USD: 2 decimals)
- **FR-027**: System MUST isolate payment gateway configurations by currency and region

#### Transaction Management

- **FR-028**: System MUST receive and process webhook notifications from payment gateways
- **FR-029**: System MUST update invoice status based on payment gateway responses (pending, paid, failed, expired)
- **FR-030**: System MUST store complete transaction history including timestamps, amounts, and gateway responses
- **FR-031**: System MUST provide API endpoints to query invoice status and payment history
- **FR-032**: System MUST handle idempotent payment requests to prevent duplicate charges

#### API Authentication & Security

- **FR-033**: System MUST authenticate API requests using API keys or tokens
- **FR-034**: System MUST validate webhook authenticity using gateway-provided signatures
- **FR-035**: System MUST log all API requests and responses for audit trail
- **FR-036**: System MUST return appropriate HTTP status codes and error messages for all API operations

### Non-Functional Requirements

- **NFR-001**: API response time MUST be under 2 seconds for invoice creation
- **NFR-002**: System MUST handle at least 100 concurrent API requests
- **NFR-003**: System MUST maintain 99.5% uptime for API availability
- **NFR-004**: Payment webhook processing MUST complete within 5 seconds
- **NFR-005**: Financial calculations MUST be accurate to the smallest currency unit (1 IDR, 0.01 MYR/USD)
- **NFR-006**: API documentation MUST be provided in OpenAPI/Swagger format
- **NFR-007**: System MUST store all transaction data for at least 7 years for audit compliance

### Key Entities

- **Invoice**: Represents a payment request with line items, currency, amounts (subtotal, tax, service fee, total), status (draft, pending, partially paid, paid, failed, expired), payment gateway assignment, and creation/update timestamps
- **Line Item**: Represents individual product/service in an invoice with product name, quantity, unit price, subtotal, and optional tax category
- **Payment Transaction**: Represents actual payment attempt/completion with transaction ID, gateway transaction reference, amount paid, payment method, timestamp, status, and gateway response data
- **Installment Schedule**: Represents payment plan with installment number, due date, amount, payment status, and associated transaction reference when paid
- **Payment Gateway Configuration**: Represents gateway credentials and settings with gateway name (Xendit/Midtrans), supported currencies, fee structure (percentage + fixed), region, and webhook endpoint
- **Financial Report**: Aggregated data showing total transactions, service fees, taxes, and revenue by currency, gateway, and time period
- **Service Fee**: Calculated charges from payment gateway with amount, calculation method, gateway reference, and currency
- **Tax**: Tax charges applied to invoice with rate percentage, calculated amount, currency, and tax jurisdiction

### Assumptions

- Payment gateways (Xendit and Midtrans) provide webhook notifications for payment status updates
- API consumers (developers) handle user-facing payment UI using gateway-provided payment URLs
- Exchange rates between currencies are NOT handled by this system - each currency operates independently
- Payment gateway credentials are configured per environment (development, staging, production)
- Installment payment scheduling and reminder notifications are handled outside this system
- Gateway-specific payment methods (credit card, bank transfer, e-wallet) are abstracted by gateway APIs
- IDR amounts are whole numbers (no decimal places), MYR and USD use 2 decimal places
- Tax rates are configured per invoice/merchant and not automatically determined by location
- Refund processing is handled directly with payment gateways, not through PayTrust API
- Invoice expiration rules are configurable and enforced by payment gateways

## Success Criteria _(mandatory)_

### Measurable Outcomes

- **SC-001**: Developers can create a complete invoice with line items and process payment in under 3 minutes using API documentation
- **SC-002**: System successfully processes 95% of payment transactions without errors or failures
- **SC-003**: Financial reports accurately reflect 100% of service fees and taxes within 1 hour of transaction completion
- **SC-004**: Payment gateway webhook notifications are processed within 5 seconds with 99% success rate
- **SC-005**: API response times remain under 2 seconds for 95% of requests under normal load (100 concurrent users)
- **SC-006**: Zero currency mismatch errors occur due to payment isolation architecture
- **SC-007**: Installment payment calculations maintain accuracy with zero discrepancy between scheduled total and invoice amount
- **SC-008**: System handles 10,000 invoices per day across all currencies without performance degradation
- **SC-009**: Developers successfully integrate payment flows with less than 5 API calls per transaction
- **SC-010**: 90% of payment status updates are reflected in real-time (within 10 seconds of gateway notification)
