use crate::core::Result;
use async_trait::async_trait;

/// Base service trait for business logic
#[async_trait]
pub trait Service: Send + Sync {
    /// Service-specific initialization or validation
    async fn initialize(&self) -> Result<()>;
}
