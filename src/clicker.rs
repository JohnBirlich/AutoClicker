use std::time::Duration;

pub const MIN_CUSTOM_CPS: u32 = 1;
pub const MAX_CUSTOM_CPS: u32 = 100_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SpeedMode {
    Maximum,
    FixedCps(u32),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RateParseError {
    Empty,
    NotANumber,
    TooLow,
    TooHigh,
}

impl std::fmt::Display for RateParseError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => formatter.write_str("Введите частоту кликов."),
            Self::NotANumber => formatter.write_str("Частота должна быть целым числом."),
            Self::TooLow => write!(formatter, "Минимальная частота — {MIN_CUSTOM_CPS} клик/с."),
            Self::TooHigh => write!(
                formatter,
                "Максимальная заданная частота — {MAX_CUSTOM_CPS} кликов/с."
            ),
        }
    }
}

impl std::error::Error for RateParseError {}

pub fn parse_cps(input: &str) -> Result<u32, RateParseError> {
    let input = input.trim();
    if input.is_empty() {
        return Err(RateParseError::Empty);
    }

    let cps = input
        .parse::<u32>()
        .map_err(|_| RateParseError::NotANumber)?;

    if cps < MIN_CUSTOM_CPS {
        return Err(RateParseError::TooLow);
    }
    if cps > MAX_CUSTOM_CPS {
        return Err(RateParseError::TooHigh);
    }

    Ok(cps)
}

pub fn interval_for_cps(cps: u32) -> Option<Duration> {
    if !(MIN_CUSTOM_CPS..=MAX_CUSTOM_CPS).contains(&cps) {
        return None;
    }

    Some(Duration::from_nanos(1_000_000_000 / u64::from(cps)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_rates_and_whitespace() {
        assert_eq!(parse_cps("1"), Ok(1));
        assert_eq!(parse_cps(" 250 "), Ok(250));
        assert_eq!(parse_cps("100000"), Ok(100_000));
    }

    #[test]
    fn rejects_empty_non_numeric_and_out_of_range_rates() {
        assert_eq!(parse_cps("  "), Err(RateParseError::Empty));
        assert_eq!(parse_cps("fast"), Err(RateParseError::NotANumber));
        assert_eq!(parse_cps("0"), Err(RateParseError::TooLow));
        assert_eq!(parse_cps("100001"), Err(RateParseError::TooHigh));
    }

    #[test]
    fn computes_monotonic_intervals_for_supported_rates() {
        assert_eq!(interval_for_cps(1), Some(Duration::from_secs(1)));
        assert_eq!(interval_for_cps(2), Some(Duration::from_millis(500)));
        assert_eq!(interval_for_cps(1_000), Some(Duration::from_millis(1)));
        assert_eq!(interval_for_cps(100_000), Some(Duration::from_micros(10)));
        assert_eq!(interval_for_cps(0), None);
        assert_eq!(interval_for_cps(100_001), None);
    }
}
