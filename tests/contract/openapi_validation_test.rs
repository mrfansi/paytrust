/// OpenAPI 3.0 Schema Compliance Tests
/// 
/// This test validates that our OpenAPI specification is:
/// 1. Valid OpenAPI 3.0.x format
/// 2. Contains all required sections
/// 3. Has proper schema definitions
/// 4. Matches our actual API implementation

use serde_json::Value;
use std::collections::HashSet;

#[test]
fn test_openapi_spec_is_valid_yaml() {
    let openapi_yaml = include_str!("../../specs/001-payment-orchestration-api/contracts/openapi.yaml");
    
    // Parse YAML to ensure it's valid
    let parsed: Result<Value, _> = serde_yaml::from_str(openapi_yaml);
    
    assert!(
        parsed.is_ok(),
        "OpenAPI spec should be valid YAML: {:?}",
        parsed.err()
    );
}

#[test]
fn test_openapi_version_is_3_0_x() {
    let openapi_yaml = include_str!("../../specs/001-payment-orchestration-api/contracts/openapi.yaml");
    let spec: Value = serde_yaml::from_str(openapi_yaml).expect("Failed to parse OpenAPI YAML");
    
    let version = spec["openapi"]
        .as_str()
        .expect("OpenAPI version should be a string");
    
    assert!(
        version.starts_with("3.0"),
        "OpenAPI version should be 3.0.x, got: {}",
        version
    );
}

#[test]
fn test_openapi_has_required_info_section() {
    let openapi_yaml = include_str!("../../specs/001-payment-orchestration-api/contracts/openapi.yaml");
    let spec: Value = serde_yaml::from_str(openapi_yaml).expect("Failed to parse OpenAPI YAML");
    
    assert!(spec["info"].is_object(), "Info section should be present");
    assert!(
        spec["info"]["title"].is_string(),
        "Info.title should be present"
    );
    assert!(
        spec["info"]["version"].is_string(),
        "Info.version should be present"
    );
    assert!(
        spec["info"]["description"].is_string(),
        "Info.description should be present"
    );
}

#[test]
fn test_openapi_has_required_paths_section() {
    let openapi_yaml = include_str!("../../specs/001-payment-orchestration-api/contracts/openapi.yaml");
    let spec: Value = serde_yaml::from_str(openapi_yaml).expect("Failed to parse OpenAPI YAML");
    
    assert!(spec["paths"].is_object(), "Paths section should be present");
    
    let paths = spec["paths"].as_object().expect("Paths should be an object");
    assert!(!paths.is_empty(), "Paths section should not be empty");
}

#[test]
fn test_openapi_has_components_schemas() {
    let openapi_yaml = include_str!("../../specs/001-payment-orchestration-api/contracts/openapi.yaml");
    let spec: Value = serde_yaml::from_str(openapi_yaml).expect("Failed to parse OpenAPI YAML");
    
    assert!(
        spec["components"].is_object(),
        "Components section should be present"
    );
    assert!(
        spec["components"]["schemas"].is_object(),
        "Components.schemas section should be present"
    );
    
    let schemas = spec["components"]["schemas"]
        .as_object()
        .expect("Schemas should be an object");
    assert!(!schemas.is_empty(), "Schemas section should not be empty");
}

#[test]
fn test_openapi_has_security_definitions() {
    let openapi_yaml = include_str!("../../specs/001-payment-orchestration-api/contracts/openapi.yaml");
    let spec: Value = serde_yaml::from_str(openapi_yaml).expect("Failed to parse OpenAPI YAML");
    
    // Check global security requirement
    assert!(
        spec["security"].is_array() || spec["components"]["securitySchemes"].is_object(),
        "Security definitions should be present"
    );
    
    if spec["components"]["securitySchemes"].is_object() {
        let security_schemes = spec["components"]["securitySchemes"]
            .as_object()
            .expect("SecuritySchemes should be an object");
        assert!(
            !security_schemes.is_empty(),
            "At least one security scheme should be defined"
        );
    }
}

