use chrono::{DateTime, FixedOffset, Utc};

/// Gateway-specific timezone handling per FR-087
/// All timestamps stored internally as UTC, converted to gateway timezone for API calls
pub struct TimezoneConverter;

impl TimezoneConverter {
    /// Convert UTC timestamp to Asia/Jakarta timezone (UTC+7) for Midtrans
    pub fn utc_to_jakarta(utc_time: DateTime<Utc>) -> DateTime<FixedOffset> {
        let jakarta_offset = FixedOffset::east_opt(7 * 3600).expect("Valid offset");
        utc_time.with_timezone(&jakarta_offset)
    }

    /// Convert Asia/Jakarta timestamp to UTC
    pub fn jakarta_to_utc(jakarta_time: DateTime<FixedOffset>) -> DateTime<Utc> {
        jakarta_time.with_timezone(&Utc)
    }

    /// Xendit uses UTC, so this is a passthrough
    pub fn utc_to_xendit(utc_time: DateTime<Utc>) -> DateTime<Utc> {
        utc_time
    }

    /// Format timestamp as ISO 8601 UTC for API responses
    pub fn format_iso8601_utc(utc_time: DateTime<Utc>) -> String {
        utc_time.to_rfc3339()
    }
}

/// Convert UTC timestamp to gateway-specific timezone
/// Returns error for unknown gateway names
pub fn convert_to_gateway_timezone(
    utc_time: DateTime<Utc>,
    gateway: &str,
) -> Result<DateTime<FixedOffset>, String> {
    match gateway.to_lowercase().as_str() {
        "midtrans" => {
            let jakarta_offset = FixedOffset::east_opt(7 * 3600).expect("Valid offset");
            Ok(utc_time.with_timezone(&jakarta_offset))
        }
        "xendit" => {
            // Xendit uses UTC, return as FixedOffset with +00:00
            let utc_offset = FixedOffset::east_opt(0).expect("Valid offset");
            Ok(utc_time.with_timezone(&utc_offset))
        }
        _ => Err(format!("Unknown gateway: {}", gateway)),
    }
}

/// Convert timezone-aware timestamp to UTC
pub fn convert_to_utc<Tz: chrono::TimeZone>(time: DateTime<Tz>) -> DateTime<Utc> {
    time.with_timezone(&Utc)
}

/// Format timestamp as ISO 8601 UTC for API responses
pub fn format_iso8601(utc_time: DateTime<Utc>) -> String {
    utc_time.to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Timelike};

    #[test]
    fn test_utc_to_jakarta_conversion() {
        let utc_time = Utc.with_ymd_and_hms(2025, 11, 1, 10, 0, 0).unwrap();
        let jakarta_time = TimezoneConverter::utc_to_jakarta(utc_time);
        
        // Jakarta is UTC+7, so 10:00 UTC = 17:00 Jakarta
        assert_eq!(jakarta_time.hour(), 17);
        assert_eq!(jakarta_time.minute(), 0);
    }

    #[test]
    fn test_jakarta_to_utc_conversion() {
        let jakarta_offset = FixedOffset::east_opt(7 * 3600).unwrap();
        let jakarta_time = jakarta_offset.with_ymd_and_hms(2025, 11, 1, 17, 0, 0).unwrap();
        let utc_time = TimezoneConverter::jakarta_to_utc(jakarta_time);
        
        // 17:00 Jakarta = 10:00 UTC
        assert_eq!(utc_time.hour(), 10);
        assert_eq!(utc_time.minute(), 0);
    }

    #[test]
    fn test_xendit_passthrough() {
        let utc_time = Utc.with_ymd_and_hms(2025, 11, 1, 10, 0, 0).unwrap();
        let xendit_time = TimezoneConverter::utc_to_xendit(utc_time);
        
        // Xendit uses UTC, so should be identical
        assert_eq!(utc_time, xendit_time);
    }

    #[test]
    fn test_iso8601_formatting() {
        let utc_time = Utc.with_ymd_and_hms(2025, 11, 1, 10, 30, 45).unwrap();
        let formatted = TimezoneConverter::format_iso8601_utc(utc_time);
        
        // Should be in RFC3339 format (ISO 8601)
        assert!(formatted.contains("2025-11-01"));
        assert!(formatted.contains("10:30:45"));
        assert!(formatted.ends_with("Z") || formatted.contains("+00:00"));
    }
}
