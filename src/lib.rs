//! Static validators for contracts, config, and runtime state.

pub mod action_plan;
pub mod action_result;
pub mod config;
pub mod gateway;
pub mod state;

use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError {
    pub code: &'static str,
}

impl ValidationError {
    pub const fn new(code: &'static str) -> Self {
        Self { code }
    }
}

pub(crate) fn require_non_empty(value: &str, code: &'static str) -> Result<(), ValidationError> {
    if value.trim().is_empty() {
        return Err(ValidationError::new(code));
    }
    Ok(())
}

pub(crate) fn parse_rfc3339(
    value: &str,
    code: &'static str,
) -> Result<OffsetDateTime, ValidationError> {
    OffsetDateTime::parse(value, &Rfc3339).map_err(|_| ValidationError::new(code))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn require_non_empty_accepts_non_blank_values() {
        assert!(require_non_empty("value", "code").is_ok());
        // 只做空白判定，不裁剪返回值：前后带空格仍算非空。
        assert!(require_non_empty(" value ", "code").is_ok());
    }

    #[test]
    fn require_non_empty_rejects_empty_and_blank_values() {
        for value in ["", " ", "\t", "\n", "   \t\n "] {
            let err = require_non_empty(value, "missing_field").expect_err("should reject");
            assert_eq!(err.code, "missing_field");
        }
    }

    #[test]
    fn parse_rfc3339_accepts_valid_timestamps() {
        assert!(parse_rfc3339("2026-04-12T10:00:00Z", "bad").is_ok());
        assert!(parse_rfc3339("2026-04-12T10:00:00+08:00", "bad").is_ok());
        assert!(parse_rfc3339("2026-04-12T10:00:00.123Z", "bad").is_ok());
    }

    #[test]
    fn parse_rfc3339_rejects_invalid_timestamps() {
        for value in [
            "",
            "not-a-time",
            // 只有日期、缺时区、缺分秒都不算 RFC3339。
            "2026-04-12",
            "2026-04-12T10:00:00",
        ] {
            let err = parse_rfc3339(value, "invalid_time").expect_err("should reject");
            assert_eq!(err.code, "invalid_time");
        }
    }

    #[test]
    fn parse_rfc3339_compares_instants_across_offsets() {
        // 同一时刻的不同时区写法应相等：比较按瞬时，而不是字面量。
        let utc = parse_rfc3339("2026-04-12T10:00:00Z", "bad").expect("utc");
        let plus8 = parse_rfc3339("2026-04-12T18:00:00+08:00", "bad").expect("plus8");
        assert_eq!(utc, plus8);
    }

    #[test]
    fn validation_error_carries_and_compares_by_code() {
        let err = ValidationError::new("some_code");
        assert_eq!(err.code, "some_code");
        assert_eq!(err, ValidationError::new("some_code"));
        assert_ne!(err, ValidationError::new("other_code"));
    }
}
