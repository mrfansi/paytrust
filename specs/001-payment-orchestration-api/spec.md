# Feature Specification: PayTrust Payment Orchestration Platform

**Feature Branch**: `001-payment-orchestration-api`  
**Created**: 2025-11-01  
**Status**: Draft  
**Version**: 0.1.0

## What is PayTrust?

PayTrust is a **backend payment orchestration API** built in Rust that unifies multiple payment gateways (currently Xendit and Midtrans) into a single, developer-friendly interface. It handles the complete transaction lifecycle from invoice creation to payment completion, supporting both single payments and flexible installment plans.

### Core Capabilities

✅ **Invoice Management**

- Create invoices with multiple line items
- Automatic calculation of subtotals, taxes, and gateway service fees
- Support for per-line-item tax rates
- Invoice expiration (default 24 hours)
- External ID tracking for your system's reference

✅ **Installment Payments**

- Configure 2-12 installment plans per invoice
- Equal or custom amount distribution
- Adjust unpaid installments dynamically
- Sequential payment enforcement (pay installment 1 before 2)
- Automatic overpayment handling (excess applies to next installments)

✅ **Multi-Currency Support**

- IDR (Indonesian Rupiah) - zero decimal places
- MYR (Malaysian Ringgit) - 2 decimal places
- USD (US Dollar) - 2 decimal places
- Complete currency isolation (no mixing/conversion)

✅ **Financial Reporting**

- Service fee breakdown by gateway and currency
- Tax breakdown by rate and currency
- Date range filtering
- Transaction counting

✅ **Payment Gateway Integration**

- **Xendit**: IDR, MYR currencies
- **Midtrans**: IDR currency only
- Gateway-specific fee calculation (percentage + fixed)
- Webhook support for payment confirmations

### Technology Stack

- **Language**: Rust 1.91+ (2021 edition)
- **Framework**: actix-web 4.9+ with async/await
- **Database**: MySQL 8.0+ with InnoDB
- **Architecture**: Modular, domain-driven design with repository pattern
- **Testing**: Real database integration tests per Constitution (mocks permitted only for isolated business logic unit tests)

### Key Design Principles

1. **Tax on Subtotal Only**: Tax calculated before service fees added
2. **Immutable Invoices**: Once payment initiated, invoice locked (must cancel/recreate)
3. **Sequential Installments**: Must pay in order (1→2→3)
4. **Proportional Distribution**: Taxes and fees distributed proportionally across installments
5. **Locked Tax Rates**: Tax rate frozen at invoice creation

**Original Input**: "Paytrust is payment orchestration to unify multiple payment gateway. The goals is to help developer to integrates their platform into their payment gateway using this platform effortless by consume the API. All transaction architectures handled by this codebase. This platform can create invoice with the items, can add additional charge like service fee (its charges from payment gateway) and tax, can handling installment payment by dynamic setup their installment, can adjust their payment like example if their payment is Rp1000.000 and has 2 installments so they can adjust for the first one is Rp200.000 and the last one is Rp800.000. For now the third party payment gateway support only Xendit and Midtrans (Indonesia Only). The currency support is IDR (Indonesia), MYR (Malaysia) and USD (Global). This platform must support for get total additional charge like service fee (its charges from payment gateway) and tax to help their finance. This platform only backend API. Consider to make the payment isolation between different currency or region to avoid mismatch calculation"

## Clarifications

### Session 2025-11-01

