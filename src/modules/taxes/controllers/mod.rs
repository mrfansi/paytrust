pub mod tax_controller;

pub use tax_controller::{configure_tax_routes as configure, get_tax_by_id, list_active_taxes};
