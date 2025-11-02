use async_trait::async_trait;
use crate::core::error::AppResult;

/// Base service trait for business logic
/// Services orchestrate business operations across repositories
#[async_trait]
pub trait Service: Send + Sync {
    type Input;
    type Output;

    /// Execute the service operation
    async fn execute(&self, input: Self::Input) -> AppResult<Self::Output>;
}