- Q: Which authentication mechanism should the API use for developer authentication? → A: API Key in Header - Static key passed in request header (e.g., X-API-Key), simple and standard. Each API key represents a separate developer/merchant tenant with isolated data access.
- Q: How should the system handle payment gateway failures when processing invoices? → A: Immediate Fail with Error Response - Return error immediately, let developer retry manually with idempotency
- Q: What rate limiting should be applied to prevent API abuse? → A: 1000 requests per minute per API key - Balanced limit for production use
- Q: How should the system handle failed webhook deliveries from payment gateways? → A: Fixed Interval Retries - 3 retries with increasing delays (1min, 5min, 30min after initial failure)
- Q: What should be the default invoice expiration timeframe before payment must be completed? → A: 24 hours - Standard expiration, balances customer convenience and system efficiency
- Q: Which payment gateway should handle MYR and USD currency transactions? → A: Developer chooses gateway per invoice - Maximum flexibility, more complex routing
- Q: In what order should service fees be calculated when both percentage and fixed amount are present? → A: (Subtotal × %) + Fixed - Standard industry practice, apply percentage first then add fixed
- Q: How should the system handle partial payments that don't match the invoice total? → A: Accept and track - Mark invoice as "partially paid", store difference, merchant handles reconciliation
- Q: Which invoice modifications should be allowed after a payment has been initiated? → A: No modifications allowed - Invoice becomes read-only once payment initiated, must cancel and recreate
- Q: How should the system handle multiple simultaneous payment requests for the same invoice? → A: Lock invoice, first wins - First request locks invoice, others get 409 Conflict "payment in progress"
- Q: In what order should tax and service fees be calculated when both are present on an invoice? → A: Tax on subtotal only - Tax applies to subtotal only, service fees added after: Total = Subtotal + (Subtotal × Tax%) + Service Fee
- Q: Should taxes be applied per line item or at invoice level? → A: Per-line-item tax rates - Each line item can have its own tax rate/category, system calculates tax per item and sums for invoice total
- Q: How should tax be distributed across installment payments when custom amounts are configured? → A: Proportional distribution - Tax distributed proportionally: installment_tax = total_tax × (installment_amount / total_amount)
- Q: What happens if external tax rates change between invoice creation and payment completion? → A: Lock tax rate at creation - Tax rate frozen when invoice created, immutable throughout invoice lifecycle regardless of external rate changes
- Q: What level of detail should tax reports provide for compliance and merchant analysis? → A: Breakdown by tax rate and currency - Group taxes by rate percentage and currency for compliance: {"IDR_10%": amount, "MYR_6%": amount}
- Q: How should installment payments integrate with payment gateways (Xendit/Midtrans) - gateway native or PayTrust-managed? → A: PayTrust-managed separate payments - Each installment treated as independent single payment to gateway, PayTrust tracks schedule and relationship
- Q: Must installments be paid in sequential order (1, 2, 3) or can customers pay any unpaid installment at any time? → A: Sequential order enforced - Installments must be paid in order, payment link for next installment only available after previous paid
- Q: How should rounding discrepancies be handled when dividing amounts across installments (especially for IDR with no decimals)? → A: Last installment absorbs difference - Round down earlier installments, last installment = total minus sum of previous
- Q: What happens when a customer overpays a single installment (e.g., pays Rp 300,000 for Rp 200,000 installment)? → A: Accept excess, auto-apply to remaining installments - Apply excess sequentially to next installments, mark them paid, if total reached mark invoice "fully paid"
- Q: Can developers adjust remaining installment amounts after first payment is made? → A: Can adjust unpaid installments only - Paid installments locked, unpaid amounts can be adjusted while maintaining total remaining balance
- Q: How should the system handle adding line items to an invoice after payment has started? → A: Create supplementary invoice - Original invoice continues unchanged, new invoice created for additional items with separate payment/installment schedule

## User Scenarios & Testing _(mandatory)_

### User Story 1 - Basic Invoice Creation and Payment (Priority: P1)

A developer integrates PayTrust API into their e-commerce platform to create invoices with line items and process payments through either Xendit or Midtrans. The system handles the payment gateway selection and returns the payment status.

**Why this priority**: This is the core functionality - without the ability to create invoices and process payments, no other features matter. This delivers immediate value by enabling basic payment processing.

**Independent Test**: Can be fully tested by creating an invoice with multiple line items, submitting it to a payment gateway, and receiving payment confirmation. Success is measured by receiving a valid payment response and transaction record.