#[test]
fn test_openapi_has_servers_defined() {
    let openapi_yaml = include_str!("../../specs/001-payment-orchestration-api/contracts/openapi.yaml");
    let spec: Value = serde_yaml::from_str(openapi_yaml).expect("Failed to parse OpenAPI YAML");
    
    assert!(spec["servers"].is_array(), "Servers section should be present");
    
    let servers = spec["servers"].as_array().expect("Servers should be an array");
    assert!(!servers.is_empty(), "At least one server should be defined");
    
    // Verify each server has a URL
    for server in servers {
        assert!(
            server["url"].is_string(),
            "Each server should have a URL"
        );
    }
}

#[test]
fn test_openapi_invoice_endpoints_defined() {
    let openapi_yaml = include_str!("../../specs/001-payment-orchestration-api/contracts/openapi.yaml");
    let spec: Value = serde_yaml::from_str(openapi_yaml).expect("Failed to parse OpenAPI YAML");
    
    let paths = spec["paths"].as_object().expect("Paths should be an object");
    
    // Check critical invoice endpoints
    let required_paths = vec![
        "/invoices",
        "/invoices/{invoice_id}",
        "/invoices/{invoice_id}/installments",
        "/invoices/{invoice_id}/transactions",
    ];
    
    for path in required_paths {
        assert!(
            paths.contains_key(path),
            "Required path {} should be defined",
            path
        );
    }
}

#[test]
fn test_openapi_schemas_have_required_properties() {
    let openapi_yaml = include_str!("../../specs/001-payment-orchestration-api/contracts/openapi.yaml");
    let spec: Value = serde_yaml::from_str(openapi_yaml).expect("Failed to parse OpenAPI YAML");
    
    let schemas = spec["components"]["schemas"]
        .as_object()
        .expect("Schemas should be an object");
    
    // Check that key schemas exist
    // Note: LineItem is defined inline in CreateInvoiceRequest, not as a separate schema
    let required_schemas = vec![
        "CreateInvoiceRequest",
        "InvoiceResponse",
        "Currency",
        "InvoiceStatus",
    ];
    
    for schema_name in required_schemas {
        assert!(
            schemas.contains_key(schema_name),
            "Required schema {} should be defined",
            schema_name
        );
    }
}

#[test]
fn test_openapi_operations_have_operation_ids() {
    let openapi_yaml = include_str!("../../specs/001-payment-orchestration-api/contracts/openapi.yaml");
    let spec: Value = serde_yaml::from_str(openapi_yaml).expect("Failed to parse OpenAPI YAML");
    
    let paths = spec["paths"].as_object().expect("Paths should be an object");
    
    let mut operation_ids = HashSet::new();
    
    for (path, path_item) in paths {
        if let Some(obj) = path_item.as_object() {
            for (method, operation) in obj {
                if ["get", "post", "put", "patch", "delete"].contains(&method.as_str()) {
                    if let Some(op_obj) = operation.as_object() {
                        assert!(
                            op_obj.contains_key("operationId"),
                            "Operation {} {} should have an operationId",
                            method.to_uppercase(),
                            path
                        );
                        
                        if let Some(op_id) = op_obj["operationId"].as_str() {
                            assert!(
                                !operation_ids.contains(op_id),
                                "Duplicate operationId found: {}",
                                op_id
                            );
                            operation_ids.insert(op_id.to_string());
                        }
                    }
                }
            }
        }
    }
    
    assert!(
        !operation_ids.is_empty(),
        "At least one operation should be defined"
    );
}

