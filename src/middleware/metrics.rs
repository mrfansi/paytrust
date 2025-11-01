// Metrics middleware for collecting response times, error rates, and other observability data
//
// Tracks:
// - Request response times (histogram)
// - HTTP status codes (counter by status)
// - Error rates (counter)
// - Endpoint hit counts (counter by path)

use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage,
};
use futures_util::future::LocalBoxFuture;
use std::future::{ready, Ready};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Metrics storage
#[derive(Debug, Clone)]
pub struct MetricsCollector {
    data: Arc<Mutex<MetricsData>>,
}

#[derive(Debug, Default)]
pub(crate) struct MetricsData {
    /// Total requests processed
    pub(crate) total_requests: u64,
    /// Successful requests (2xx status)
    pub(crate) successful_requests: u64,
    /// Client errors (4xx status)
    pub(crate) client_errors: u64,
    /// Server errors (5xx status)
    pub(crate) server_errors: u64,
    /// Sum of all response times (for calculating average)
    pub(crate) total_response_time_ms: u64,
    /// Minimum response time
    pub(crate) min_response_time_ms: u64,
    /// Maximum response time
    pub(crate) max_response_time_ms: u64,
    /// Request counts by endpoint
    pub(crate) endpoint_counts: std::collections::HashMap<String, u64>,
    /// Error counts by endpoint
    pub(crate) endpoint_errors: std::collections::HashMap<String, u64>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            data: Arc::new(Mutex::new(MetricsData::default())),
        }
    }

    /// Record a request metric
    fn record_request(
        &self,
        path: &str,
        status_code: u16,
        response_time_ms: u64,
    ) {
        let mut data = self.data.lock().unwrap();
        
        data.total_requests += 1;
        data.total_response_time_ms += response_time_ms;
        
        // Update min/max response times
        if data.min_response_time_ms == 0 || response_time_ms < data.min_response_time_ms {
            data.min_response_time_ms = response_time_ms;
        }
        if response_time_ms > data.max_response_time_ms {
            data.max_response_time_ms = response_time_ms;
        }
        
        // Count by status code range
        match status_code {
            200..=299 => data.successful_requests += 1,
            400..=499 => data.client_errors += 1,
            500..=599 => data.server_errors += 1,
            _ => {}
        }
        
        // Count by endpoint
        *data.endpoint_counts.entry(path.to_string()).or_insert(0) += 1;
        
        // Count errors by endpoint
        if status_code >= 400 {
            *data.endpoint_errors.entry(path.to_string()).or_insert(0) += 1;
        }
    }

    /// Get current metrics snapshot
    pub fn get_metrics(&self) -> Metrics {
        let data = self.data.lock().unwrap();
        
        let avg_response_time_ms = if data.total_requests > 0 {
            data.total_response_time_ms / data.total_requests
        } else {
            0
        };
        
        let error_rate = if data.total_requests > 0 {
            ((data.client_errors + data.server_errors) as f64 / data.total_requests as f64) * 100.0
        } else {
            0.0
        };
        
        let success_rate = if data.total_requests > 0 {
            (data.successful_requests as f64 / data.total_requests as f64) * 100.0
        } else {
            0.0
        };
        
        Metrics {
            total_requests: data.total_requests,
            successful_requests: data.successful_requests,
            client_errors: data.client_errors,
            server_errors: data.server_errors,
            avg_response_time_ms,
            min_response_time_ms: data.min_response_time_ms,
            max_response_time_ms: data.max_response_time_ms,
            error_rate,
            success_rate,
            endpoint_counts: data.endpoint_counts.clone(),
            endpoint_errors: data.endpoint_errors.clone(),
        }
    }

    /// Reset all metrics (useful for testing)
    pub fn reset(&self) {
        let mut data = self.data.lock().unwrap();
        *data = MetricsData::default();
    }

    /// Set test data (only available in test builds)
    #[cfg(test)]
    pub fn set_test_data<F>(&self, f: F)
    where
        F: FnOnce(&mut MetricsData),
    {
        let mut data = self.data.lock().unwrap();
        f(&mut data);
    }
}

/// Metrics snapshot
#[derive(Debug, Clone, serde::Serialize)]
pub struct Metrics {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub client_errors: u64,
    pub server_errors: u64,
    pub avg_response_time_ms: u64,
    pub min_response_time_ms: u64,
    pub max_response_time_ms: u64,
    pub error_rate: f64,
    pub success_rate: f64,
    pub endpoint_counts: std::collections::HashMap<String, u64>,
    pub endpoint_errors: std::collections::HashMap<String, u64>,
}

