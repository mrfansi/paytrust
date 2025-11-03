mod report_controller;

pub use report_controller::ReportController;

// Re-export configure for main.rs
pub fn configure(cfg: &mut actix_web::web::ServiceConfig) {
    report_controller::configure_routes(cfg);
}
