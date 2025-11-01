use crate::core::AppError;
use actix_web::{
    error::{JsonPayloadError, PathError, QueryPayloadError},
    http::StatusCode,
    middleware::ErrorHandlerResponse,
    Result,
};

/// Custom error handler middleware for formatting error responses
pub fn error_handler<B>(
    res: actix_web::dev::ServiceResponse<B>,
) -> Result<ErrorHandlerResponse<B>> {
    // Let AppError handle its own formatting via ResponseError trait
    Ok(ErrorHandlerResponse::Response(res.map_into_left_body()))
}

/// Handle JSON payload errors
pub fn json_error_handler(err: JsonPayloadError, _req: &actix_web::HttpRequest) -> actix_web::Error {
    let error_message = match &err {
        JsonPayloadError::Overflow { .. } => "JSON payload too large",
        JsonPayloadError::ContentType => "Content-Type header is not application/json",
        JsonPayloadError::Deserialize(e) => {
            tracing::error!("JSON deserialization error: {}", e);
            "Invalid JSON payload"
        }
        _ => {
            tracing::error!("JSON payload error: {:?}", err);
            "JSON payload error"
        }
    };

    AppError::validation(error_message).into()
}

/// Handle path parameter errors
pub fn path_error_handler(err: PathError, _req: &actix_web::HttpRequest) -> actix_web::Error {
    let error_message = match &err {
        PathError::Deserialize(e) => {
            tracing::error!("Path parameter error: {}", e);
            "Invalid path parameter"
        }
        _ => {
            tracing::error!("Path error: {:?}", err);
            "Invalid path parameter"
        }
    };

    AppError::validation(error_message).into()
}

/// Handle query parameter errors
pub fn query_error_handler(
    err: QueryPayloadError,
    _req: &actix_web::HttpRequest,
) -> actix_web::Error {
    let error_message = match &err {
        QueryPayloadError::Deserialize(e) => {
            tracing::error!("Query parameter error: {}", e);
            "Invalid query parameter"
        }
        _ => {
            tracing::error!("Query payload error: {:?}", err);
            "Invalid query parameter"
        }
    };

    AppError::validation(error_message).into()
}

/// Log errors with appropriate level based on status code
pub fn log_error(err: &actix_web::Error, req: &actix_web::HttpRequest) {
    let status = err
        .as_response_error()
        .status_code();

    let path = req.path();
    let method = req.method();

    match status {
        StatusCode::INTERNAL_SERVER_ERROR => {
            tracing::error!(
                method = %method,
                path = %path,
                status = %status,
                error = %err,
                "Internal server error"
            );
        }
        StatusCode::BAD_REQUEST | StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
            tracing::warn!(
                method = %method,
                path = %path,
                status = %status,
                error = %err,
                "Client error"
            );
        }
        _ => {
            tracing::info!(
                method = %method,
                path = %path,
                status = %status,
                "Request completed with error"
            );
        }
    }
}