#[test]
fn test_openapi_operations_have_responses() {
    let openapi_yaml = include_str!("../../specs/001-payment-orchestration-api/contracts/openapi.yaml");
    let spec: Value = serde_yaml::from_str(openapi_yaml).expect("Failed to parse OpenAPI YAML");
    
    let paths = spec["paths"].as_object().expect("Paths should be an object");
    
    for (path, path_item) in paths {
        if let Some(obj) = path_item.as_object() {
            for (method, operation) in obj {
                if ["get", "post", "put", "patch", "delete"].contains(&method.as_str()) {
                    if let Some(op_obj) = operation.as_object() {
                        assert!(
                            op_obj.contains_key("responses"),
                            "Operation {} {} should have responses defined",
                            method.to_uppercase(),
                            path
                        );
                        
                        let responses = op_obj["responses"]
                            .as_object()
                            .expect("Responses should be an object");
                        assert!(
                            !responses.is_empty(),
                            "Operation {} {} should have at least one response",
                            method.to_uppercase(),
                            path
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn test_openapi_currency_enum_matches_implementation() {
    let openapi_yaml = include_str!("../../specs/001-payment-orchestration-api/contracts/openapi.yaml");
    let spec: Value = serde_yaml::from_str(openapi_yaml).expect("Failed to parse OpenAPI YAML");
    
    let currency_schema = &spec["components"]["schemas"]["Currency"];
    assert!(
        currency_schema.is_object(),
        "Currency schema should be defined"
    );
    
    let enum_values = currency_schema["enum"]
        .as_array()
        .expect("Currency should have enum values");
    
    // Check that our supported currencies are defined
    let currencies: Vec<&str> = enum_values
        .iter()
        .filter_map(|v| v.as_str())
        .collect();
    
    assert!(currencies.contains(&"IDR"), "IDR should be in Currency enum");
    assert!(currencies.contains(&"MYR"), "MYR should be in Currency enum");
    assert!(currencies.contains(&"USD"), "USD should be in Currency enum");
}

#[test]
fn test_openapi_invoice_status_enum_defined() {
    let openapi_yaml = include_str!("../../specs/001-payment-orchestration-api/contracts/openapi.yaml");
    let spec: Value = serde_yaml::from_str(openapi_yaml).expect("Failed to parse OpenAPI YAML");
    
    let status_schema = &spec["components"]["schemas"]["InvoiceStatus"];
    assert!(
        status_schema.is_object(),
        "InvoiceStatus schema should be defined"
    );
    
    let enum_values = status_schema["enum"]
        .as_array()
        .expect("InvoiceStatus should have enum values");
    
    let statuses: Vec<&str> = enum_values
        .iter()
        .filter_map(|v| v.as_str())
        .collect();
    
    // Check required statuses (matching OpenAPI spec: draft, pending, partially_paid, paid, failed, expired)
    assert!(statuses.contains(&"pending"), "pending status should be defined");
    assert!(statuses.contains(&"partially_paid"), "partially_paid status should be defined");
    assert!(statuses.contains(&"paid"), "paid status should be defined");
    assert!(statuses.contains(&"expired"), "expired status should be defined");
    assert!(statuses.contains(&"failed"), "failed status should be defined");
    assert!(statuses.contains(&"draft"), "draft status should be defined");
}

#[test]
fn test_openapi_can_be_converted_to_json() {
    let openapi_yaml = include_str!("../../specs/001-payment-orchestration-api/contracts/openapi.yaml");
    
    // This is what the /api/docs/openapi.json endpoint does
    let result: Result<Value, _> = serde_yaml::from_str(openapi_yaml);
    
    assert!(result.is_ok(), "OpenAPI YAML should be convertible to JSON");
    
    let json_value = result.unwrap();
    let json_string = serde_json::to_string_pretty(&json_value);
    
    assert!(
        json_string.is_ok(),
        "Converted OpenAPI should be serializable to JSON string"
    );
}

#[test]
fn test_openapi_has_proper_content_types() {
    let openapi_yaml = include_str!("../../specs/001-payment-orchestration-api/contracts/openapi.yaml");
    let spec: Value = serde_yaml::from_str(openapi_yaml).expect("Failed to parse OpenAPI YAML");
    
    let paths = spec["paths"].as_object().expect("Paths should be an object");
    
    for (path, path_item) in paths {
        if let Some(obj) = path_item.as_object() {
            for (method, operation) in obj {
                if ["post", "put", "patch"].contains(&method.as_str()) {
                    if let Some(op_obj) = operation.as_object() {
                        if op_obj.contains_key("requestBody") {
                            let request_body = &op_obj["requestBody"];
                            if let Some(content) = request_body["content"].as_object() {
                                assert!(
                                    content.contains_key("application/json"),
                                    "Operation {} {} should accept application/json",
                                    method.to_uppercase(),
                                    path
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}
