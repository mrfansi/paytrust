use chrono::{DateTime, TimeZone, Timelike, Utc};
use paytrust::core::timezone::{convert_to_gateway_timezone, convert_to_utc, format_iso8601};

#[test]
fn test_utc_to_jakarta_conversion() {
    // Test UTC to Asia/Jakarta conversion (UTC+7)
    let utc_time = Utc.with_ymd_and_hms(2025, 11, 3, 10, 0, 0).unwrap();
    let jakarta_time = convert_to_gateway_timezone(utc_time, "midtrans").unwrap();
    
    // Jakarta should be 7 hours ahead
    assert_eq!(jakarta_time.hour(), 17);
    assert_eq!(jakarta_time.minute(), 0);
}

#[test]
fn test_jakarta_to_utc_conversion() {
    // Test Asia/Jakarta to UTC conversion
    let jakarta_time = chrono_tz::Asia::Jakarta
        .with_ymd_and_hms(2025, 11, 3, 17, 0, 0)
        .unwrap();
    let utc_time = convert_to_utc(jakarta_time);
    
    // UTC should be 7 hours behind
    assert_eq!(utc_time.hour(), 10);
    assert_eq!(utc_time.minute(), 0);
}

#[test]
fn test_xendit_utc_passthrough() {
    // Test that Xendit uses UTC (no conversion)
    let utc_time = Utc.with_ymd_and_hms(2025, 11, 3, 10, 0, 0).unwrap();
    let xendit_time = convert_to_gateway_timezone(utc_time, "xendit").unwrap();
    
    // Should remain the same (UTC)
    assert_eq!(xendit_time.hour(), 10);
    assert_eq!(xendit_time.minute(), 0);
}

#[test]
fn test_iso8601_formatting() {
    // Test ISO 8601 UTC formatting
    let utc_time = Utc.with_ymd_and_hms(2025, 11, 3, 10, 30, 45).unwrap();
    let formatted = format_iso8601(utc_time);
    
    // Should be in ISO 8601 format with Z suffix or +00:00
    assert!(formatted.contains("2025-11-03"));
    assert!(formatted.contains("10:30:45"));
    assert!(formatted.ends_with('Z') || formatted.contains("+00:00"));
}

#[test]
fn test_round_trip_conversion() {
    // Test UTC -> Jakarta -> UTC preserves the original time
    let original_utc = Utc.with_ymd_and_hms(2025, 11, 3, 10, 0, 0).unwrap();
    let jakarta_time = convert_to_gateway_timezone(original_utc, "midtrans").unwrap();
    let back_to_utc = convert_to_utc(jakarta_time);
    
    assert_eq!(original_utc.timestamp(), back_to_utc.timestamp());
}

#[test]
fn test_invalid_gateway_timezone() {
    // Test handling of invalid gateway name
    let utc_time = Utc.with_ymd_and_hms(2025, 11, 3, 10, 0, 0).unwrap();
    let result = convert_to_gateway_timezone(utc_time, "invalid_gateway");
    
    // Should return error for unknown gateway
    assert!(result.is_err());
}

#[test]
fn test_daylight_saving_time_handling() {
    // Jakarta doesn't observe DST, so this should be consistent year-round
    let winter_utc = Utc.with_ymd_and_hms(2025, 1, 15, 10, 0, 0).unwrap();
    let summer_utc = Utc.with_ymd_and_hms(2025, 7, 15, 10, 0, 0).unwrap();
    
    let winter_jakarta = convert_to_gateway_timezone(winter_utc, "midtrans").unwrap();
    let summer_jakarta = convert_to_gateway_timezone(summer_utc, "midtrans").unwrap();
    
    // Both should be +7 hours from UTC
    assert_eq!(winter_jakarta.hour(), 17);
    assert_eq!(summer_jakarta.hour(), 17);
}
