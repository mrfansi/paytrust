use super::gateway_trait::{PaymentGateway, PaymentRequest, PaymentResponse, WebhookPayload};
use super::{MidtransClient, XenditClient};
use crate::core::{AppError, Currency, Result};
use crate::modules::gateways::repositories::GatewayRepository;
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::sync::Arc;

/// Gateway service for routing payments to appropriate gateway
///
/// Manages multiple payment gateway clients and routes requests
/// based on gateway_id and currency support
pub struct GatewayService {
    gateways: HashMap<String, Arc<dyn PaymentGateway>>,
    repository: GatewayRepository,
}

impl GatewayService {
    /// Create a new GatewayService
    ///
    /// # Arguments
    /// * `repository` - Gateway configuration repository
    /// * `xendit_client` - Xendit gateway client
    /// * `midtrans_client` - Midtrans gateway client
    pub fn new(
        repository: GatewayRepository,
        xendit_client: XenditClient,
        midtrans_client: MidtransClient,
    ) -> Self {
        let mut gateways: HashMap<String, Arc<dyn PaymentGateway>> = HashMap::new();

        gateways.insert("xendit".to_string(), Arc::new(xendit_client));
        gateways.insert("midtrans".to_string(), Arc::new(midtrans_client));

        Self {
            gateways,
            repository,
        }
    }

    /// Get a gateway by ID
    ///
    /// # Arguments
    /// * `gateway_id` - Gateway identifier (e.g., "xendit", "midtrans")
    ///
    /// # Returns
    /// * `Result<Arc<dyn PaymentGateway>>` - Gateway client
    pub fn get_gateway(&self, gateway_id: &str) -> Result<Arc<dyn PaymentGateway>> {
        self.gateways
            .get(gateway_id)
            .cloned()
            .ok_or_else(|| AppError::not_found(format!("Gateway '{}' not found", gateway_id)))
    }

    /// Validate that gateway supports the given currency
    ///
    /// # Arguments
    /// * `gateway_id` - Gateway identifier
    /// * `currency` - Currency to validate
    ///
    /// # Returns
    /// * `Result<bool>` - True if gateway supports currency (FR-046)
    pub fn validate_gateway_currency(&self, gateway_id: &str, currency: Currency) -> Result<bool> {
        let gateway = self.get_gateway(gateway_id)?;
        Ok(gateway.supports_currency(currency))
    }

    /// Create a payment through the specified gateway
    ///
    /// # Arguments
    /// * `gateway_id` - Gateway identifier
    /// * `request` - Payment request details
    ///
    /// # Returns
    /// * `Result<PaymentResponse>` - Payment response with URL and reference
    pub async fn create_payment(
        &self,
        gateway_id: &str,
        request: PaymentRequest,
    ) -> Result<PaymentResponse> {
        // Validate gateway exists
        let gateway = self.get_gateway(gateway_id)?;

        // Validate currency support (FR-046)
        if !gateway.supports_currency(request.currency) {
            return Err(AppError::validation(format!(
                "Gateway '{}' does not support currency '{}'",
                gateway_id,
                request.currency.to_string()
            )));
        }

        // Create payment
        gateway.create_payment(request).await
    }

    /// Verify webhook signature
    ///
    /// # Arguments
    /// * `gateway_id` - Gateway identifier
    /// * `signature` - Webhook signature from gateway
    /// * `payload` - Raw webhook payload
    ///
    /// # Returns
    /// * `Result<bool>` - True if signature is valid
    pub async fn verify_webhook(
        &self,
        gateway_id: &str,
        signature: &str,
        payload: &str,
    ) -> Result<bool> {
        let gateway = self.get_gateway(gateway_id)?;
        gateway.verify_webhook(signature, payload).await
    }

    /// Process webhook payload
    ///
    /// # Arguments
    /// * `gateway_id` - Gateway identifier
    /// * `payload` - Raw webhook payload
    ///
    /// # Returns
    /// * `Result<WebhookPayload>` - Parsed webhook data
    pub async fn process_webhook(&self, gateway_id: &str, payload: &str) -> Result<WebhookPayload> {
        let gateway = self.get_gateway(gateway_id)?;
        gateway.process_webhook(payload).await
    }

    /// Create installment payment (T096 - FR-065, FR-066, FR-067)
    ///
    /// Creates a separate payment transaction for a specific installment.
    /// Each installment gets its own payment URL and gateway reference.
    ///
    /// # Arguments
    /// * `gateway_id` - Gateway identifier
    /// * `invoice_id` - Invoice ID
    /// * `installment_id` - Installment schedule ID
    /// * `installment_number` - Installment number (1, 2, 3, etc.)
    /// * `total_installments` - Total number of installments
    /// * `amount` - Installment amount
    /// * `currency` - Currency
    /// * `description` - Base description
    ///
    /// # Returns
    /// * `Result<PaymentResponse>` - Payment response with installment-specific URL
    pub async fn create_installment_payment(
        &self,
        gateway_id: &str,
        invoice_id: String,
        installment_id: String,
        installment_number: i32,
        total_installments: i32,
        amount: Decimal,
        currency: Currency,
        description: String,
    ) -> Result<PaymentResponse> {
        let gateway = self.get_gateway(gateway_id)?;

        // Validate currency support (FR-046)
        if !gateway.supports_currency(currency) {
            return Err(AppError::validation(format!(
                "Gateway '{}' does not support currency '{}'",
                gateway_id,
                currency.to_string()
            )));
        }

        // Create installment-specific external ID (FR-066)
        let external_id = format!("{}-installment-{}", invoice_id, installment_number);

        // Create installment description
        let installment_description = format!(
            "{} (Installment {}/{})",
            description, installment_number, total_installments
        );

        // Build payment request with installment info
        let request = PaymentRequest {
            external_id,
            amount,
            currency,
            description: installment_description,
            payer_email: None,
            success_redirect_url: None,
            failure_redirect_url: None,
            installment_info: Some(super::gateway_trait::InstallmentInfo {
                installment_id: installment_id.clone(),
                installment_number,
                total_installments,
                description_suffix: format!(
                    "Installment {}/{}",
                    installment_number, total_installments
                ),
            }),
        };

        // Create payment through gateway (FR-065)
        gateway.create_payment(request).await
    }

