use crate::core::Result;
use async_trait::async_trait;

/// Base repository trait for CRUD operations
#[async_trait]
pub trait Repository<T>: Send + Sync {
    /// Create a new entity
    async fn create(&self, entity: T) -> Result<T>;

    /// Find an entity by ID
    async fn find_by_id(&self, id: &str) -> Result<Option<T>>;

    /// Update an existing entity
    async fn update(&self, id: &str, entity: T) -> Result<T>;

    /// Delete an entity by ID
    async fn delete(&self, id: &str) -> Result<()>;

    /// List all entities with optional pagination
    async fn list(&self, limit: Option<u32>, offset: Option<u32>) -> Result<Vec<T>>;
}
