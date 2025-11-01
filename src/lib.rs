//! PayTrust Payment Orchestration Platform Library
//!
//! This library provides the core functionality for the PayTrust payment orchestration system.

pub mod config;
pub mod core;
pub mod middleware;
pub mod modules;

// Re-export commonly used types
pub use modules::gateways;
pub use modules::invoices;
pub use modules::taxes;
pub use modules::transactions;
