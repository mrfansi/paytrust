use async_trait::async_trait;
use crate::core::error::AppResult;

/// Base repository trait for CRUD operations
/// All repositories should implement this trait for consistency
#[async_trait]
pub trait Repository<T, ID>: Send + Sync {
    /// Create a new entity
    async fn create(&self, entity: T) -> AppResult<T>;

    /// Find entity by ID
    async fn find_by_id(&self, id: ID) -> AppResult<Option<T>>;

    /// Update an existing entity
    async fn update(&self, id: ID, entity: T) -> AppResult<T>;

    /// Delete an entity by ID
    async fn delete(&self, id: ID) -> AppResult<()>;

    /// List all entities (with optional pagination)
    async fn list(&self, limit: Option<u32>, offset: Option<u32>) -> AppResult<Vec<T>>;
}