/// Metrics middleware
pub struct MetricsMiddleware {
    collector: MetricsCollector,
}

impl MetricsMiddleware {
    pub fn new(collector: MetricsCollector) -> Self {
        Self { collector }
    }
}

impl<S, B> Transform<S, ServiceRequest> for MetricsMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = MetricsMiddlewareService<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(MetricsMiddlewareService {
            service: Rc::new(service),
            collector: self.collector.clone(),
        }))
    }
}

pub struct MetricsMiddlewareService<S> {
    service: Rc<S>,
    collector: MetricsCollector,
}

impl<S, B> Service<ServiceRequest> for MetricsMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let svc = self.service.clone();
        let collector = self.collector.clone();
        let path = req.path().to_string();
        let start_time = Instant::now();

        Box::pin(async move {
            // Get request ID for logging correlation
            let request_id = req
                .extensions()
                .get::<String>()
                .map(|id| id.clone())
                .unwrap_or_else(|| "unknown".to_string());

            // Process request
            let response = svc.call(req).await?;
            
            // Calculate metrics
            let elapsed = start_time.elapsed();
            let response_time_ms = elapsed.as_millis() as u64;
            let status_code = response.status().as_u16();
            
            // Record metrics
            collector.record_request(&path, status_code, response_time_ms);
            
            // Log metrics with structured logging
            tracing::info!(
                request_id = %request_id,
                path = %path,
                status = status_code,
                response_time_ms = response_time_ms,
                "Request completed"
            );
            
            Ok(response)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_collector_initialization() {
        let collector = MetricsCollector::new();
        let metrics = collector.get_metrics();
        
        assert_eq!(metrics.total_requests, 0);
        assert_eq!(metrics.successful_requests, 0);
        assert_eq!(metrics.client_errors, 0);
        assert_eq!(metrics.server_errors, 0);
    }

    #[test]
    fn test_metrics_recording() {
        let collector = MetricsCollector::new();
        
        collector.set_test_data(|data| {
            data.total_requests = 1;
            data.successful_requests = 1;
            data.total_response_time_ms = 50;
            data.min_response_time_ms = 50;
            data.max_response_time_ms = 50;
        });
        
        let metrics = collector.get_metrics();
        assert_eq!(metrics.total_requests, 1);
        assert_eq!(metrics.successful_requests, 1);
        assert_eq!(metrics.avg_response_time_ms, 50);
    }

    #[test]
    fn test_metrics_error_tracking() {
        let collector = MetricsCollector::new();
        
        collector.set_test_data(|data| {
            data.total_requests = 3;
            data.successful_requests = 1;
            data.client_errors = 1;
            data.server_errors = 1;
        });
        
        let metrics = collector.get_metrics();
        assert_eq!(metrics.total_requests, 3);
        assert_eq!(metrics.successful_requests, 1);
        assert_eq!(metrics.client_errors, 1);
        assert_eq!(metrics.server_errors, 1);
        assert_eq!(metrics.error_rate, 66.66666666666666); // 2/3 * 100
    }

    #[test]
    fn test_metrics_response_time_tracking() {
        let collector = MetricsCollector::new();
        
        collector.set_test_data(|data| {
            data.total_requests = 3;
            data.total_response_time_ms = 300; // 50+100+150
            data.min_response_time_ms = 50;
            data.max_response_time_ms = 150;
        });
        
        let metrics = collector.get_metrics();
        assert_eq!(metrics.avg_response_time_ms, 100); // 300/3
        assert_eq!(metrics.min_response_time_ms, 50);
        assert_eq!(metrics.max_response_time_ms, 150);
    }

    #[test]
    fn test_metrics_endpoint_tracking() {
        let collector = MetricsCollector::new();
        
        collector.set_test_data(|data| {
            data.endpoint_counts.insert("/invoices".to_string(), 2);
            data.endpoint_counts.insert("/health".to_string(), 1);
        });
        
        let metrics = collector.get_metrics();
        assert_eq!(metrics.endpoint_counts.get("/invoices"), Some(&2));
        assert_eq!(metrics.endpoint_counts.get("/health"), Some(&1));
    }

    #[test]
    fn test_metrics_reset() {
        let collector = MetricsCollector::new();
        
        collector.set_test_data(|data| {
            data.total_requests = 1;
        });
        assert_eq!(collector.get_metrics().total_requests, 1);
        
        collector.reset();
        assert_eq!(collector.get_metrics().total_requests, 0);
    }
}
