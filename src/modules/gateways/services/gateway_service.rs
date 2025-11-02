use std::collections::HashMap;
use std::sync::Arc;

use tracing::{error, info};

use crate::core::error::{AppError, AppResult};
use super::gateway_trait::{PaymentGateway, PaymentRequest, PaymentResponse};

/// Service for managing and routing to payment gateways
pub struct GatewayService {
    gateways: HashMap<String, Arc<dyn PaymentGateway>>,
}

impl GatewayService {
    /// Create a new GatewayService with configured gateways
    pub fn new() -> Self {
        let gateways: HashMap<String, Arc<dyn PaymentGateway>> = HashMap::new();
        
        // Note: In production, these should be loaded from configuration/environment
        // For now, we'll initialize with empty keys - they should be injected
        Self { gateways }
    }

    /// Register a gateway
    pub fn register_gateway(&mut self, gateway: Arc<dyn PaymentGateway>) {
        let name = gateway.name().to_string();
        self.gateways.insert(name, gateway);
    }

    /// Get a gateway by name
    pub fn get_gateway(&self, name: &str) -> AppResult<Arc<dyn PaymentGateway>> {
        self.gateways
            .get(name)
            .cloned()
            .ok_or_else(|| AppError::NotFound(format!("Gateway '{}' not found", name)))
    }

    /// Create a payment using the specified gateway
    pub async fn create_payment(
        &self,
        gateway_name: &str,
        request: PaymentRequest,
    ) -> AppResult<PaymentResponse> {
        info!(
            gateway = %gateway_name,
            external_id = %request.external_id,
            amount = %request.amount,
            currency = %request.currency,
            "Creating payment with gateway"
        );

        let gateway = self.get_gateway(gateway_name)?;
        
        match gateway.create_payment(request).await {
            Ok(response) => {
                info!(
                    gateway = %gateway_name,
                    gateway_reference = %response.gateway_reference,
                    "Payment created successfully"
                );
                Ok(response)
            }
            Err(e) => {
                error!(
                    gateway = %gateway_name,
                    error = %e,
                    "Failed to create payment"
                );
                Err(e)
            }
        }
    }

    /// List all available gateways
    pub fn list_gateways(&self) -> Vec<GatewayInfo> {
        self.gateways
            .values()
            .map(|gateway| GatewayInfo {
                name: gateway.name().to_string(),
                supported_currencies: gateway.supported_currencies(),
            })
            .collect()
    }

    /// Check if a gateway supports a currency
    pub fn supports_currency(&self, gateway_name: &str, currency: &str) -> AppResult<bool> {
        let gateway = self.get_gateway(gateway_name)?;
        Ok(gateway
            .supported_currencies()
            .iter()
            .any(|c| c.eq_ignore_ascii_case(currency)))
    }
}

impl Default for GatewayService {
    fn default() -> Self {
        Self::new()
    }
}

/// Gateway information for listing
#[derive(Debug, Clone, serde::Serialize)]
pub struct GatewayInfo {
    pub name: String,
    pub supported_currencies: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gateway_service_creation() {
        let service = GatewayService::new();
        assert_eq!(service.list_gateways().len(), 0);
    }

    #[test]
    fn test_get_nonexistent_gateway() {
        let service = GatewayService::new();
        let result = service.get_gateway("nonexistent");
        assert!(result.is_err());
    }
}
