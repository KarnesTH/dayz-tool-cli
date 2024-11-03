use crate::DncError;
use regex::Regex;

/// Calculates DayZ server settings for Day/Night cycle acceleration.
///
/// This function takes the desired day and night lengths as strings
/// (e.g. "8h" or "10min") and calculates the `serverTimeAcceleration`
/// and `serverNightTimeAcceleration` values to achieve those lengths.
///
/// # Arguments
///
/// * `day`: The desired day length as a string (e.g. "8h" or "480min").
/// * `night`: The desired night length as a string (e.g. "10min" or "0.16667h").
///
/// # Returns
///
/// A `Result` object containing a tuple with the calculated
/// `serverTimeAcceleration` and `serverNightTimeAcceleration` values as `f32`.
/// If an error occurs, an `Err` result with an error message is returned.
pub fn calculate_dnc(day: &str, night: &str) -> Result<(f32, f32), DncError> {
    let day_time = parse_time(day)?;
    let night_time = parse_time(night)?;

    let full_day_duration = 720.0; // 24 hours = 12 hours * 60 minutes

    let time_acceleration = full_day_duration / day_time;
    let night_time_acceleration = (full_day_duration / night_time) / time_acceleration;

    validate_dnc(time_acceleration, night_time_acceleration)
}

/// Parses a time string into a number of minutes.
///
/// The function expects a time string in the format "<number>h" or "<number>min",
/// where <number> is a valid floating-point number.
///
/// # Arguments
///
/// * `time`: The time string to parse.
///
/// # Returns
///
/// A `Result` object containing the parsed time in minutes as a `f32`.
/// If an error occurs, an `Err` result with a `DncError` is returned.
fn parse_time(time: &str) -> Result<f32, DncError> {
    let re = Regex::new(r"(\d+)").unwrap();
    let captures = re.captures(time).ok_or(DncError::InvalidTimeFormat)?;
    let number = captures
        .get(1)
        .ok_or(DncError::InvalidNumber)?
        .as_str()
        .parse::<f32>()
        .map_err(|_| DncError::InvalidNumber)?;

    if time.ends_with('h') {
        Ok(number * 60.0)
    } else if time.ends_with("min") {
        Ok(number)
    } else {
        Err(DncError::InvalidTimeFormat)
    }
}

/// Validates the calculated time acceleration values.
///
/// This function checks if the `time_acceleration` and `night_time_acceleration`
/// values are within the valid range of 0.1 to 64.0.
///
/// # Arguments
///
/// * `time_acceleration`: The calculated time acceleration value.
/// * `night_time_acceleration`: The calculated night time acceleration value.
///
/// # Returns
///
/// A `Result` object containing the validated time acceleration values as a tuple of `f32`.
/// If an error occurs, an `Err` result with a `DncError` is returned.
fn validate_dnc(
    time_acceleration: f32,
    night_time_acceleration: f32,
) -> Result<(f32, f32), DncError> {
    if !(0.1..=64.0).contains(&time_acceleration) {
        return Err(DncError::InvalidTimeAcceleration);
    }

    if !(0.1..=64.0).contains(&night_time_acceleration) {
        return Err(DncError::InvalidNightTimeAcceleration);
    }

    Ok((time_acceleration, night_time_acceleration))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_dnc_valid_input() {
        let result = calculate_dnc("8h", "10min");
        assert_eq!(result.unwrap(), (1.5, 48.0));
    }

    #[test]
    fn test_calculate_dnc_invalid_time_format() {
        let result = calculate_dnc("8", "10");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), DncError::InvalidTimeFormat);
    }

    #[test]
    fn test_calculate_dnc_invalid_time_acceleration() {
        let result = calculate_dnc("0.5h", "10min");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), DncError::InvalidTimeAcceleration);
    }

    #[test]
    fn test_calculate_dnc_invalid_night_time_acceleration() {
        let result = calculate_dnc("8h", "1min");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), DncError::InvalidNightTimeAcceleration);
    }

    #[test]
    fn test_parse_time_valid_hours() {
        assert_eq!(parse_time("8h").unwrap(), 480.0);
    }

    #[test]
    fn test_parse_time_valid_minutes() {
        assert_eq!(parse_time("10min").unwrap(), 10.0);
    }

    #[test]
    fn test_parse_time_invalid_number() {
        let result = parse_time("abc");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), DncError::InvalidTimeFormat);
    }

    #[test]
    fn test_validate_dnc_valid_values() {
        let result = validate_dnc(1.5, 48.0);
        assert_eq!(result.unwrap(), (1.5, 48.0));
    }

    #[test]
    fn test_validate_dnc_invalid_time_acceleration() {
        let result = validate_dnc(0.05, 48.0);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), DncError::InvalidTimeAcceleration);
    }

    #[test]
    fn test_validate_dnc_invalid_night_time_acceleration() {
        let result = validate_dnc(1.5, 65.0);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), DncError::InvalidNightTimeAcceleration);
    }
}