**Acceptance Scenarios**:

1. **Given** a developer has API credentials, **When** they submit an invoice with line items (product name, quantity, price), currency (IDR/MYR/USD), and preferred gateway (Xendit or Midtrans), **Then** the system creates an invoice and returns a unique invoice ID and payment URL
2. **Given** an invoice is created, **When** payment is made through the selected gateway, **Then** the system receives webhook notification and updates invoice status to "paid"
3. **Given** an invoice with MYR currency and Midtrans gateway selection, **When** developer submits the invoice, **Then** system validates gateway currency support and returns error if unsupported
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

### User Story 5 - API Key Management and Invoice Extensions (Priority: P2)

A developer manages API keys for authentication and creates supplementary invoices when customers request additional items after payment has started on the original invoice.

**Why this priority**: API key management is essential for production security (key rotation, revocation) and supplementary invoices enable flexible order modifications without disrupting active payment flows. Both are production-ready features but not required for MVP.

**Independent Test**: Can be tested by generating API keys, rotating them, revoking them, and verifying authentication works correctly. Supplementary invoice testing involves creating an invoice with active payment, then creating a supplementary invoice for additional items with separate payment schedule.

**Acceptance Scenarios**:

1. **Given** an admin has master API key credentials, **When** they request new API key generation via POST /api-keys, **Then** system generates unique API key, stores argon2 hash, returns key once, and logs creation event
2. **Given** an existing API key, **When** admin requests key rotation via PUT /api-keys/{id}/rotate, **Then** system invalidates old key, generates new key, returns new key once, and logs rotation event
3. **Given** an API key is compromised, **When** admin revokes it via DELETE /api-keys/{id}, **Then** system marks key as revoked, rejects future requests with that key, and logs revocation event
4. **Given** an invoice with payment in progress, **When** developer creates supplementary invoice via POST /invoices/{id}/supplementary with new line items, **Then** system creates new invoice referencing parent, inherits currency and gateway, maintains separate payment schedule
5. **Given** a supplementary invoice request, **When** parent invoice does not exist or is itself supplementary, **Then** system rejects request with 400 Bad Request and appropriate error message

---

### Edge Cases

