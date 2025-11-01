// Metrics endpoint controller - exposes collected metrics
//
// GET /metrics - Returns current metrics snapshot

use actix_web::{web, HttpResponse};
use crate::middleware::MetricsCollector;

/// Get current metrics
#[tracing::instrument(skip(collector))]
pub async fn get_metrics(
    collector: web::Data<MetricsCollector>,
) -> HttpResponse {
    let metrics = collector.get_metrics();
    HttpResponse::Ok().json(metrics)
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::test;

    #[actix_web::test]
    async fn test_get_metrics() {
        let collector = MetricsCollector::new();
        
        // Set some test data
        collector.set_test_data(|data| {
            data.total_requests = 1;
            data.successful_requests = 1;
        });
        
        let app_data = web::Data::new(collector);
        let response = get_metrics(app_data).await;
        
        assert_eq!(response.status(), 200);
    }
}
