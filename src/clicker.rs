use std::time::Duration;

const NANOS_PER_SECOND: u64 = 1_000_000_000;
const NANOS_PER_SECOND_DECIMAL: &str = "1000000000";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FixedRate {
    requested_cps: String,
    interval: Duration,
}

impl FixedRate {
    pub fn requested_cps(&self) -> &str {
        &self.requested_cps
    }

    pub fn interval(&self) -> Duration {
        self.interval
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SpeedMode {
    Maximum,
    FixedCps(FixedRate),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RateParseError {
    Empty,
    NotANumber,
    Zero,
}

impl std::fmt::Display for RateParseError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => formatter.write_str("Введите частоту кликов."),
            Self::NotANumber => formatter.write_str("Введите положительное целое число."),
            Self::Zero => formatter.write_str("Частота должна быть больше нуля."),
        }
    }
}

impl std::error::Error for RateParseError {}

/// Accepts any positive decimal integer. There is no configurable CPS ceiling.
/// Values above nanosecond timer precision are treated as unthrottled maximum speed.
pub fn parse_cps(input: &str) -> Result<FixedRate, RateParseError> {
    let input = input.trim();
    if input.is_empty() {
        return Err(RateParseError::Empty);
    }
    if !input.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(RateParseError::NotANumber);
    }

    let normalized = input.trim_start_matches('0');
    if normalized.is_empty() {
        return Err(RateParseError::Zero);
    }

    let interval = interval_for_decimal(normalized);
    Ok(FixedRate {
        requested_cps: normalized.to_owned(),
        interval,
    })
}

fn interval_for_decimal(cps: &str) -> Duration {
    if cps.len() > NANOS_PER_SECOND_DECIMAL.len()
        || (cps.len() == NANOS_PER_SECOND_DECIMAL.len()
            && cps > NANOS_PER_SECOND_DECIMAL)
    {
        return Duration::ZERO;
    }

    let cps = cps
        .parse::<u64>()
        .expect("validated decimal CPS value must fit in u64");
    Duration::from_nanos(NANOS_PER_SECOND / cps)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_positive_rates_and_normalizes_leading_zeroes() {
        let rate = parse_cps("1").unwrap();
        assert_eq!(rate.requested_cps(), "1");
        assert_eq!(rate.interval(), Duration::from_secs(1));

        let rate = parse_cps(" 000250 ").unwrap();
        assert_eq!(rate.requested_cps(), "250");
        assert_eq!(rate.interval(), Duration::from_millis(4));
    }

    #[test]
    fn has_no_arbitrary_cps_ceiling() {
        let rate = parse_cps("100001").unwrap();
        assert_eq!(rate.requested_cps(), "100001");
        assert_eq!(rate.interval(), Duration::from_nanos(9_999));

        let rate = parse_cps("1000000000").unwrap();
        assert_eq!(rate.interval(), Duration::from_nanos(1));

        let rate = parse_cps("999999999999999999999999999999999999999999999999").unwrap();
        assert_eq!(rate.interval(), Duration::ZERO);
    }

    #[test]
    fn rejects_empty_non_numeric_and_zero_rates() {
        assert_eq!(parse_cps("  "), Err(RateParseError::Empty));
        assert_eq!(parse_cps("fast"), Err(RateParseError::NotANumber));
        assert_eq!(parse_cps("0"), Err(RateParseError::Zero));
        assert_eq!(parse_cps("0000"), Err(RateParseError::Zero));
        assert_eq!(parse_cps("-20"), Err(RateParseError::NotANumber));
    }
}