- What happens when a payment gateway (Xendit or Midtrans) is temporarily unavailable or returns an error? System returns immediate error response to developer, who can retry using idempotency key (FR-032)
- How does the system handle webhook failures or delayed webhook notifications from payment gateways? System retries webhook processing 3 times with exponential backoff (1min, 5min, 30min)
- What happens when a customer partially pays an invoice amount (underpayment or overpayment)? System accepts payment, marks invoice as "partially paid", stores difference, and provides reconciliation API for merchant
- How does the system handle installment payment when a customer skips an installment? System marks installment as overdue, merchant handles collection/enforcement, invoice remains "partially paid" until all installments complete
- Can customers pay installments out of order (e.g., pay installment #3 before #1)? No, sequential order enforced - must pay installment 1 before 2, payment URLs only active for next unpaid installment in sequence
- What happens if customer overpays an installment (pays more than installment amount)? System accepts overpayment, automatically applies excess to next installments sequentially, marks invoice "fully paid" if total reached
- Can installment schedule be modified after first payment? Yes, unpaid installments can be adjusted (paid installments locked), system validates sum equals remaining balance and recalculates proportional taxes/fees
- What happens if customer wants to add items to invoice after payment started? System rejects line item additions, developer must create new supplementary invoice for additional items with separate payment schedule
- How are tax and service fees distributed when installment amounts are customized? Tax and service fees distributed proportionally based on each installment's share of total amount
- What happens when a developer tries to modify an invoice after payment has started? System rejects modification with 400 error, invoice is immutable once payment initiated, must cancel and create new invoice
- How does the system handle refunds or payment reversals from the gateway?
- What happens when timezone differences cause payment timestamp discrepancies?
- How does the system handle concurrent payment attempts for the same invoice? System uses pessimistic locking - first payment request locks invoice, subsequent requests receive 409 Conflict with "payment already in progress" message until lock is released
- What happens when tax or service fee rules change mid-transaction? Tax rates and service fee structures are locked at invoice creation (immutable), external changes don't affect existing invoices
- How does the system handle currency-specific formatting and decimal places (IDR has no decimals, USD/MYR have 2)? System respects currency decimal rules (IDR whole numbers, MYR/USD 2 decimals), uses rounding for installment calculations with last installment absorbing difference
- What happens when a developer exceeds the rate limit (1000 requests/minute)? System returns 429 Too Many Requests with Retry-After header

## Requirements _(mandatory)_

### Functional Requirements

#### Core Payment Processing

- **FR-001**: System MUST create invoices with multiple line items, each containing product name, quantity, unit price, and subtotal
- **FR-002**: System MUST support three currencies: IDR (Indonesian Rupiah), MYR (Malaysian Ringgit), and USD (US Dollar)
- **FR-003**: System MUST integrate with Xendit and Midtrans payment gateways for payment processing
- **FR-004**: System MUST generate unique invoice IDs for tracking and reference
- **FR-005**: System MUST calculate invoice subtotal by summing all line item totals (quantity × unit price)
- **FR-006**: System MUST provide RESTful API endpoints for all payment operations
- **FR-007**: System MUST allow developers to specify preferred payment gateway (Xendit or Midtrans) per invoice at creation time via gateway_id parameter (integer foreign key to gateway_configs table) in POST /invoices request body
- **FR-046**: System MUST validate that the selected gateway supports the invoice currency before processing
- **FR-051**: System MUST make invoices immutable (read-only) once payment is initiated - no modifications to line items, amounts, or financial data allowed (exception: unpaid installment amounts can be adjusted per FR-077)
- **FR-052**: System MUST reject modification requests for invoices with status other than "draft" with 400 Bad Request and appropriate error message (exception: unpaid installment schedule adjustments allowed)
- **FR-081**: System MUST reject attempts to add or remove line items from invoices after payment is initiated
- **FR-082**: System MUST provide API to create supplementary invoices that reference the original invoice for additional items requested mid-payment-cycle, with validation: supplementary invoice MUST reference valid parent invoice_id, inherit currency and gateway from parent invoice, and maintain separate payment schedule
- **FR-044**: System MUST set default invoice expiration to 24 hours from creation unless explicitly configured otherwise
- **FR-044a**: System MUST accept optional expires_at parameter (ISO 8601 timestamp) in invoice creation request with validation: maximum 30 days from creation timestamp, minimum 1 hour from creation timestamp, reject requests with expires_at in the past or outside allowed range with 400 Bad Request
- **FR-045**: System MUST automatically mark invoices as "expired" when expiration time is reached and payment is not completed

#### Additional Charges

- **FR-008**: System MUST allow configuring tax rates as percentages applied to invoice subtotal
- **FR-009**: System MUST calculate service fees using the formula: (subtotal × percentage) + fixed_amount, where both percentage and fixed amount are gateway-specific
- **FR-047**: System MUST calculate service fee percentage component before adding fixed amount (industry standard order of operations)
- **FR-055**: System MUST calculate tax on subtotal only, excluding service fees from tax base
- **FR-056**: System MUST calculate total amount using formula: Total = Subtotal + (Subtotal × Tax%) + Service Fee
- **FR-057**: System MUST support per-line-item tax rates, allowing each line item to have its own tax rate or category
- **FR-058**: System MUST calculate tax per line item as (line_item_subtotal × line_item_tax_rate), then sum all line item taxes for invoice total tax
- **FR-010**: System MUST add tax and service fees to subtotal to calculate final payable amount
- **FR-011**: System MUST track and store tax amounts separately from service fees for reporting
- **FR-012**: System MUST generate financial reports showing total service fees collected by gateway and currency
- **FR-013**: System MUST generate financial reports showing total taxes collected by currency
- **FR-063**: System MUST provide tax breakdown in reports grouped by tax rate percentage and currency (e.g., IDR_10%, MYR_6%, USD_0%)
- **FR-064**: System MUST include transaction count for each tax rate category in financial reports

#### Installment Payments

- **FR-014**: System MUST allow configuring installment plans with specified number of installments (2-12 installments)
- **FR-015**: System MUST create default installment schedules with equal payment amounts
- **FR-016**: System MUST allow adjusting individual installment amounts while maintaining total invoice amount
- **FR-017**: System MUST validate that sum of all installment amounts equals total invoice amount
- **FR-077**: System MUST allow modification of unpaid installment amounts after first payment is made
- **FR-078**: System MUST prevent modification of already-paid installments
- **FR-079**: System MUST validate that sum of unpaid installment adjustments equals remaining balance (total - paid amount)
- **FR-080**: System MUST recalculate proportional tax and service fee distribution for adjusted unpaid installments
- **FR-065**: System MUST treat each installment as an independent single payment transaction to the payment gateway, generating separate payment URLs/references for each installment while maintaining internal tracking of installment relationships and schedule independently from gateway (PayTrust-managed installments, not gateway-native)
- **FR-068**: System MUST enforce sequential installment payment order (installment N can only be paid after installment N-1 is completed)
- **FR-069**: System MUST only generate/activate payment URL for the next unpaid installment in sequence
- **FR-070**: System MUST reject payment attempts for out-of-sequence installments with appropriate error message
- **FR-059**: System MUST distribute tax proportionally across installments using formula: installment_tax = total_tax × (installment_amount / total_amount)
- **FR-060**: System MUST distribute service fees proportionally across installments using the same proportional formula
- **FR-071**: System MUST handle rounding discrepancies by rounding down all installments except the last
- **FR-072**: System MUST calculate last installment amount as: total_amount - sum_of_all_previous_installments to ensure exact total match
- **FR-073**: System MUST accept overpayment on individual installments (payment amount exceeds installment amount)
- **FR-074**: System MUST automatically apply excess payment amount sequentially to remaining unpaid installments in order
- **FR-075**: System MUST mark installments as "paid" when covered by excess payment application
- **FR-076**: System MUST mark entire invoice as "fully paid" if total payment received equals or exceeds invoice total, regardless of installment distribution
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
- **FR-027**: System MUST isolate payment gateway configurations by currency (region-based routing is handled by gateway selection per invoice per FR-007, not by system)

#### Transaction Management

- **FR-028**: System MUST receive and process webhook notifications from payment gateways, update invoice status based on gateway responses (pending, paid, failed, expired), and store complete transaction history including timestamps, amounts, and gateway responses
- **FR-031**: System MUST provide API endpoints to query invoice status and payment history
- **FR-032**: System MUST handle idempotent payment requests to prevent duplicate charges
- **FR-038**: System MUST return descriptive error responses when payment gateway is unavailable, including gateway name and error type
- **FR-039**: System MUST NOT automatically retry failed payment gateway requests - developers must explicitly retry with idempotency
- **FR-042**: System MUST retry failed webhook processing 3 times using fixed intervals at cumulative delays from initial failure: retry 1 at T+1min, retry 2 at T+6min, retry 3 at T+36min (total retry window: 36 minutes)
- **FR-043**: System MUST log all webhook retry attempts with timestamps and final status (success/failed after retries)
- **FR-048**: System MUST accept partial payments (underpayment or overpayment) and mark invoice as "partially paid"
- **FR-049**: System MUST store payment amount received, invoice total, and difference (positive for overpayment, negative for underpayment)
- **FR-050**: System MUST provide API endpoint for developers to query payment discrepancies and handle reconciliation

#### API Authentication & Security

- **FR-033**: System MUST authenticate API requests using API keys passed in request header (X-API-Key header)
- **FR-034**: System MUST validate webhook authenticity using gateway-provided signatures
- **FR-035**: System MUST log all API requests and responses for audit trail
- **FR-036**: System MUST return appropriate HTTP status codes and error messages for all API operations
- **FR-037**: System MUST reject requests with missing or invalid API keys with 401 Unauthorized status
- **FR-083**: System MUST provide API endpoints for generating, rotating, and revoking API keys with audit logging (POST /api-keys, PUT /api-keys/{id}/rotate, DELETE /api-keys/{id}), using argon2 hashing algorithm for secure key storage
- **FR-084**: System MUST authenticate API key management endpoints (POST /api-keys, PUT /api-keys/{id}/rotate, DELETE /api-keys/{id}) using master admin API key separate from regular API keys, loaded from ADMIN_API_KEY environment variable, with 401 Unauthorized response for missing or invalid admin key
- **FR-085**: System MUST set payment_initiated_at timestamp on Invoice entity when first payment attempt is made (payment transaction created or gateway payment URL requested), and use this timestamp to enforce invoice immutability per FR-051 (reject modifications when payment_initiated_at IS NOT NULL)
- **FR-040**: System MUST enforce rate limiting of 1000 requests per minute per API key
- **FR-041**: System MUST return 429 Too Many Requests status when rate limit is exceeded with retry-after header
- **FR-061**: System MUST lock all tax rates at invoice creation time, making them immutable throughout invoice lifecycle
- **FR-062**: System MUST use locked tax rates for all payment calculations regardless of external tax rate changes
- **FR-053**: System MUST implement pessimistic locking for invoice payment processing using MySQL SELECT FOR UPDATE with row-level locks, 5-second lock timeout, and automatic deadlock retry (maximum 3 retry attempts with 100ms exponential backoff)
- **FR-054**: System MUST return 409 Conflict status with "payment already in progress" message for concurrent payment requests when lock cannot be acquired within timeout period

### Non-Functional Requirements

- **NFR-001**: API response time MUST be under 2 seconds at 95th percentile for invoice creation, measured using k6 load testing tool against test environment with minimum hardware specification: 4 vCPU cores (2.5GHz+ clock speed), 8GB DDR4 RAM, MySQL 8.0 on dedicated database server with SSD storage (minimum 1000 IOPS, 100GB capacity) (this represents minimum production deployment configuration, not recommended specification)
- **NFR-002**: System MUST handle at least 100 concurrent API requests sustained for 5 minutes, measured using k6 load testing tool with concurrent virtual users maintaining steady request rate (as defined in SC-005)
- **NFR-003**: System MUST maintain 99.5% uptime for API availability measured monthly (allows ~3.6 hours unplanned downtime per month; excludes scheduled maintenance windows announced 48 hours in advance; partial degradation defined as: sustained error rate >50% measured in 1-minute intervals within any consecutive 5-minute window counts as downtime for that period; error rate ≤50% is considered operational)
- **NFR-004**: Payment webhook processing MUST complete within 5 seconds at 95th percentile with 99% success rate (as measured in SC-004), with retry logic per FR-042 for failures
- **NFR-005**: Financial calculations MUST be accurate to the smallest currency unit (1 IDR, 0.01 MYR/USD)
- **NFR-006**: API documentation MUST be provided in OpenAPI 3.0 format as manually-maintained specification file in specs/001-payment-orchestration-api/contracts/openapi.yaml, served via GET /openapi.json endpoint (reading from contracts/ directory), with interactive Swagger UI available at GET /docs rendering the specification
- **NFR-007**: System MUST store all transaction data for at least 7 years for audit compliance (configurable per jurisdiction requirements)

### Key Entities

- **Invoice**: Represents a payment request with line items, currency, amounts (subtotal, tax, service fee, total), status (draft, pending, partially paid, paid, failed, expired), payment gateway assignment (gateway_id foreign key), payment_initiated_at timestamp (TIMESTAMP NULL DEFAULT NULL, set on first payment attempt per FR-085, when NOT NULL invoice becomes read-only per FR-051, no automatic updates), expires_at timestamp (TIMESTAMP, defaults to created_at + 24 hours per FR-044, configurable via optional parameter per FR-044a), optional original_invoice_id field (BIGINT UNSIGNED NULL, foreign key reference to parent invoice for supplementary invoices created when adding items mid-payment per FR-082), and creation/update timestamps (created_at, updated_at)
- **Line Item**: Represents individual product/service in an invoice with product name, quantity, unit price, subtotal, tax rate (percentage), tax category (optional identifier), and calculated tax amount
- **Payment Transaction**: Represents actual payment attempt/completion with transaction ID, gateway transaction reference, amount paid, payment method, timestamp, status, and gateway response data
- **Installment Schedule**: Represents payment plan with installment number, due date, amount, proportionally-calculated tax amount, proportionally-calculated service fee amount, payment status, and associated transaction reference when paid
- **Payment Gateway Configuration**: Represents gateway credentials and settings with gateway name (Xendit/Midtrans), supported currencies, fee structure (percentage + fixed), region, and webhook endpoint
- **Financial Report**: Aggregated data showing total transactions, service fees, taxes, and revenue by currency, gateway, and time period
- **Service Fee**: Calculated charges from payment gateway with amount, calculation method, gateway reference, and currency
- **Tax**: Tax charges applied to invoice with rate percentage, calculated amount, currency, and tax jurisdiction

### Assumptions

- Payment gateways (Xendit and Midtrans) provide webhook notifications for payment status updates
- API consumers (developers) handle user-facing payment UI using gateway-provided payment URLs
- Each API key represents a separate developer/merchant tenant with isolated data access (multi-tenant architecture with tenant_id derived from API key)
- Exchange rates between currencies are NOT handled by this system - each currency operates independently
- Payment gateway credentials are configured per environment (development, staging, production)
- Installment payment scheduling and reminder notifications are handled outside this system
- Each installment is processed as an independent payment transaction to the gateway (not using gateway-native installment features)
- Gateway-specific payment methods (credit card, bank transfer, e-wallet) are abstracted by gateway APIs
- IDR amounts are whole numbers (no decimal places), MYR and USD use 2 decimal places
- Tax rates are configured per invoice/merchant and not automatically determined by location
- Refund processing is handled directly with payment gateways, not through PayTrust API
- Default invoice expiration is 24 hours but can be configured per invoice at creation time

## Success Criteria _(mandatory)_

### Measurable Outcomes

- **SC-001**: Developers can create a complete invoice with line items and process payment in under 3 minutes using API documentation (measured from reading documentation to receiving webhook payment confirmation, including first API call to final webhook receipt)
- **SC-002**: System successfully processes 95% of payment transactions without errors or failures (user input errors defined as 400-level HTTP responses for validation failures; all 500-level responses and gateway timeout errors count toward failure rate)
- **SC-003**: Financial reports accurately reflect 100% of service fees and taxes within 1 hour of transaction completion
- **SC-004**: Payment gateway webhook notifications are processed within 5 seconds with 99% success rate
- **SC-005**: API response times remain under 2 seconds for 95% of requests under sustained load (100 concurrent requests maintained for 5 minutes)
- **SC-006**: Zero currency mismatch errors occur due to payment isolation architecture
- **SC-007**: Installment payment calculations maintain accuracy with zero discrepancy between scheduled total and invoice amount
- **SC-008**: System handles 10,000 invoices per day across all currencies without performance degradation (distributed throughout business hours with peak of 500 invoices/hour)
- **SC-009**: Developers successfully integrate payment flows with less than 5 API calls per transaction
- **SC-010**: 90% of payment status updates are reflected in real-time (within 10 seconds of gateway notification)
- **SC-011**: API key rotation completes within 2 seconds with zero downtime for active requests using old key during rotation window
- **SC-012**: Supplementary invoices are created and linked to parent invoices with 100% referential integrity (no orphaned supplementary invoices)