    /// List all available gateways
    ///
    /// # Returns
    /// * `Vec<String>` - List of gateway identifiers
    pub fn list_gateways(&self) -> Vec<String> {
        self.gateways.keys().cloned().collect()
    }

    /// Get gateway information including supported currencies
    ///
    /// # Arguments
    /// * `gateway_id` - Gateway identifier
    ///
    /// # Returns
    /// * `Result<GatewayInfo>` - Gateway information
    pub fn get_gateway_info(&self, gateway_id: &str) -> Result<GatewayInfo> {
        let gateway = self.get_gateway(gateway_id)?;

        let supported_currencies = vec![Currency::IDR, Currency::MYR, Currency::USD]
            .into_iter()
            .filter(|c| gateway.supports_currency(*c))
            .collect();

        Ok(GatewayInfo {
            id: gateway_id.to_string(),
            name: gateway.name().to_string(),
            supported_currencies,
        })
    }
}

/// Gateway information response
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GatewayInfo {
    pub id: String,
    pub name: String,
    pub supported_currencies: Vec<Currency>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_service_initialization() {
        let pool = sqlx::MySqlPool::connect_lazy("mysql://test:test@localhost/test").unwrap();
        let repository = GatewayRepository::new(pool);

        let xendit = XenditClient::new("test_key".to_string(), "test_secret".to_string(), None);

        let midtrans = MidtransClient::new("test_key".to_string(), "test_secret".to_string(), None);

        let service = GatewayService::new(repository, xendit, midtrans);
        let gateways = service.list_gateways();

        assert_eq!(gateways.len(), 2);
        assert!(gateways.contains(&"xendit".to_string()));
        assert!(gateways.contains(&"midtrans".to_string()));
    }

    #[tokio::test]
    async fn test_get_gateway() {
        let pool = sqlx::MySqlPool::connect_lazy("mysql://test:test@localhost/test").unwrap();
        let repository = GatewayRepository::new(pool);

        let xendit = XenditClient::new("test_key".to_string(), "test_secret".to_string(), None);
        let midtrans = MidtransClient::new("test_key".to_string(), "test_secret".to_string(), None);
        let service = GatewayService::new(repository, xendit, midtrans);

        assert!(service.get_gateway("xendit").is_ok());
        assert!(service.get_gateway("midtrans").is_ok());
        assert!(service.get_gateway("invalid").is_err());
    }

    #[tokio::test]
    async fn test_validate_currency() {
        let pool = sqlx::MySqlPool::connect_lazy("mysql://test:test@localhost/test").unwrap();
        let repository = GatewayRepository::new(pool);

        let xendit = XenditClient::new("test_key".to_string(), "test_secret".to_string(), None);
        let midtrans = MidtransClient::new("test_key".to_string(), "test_secret".to_string(), None);
        let service = GatewayService::new(repository, xendit, midtrans);

        // Xendit supports IDR and MYR
        assert!(service
            .validate_gateway_currency("xendit", Currency::IDR)
            .unwrap());
        assert!(service
            .validate_gateway_currency("xendit", Currency::MYR)
            .unwrap());
        assert!(!service
            .validate_gateway_currency("xendit", Currency::USD)
            .unwrap());

        // Midtrans supports only IDR
        assert!(service
            .validate_gateway_currency("midtrans", Currency::IDR)
            .unwrap());
        assert!(!service
            .validate_gateway_currency("midtrans", Currency::MYR)
            .unwrap());
    }

    #[tokio::test]
    async fn test_get_gateway_info() {
        let pool = sqlx::MySqlPool::connect_lazy("mysql://test:test@localhost/test").unwrap();
        let repository = GatewayRepository::new(pool);

        let xendit = XenditClient::new("test_key".to_string(), "test_secret".to_string(), None);
        let midtrans = MidtransClient::new("test_key".to_string(), "test_secret".to_string(), None);
        let service = GatewayService::new(repository, xendit, midtrans);

        let xendit_info = service.get_gateway_info("xendit").unwrap();
        assert_eq!(xendit_info.id, "xendit");
        assert_eq!(xendit_info.name, "xendit");
        assert!(xendit_info.supported_currencies.contains(&Currency::IDR));
        assert!(xendit_info.supported_currencies.contains(&Currency::MYR));

        let midtrans_info = service.get_gateway_info("midtrans").unwrap();
        assert_eq!(midtrans_info.id, "midtrans");
        assert!(midtrans_info.supported_currencies.contains(&Currency::IDR));
        assert!(!midtrans_info.supported_currencies.contains(&Currency::MYR));
    }
}
