use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpResponse,
};
use futures_util::future::LocalBoxFuture;
use std::future::{ready, Ready};
use std::rc::Rc;

/// Middleware for consistent error response formatting
pub struct ErrorHandler;

impl<S> Transform<S, ServiceRequest> for ErrorHandler
where
    S: Service<ServiceRequest, Response = ServiceResponse, Error = Error> + 'static,
    S::Future: 'static,
{
    type Response = ServiceResponse;
    type Error = Error;
    type InitError = ();
    type Transform = ErrorHandlerMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(ErrorHandlerMiddleware {
            service: Rc::new(service),
        }))
    }
}

pub struct ErrorHandlerMiddleware<S> {
    service: Rc<S>,
}

impl<S> Service<ServiceRequest> for ErrorHandlerMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse, Error = Error> + 'static,
    S::Future: 'static,
{
    type Response = ServiceResponse;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = self.service.clone();

        Box::pin(async move {
            let result = service.call(req).await;

            // Log errors for monitoring
            if let Err(ref err) = result {
                tracing::error!("Request error: {:?}", err);
            }

            result
        })
    }
}

/// Helper function to create standardized error responses
pub fn error_response(status_code: u16, message: String) -> HttpResponse {
    HttpResponse::build(actix_web::http::StatusCode::from_u16(status_code).unwrap())
        .json(serde_json::json!({
            "error": {
                "code": status_code,
                "message": message,
            }
        }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_response_format() {
        let response = error_response(400, "Bad request".to_string());
        assert_eq!(response.status().as_u16(), 400);
    }
}
