//! Parse duration strings into milliseconds and format milliseconds for display.
//!
//! [`parse`] and [`format()`] are the explicit APIs. [`ms()`] and [`Ms::ms`]
//! select parsing or short formatting from the input type.
//! Years and months have fixed lengths; these functions do not perform calendar arithmetic.
//!
//! ```
//! use ms_compat::{Options, format, parse};
//!
//! assert_eq!(parse("1.5 hours"), Ok(5_400_000.0));
//! assert_eq!(format(5_400_000.0, Options::default()), Ok("2h".into()));
//! assert_eq!(format(5_400_000.0, Options { long: true }), Ok("2 hours".into()));
//! ```

use regex::Regex;
use std::{fmt, sync::LazyLock};

const S: f64 = 1000.0;
const M: f64 = S * 60.0;
const H: f64 = M * 60.0;
const D: f64 = H * 24.0;
const W: f64 = D * 7.0;
const Y: f64 = D * 365.25;
const MO: f64 = Y / 12.0;

#[derive(Debug, PartialEq)]
/// An invalid duration string or a non-finite numeric duration.
pub enum MsError {
    /// The input string is empty or longer than 100 UTF-16 code units.
    InvalidLength,
    /// The string has invalid syntax or an unsupported unit.
    InvalidFormat,
    /// The numeric duration is NaN or positive or negative infinity.
    NotFinite,
}

impl fmt::Display for MsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidLength => "duration must contain between 1 and 100 UTF-16 code units",
            Self::InvalidFormat => "invalid duration format",
            Self::NotFinite => "duration must be finite",
        })
    }
}

impl std::error::Error for MsError {}

#[derive(Debug, Default, Clone, Copy)]
/// Controls the output of [`format()`]. The default uses short unit names.
pub struct Options {
    /// Use English unit names, such as `"2 hours"`, instead of `"2h"`.
    pub long: bool,
}

/// Converts strings to milliseconds and numbers to short duration strings.
///
/// Implemented for `&str`, `String`, `&String`, and `f64`.
/// Owned strings are consumed; borrowed strings remain available to the caller.
/// Use [`format()`] directly to select long output.
pub trait Ms {
    /// `Result<f64, MsError>` for strings or `Result<String, MsError>` for `f64`.
    type Out;

    /// Parses a string with [`parse`] or formats a number with default [`Options`].
    fn ms(self) -> Self::Out;
}

impl Ms for &str {
    type Out = Result<f64, MsError>;

    fn ms(self) -> Self::Out {
        parse(self)
    }
}

impl Ms for String {
    type Out = Result<f64, MsError>;

    fn ms(self) -> Self::Out {
        parse(&self)
    }
}

impl Ms for &String {
    type Out = Result<f64, MsError>;

    fn ms(self) -> Self::Out {
        parse(self)
    }
}

impl Ms for f64 {
    type Out = Result<String, MsError>;

    fn ms(self) -> Self::Out {
        format(self, Options::default())
    }
}

/// Converts a duration using the implementation for its input type.
///
/// Accepts `&str`, `String`, `&String`, or `f64`. String inputs return
/// `Result<f64, MsError>`; numbers return `Result<String, MsError>` in short form.
/// Passing a `String` transfers ownership; pass `&String` to retain it.
///
/// # Errors
///
/// Returns the same errors as [`parse`] for strings or [`format()`] for numbers.
///
/// # Examples
///
/// ```
/// use ms_compat::{Ms, ms};
///
/// assert_eq!(ms("2h"), Ok(7_200_000.0));
/// assert_eq!(ms(7_200_000.0), Ok("2h".to_owned()));
/// assert_eq!("2h".ms(), Ok(7_200_000.0));
/// let input = String::from("2h");
/// assert_eq!(ms(&input), Ok(7_200_000.0));
/// assert_eq!(ms(input), Ok(7_200_000.0));
/// assert_eq!(String::from("2h").ms(), Ok(7_200_000.0));
/// ```
pub fn ms<T: Ms>(value: T) -> T::Out {
    value.ms()
}

static RE: LazyLock<Regex> = LazyLock::new(|| {
    // Validate syntax here; the match in parse is the source of accepted units.
    Regex::new(r"(?i-u)^(?<value>-?[0-9]*\.?[0-9]+) *(?<unit>[a-z]+)?$")
        .expect("duration regex must be valid")
});

/// Parses a duration into milliseconds, using milliseconds when no unit is given.
///
/// Numbers may have a leading minus and a fractional part, including `.5`.
/// A leading plus, exponent notation, and a trailing decimal point are rejected.
/// Zero or more ASCII spaces may follow the number before the optional unit.
/// Leading whitespace and whitespace after a unit are rejected.
///
/// Units are ASCII case-insensitive:
///
/// | Unit | Accepted names |
/// | --- | --- |
/// | Millisecond | `ms`, `msec`, `msecs`, `millisecond`, `milliseconds` |
/// | Second | `s`, `sec`, `secs`, `second`, `seconds` |
/// | Minute | `m`, `min`, `mins`, `minute`, `minutes` |
/// | Hour | `h`, `hr`, `hrs`, `hour`, `hours` |
/// | Day | `d`, `day`, `days` |
/// | Week | `w`, `week`, `weeks` |
/// | Month | `mo`, `month`, `months` |
/// | Year | `y`, `yr`, `yrs`, `year`, `years` |
///
/// A day is 24 hours, a year is 365.25 days, and a month is one twelfth
/// of a year (30.4375 days). These are fixed durations, independent of dates.
///
/// # Errors
///
/// Returns [`MsError::InvalidLength`] for empty input or input longer than 100
/// UTF-16 code units, and [`MsError::InvalidFormat`] for invalid syntax or units.
///
/// # Examples
///
/// ```
/// use ms_compat::{MsError, parse};
///
/// assert_eq!(parse("-.5 H"), Ok(-1_800_000.0));
/// assert_eq!(parse("42"), Ok(42.0));
/// assert_eq!(parse("1e3ms"), Err(MsError::InvalidFormat));
/// ```
pub fn parse(s: &str) -> Result<f64, MsError> {
    // JS считает длину в UTF-16, а не в байтах
    let len = s.encode_utf16().take(101).count();
    if len == 0 || len > 100 {
        return Err(MsError::InvalidLength);
    }

    let caps = RE.captures(s).ok_or(MsError::InvalidFormat)?;
    let n: f64 = caps["value"].parse().map_err(|_| MsError::InvalidFormat)?;
    let unit = caps
        .name("unit")
        .map_or("ms", |m| m.as_str())
        .to_ascii_lowercase();

    let k = match unit.as_str() {
        "years" | "year" | "yrs" | "yr" | "y" => Y,
        "months" | "month" | "mo" => MO,
        "weeks" | "week" | "w" => W,
        "days" | "day" | "d" => D,
        "hours" | "hour" | "hrs" | "hr" | "h" => H,
        "minutes" | "minute" | "mins" | "min" | "m" => M,
        "seconds" | "second" | "secs" | "sec" | "s" => S,
        "milliseconds" | "millisecond" | "msecs" | "msec" | "ms" => 1.0,
        _ => return Err(MsError::InvalidFormat),
    };
    Ok(n * k)
}

/// Formats milliseconds as one rounded duration, such as `"2h"` or `"2 hours"`.
///
/// Selects the largest unit not exceeding the absolute input, from years down
/// to seconds. Years and months use the fixed lengths documented in [`parse`].
/// Values smaller than one second retain their fractional milliseconds.
///
/// Rounding follows JavaScript `Math.round`: ties go toward positive infinity.
/// Number strings follow ECMAScript rules, including `-0` becoming `0`, and
/// exponential notation below `1e-6` or at and above `1e21` in magnitude.
/// These thresholds apply to the number after unit conversion and rounding.
/// Long-form pluralization uses the absolute input before rounding; consequently,
/// negative one and a half days becomes `"-1 days"`, matching the JS behavior.
/// The result is intended for display; rounding makes it unsuitable for lossless storage.
///
/// # Errors
///
/// Returns [`MsError::NotFinite`] for NaN or either infinity.
///
/// # Examples
///
/// ```
/// use ms_compat::{Options, format};
///
/// assert_eq!(format(-129_600_000.0, Options::default()), Ok("-1d".into()));
/// assert_eq!(format(1e-7, Options { long: true }), Ok("1e-7 ms".into()));
/// ```
pub fn format(ms: f64, options: Options) -> Result<String, MsError> {
    if !ms.is_finite() {
        return Err(MsError::NotFinite);
    }
    Ok(if options.long {
        fmt_long(ms)
    } else {
        fmt_short(ms)
    })
}

/// Math.round из JS: половинки округляются вверх (-2.5 -> -2)
fn js_round(x: f64) -> f64 {
    // Adding 0.5 first loses precision near half-integers and for large values.
    // Truncation also preserves the negative zero required by Math.round.
    let integer = x.trunc();
    let fraction = x - integer;
    if fraction >= 0.5 {
        integer + 1.0
    } else if fraction < -0.5 {
        integer - 1.0
    } else {
        integer
    }
}

const STEPS: [(f64, &str, &str); 7] = [
    (Y, "y", "year"),
    (MO, "mo", "month"),
    (W, "w", "week"),
    (D, "d", "day"),
    (H, "h", "hour"),
    (M, "m", "minute"),
    (S, "s", "second"),
];

fn fmt_short(ms: f64) -> String {
    let mut buffer = ryu_js::Buffer::new();
    let abs = ms.abs();
    for (n, short, _) in STEPS {
        if abs >= n {
            return format!("{}{}", buffer.format_finite(js_round(ms / n)), short);
        }
    }
    format!("{}ms", buffer.format_finite(ms))
}

fn fmt_long(ms: f64) -> String {
    let abs = ms.abs();
    for (n, _, long) in STEPS {
        if abs >= n {
            return plural(ms, abs, n, long);
        }
    }
    let mut buffer = ryu_js::Buffer::new();
    format!("{} ms", buffer.format_finite(ms))
}

fn plural(ms: f64, abs: f64, n: f64, name: &str) -> String {
    let suffix = if abs >= n * 1.5 { "s" } else { "" };
    let mut buffer = ryu_js::Buffer::new();
    format!(
        "{} {}{}",
        buffer.format_finite(js_round(ms / n)),
        name,
        suffix
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid() {
        let cases = [
            ("100", 100.0),
            ("1m", 60_000.0),
            ("1h", 3_600_000.0),
            ("2d", 172_800_000.0),
            ("3w", 1_814_400_000.0),
            ("1s", 1000.0),
            ("100ms", 100.0),
            ("1y", 31_557_600_000.0),
            ("1.5h", 5_400_000.0),
            ("1   s", 1000.0),
            (".5ms", 0.5),
            ("-100ms", -100.0),
            ("-1.5h", -5_400_000.0),
            ("-10.5h", -37_800_000.0),
            ("-.5h", -1_800_000.0),
            ("53 YeArS", 1_672_552_800_000.0),
            ("53 MiLliSeCondS", 53.0),
            ("2 days", 172_800_000.0),
            ("1 month", 2_629_800_000.0),
            ("-.5 hr", -1_800_000.0),
        ];
        for (input, expected) in cases {
            assert_eq!(parse(input), Ok(expected), "parse({input:?})");
        }
    }

    #[test]
    fn parse_invalid() {
        for input in ["☃", "10-.5", "foo"] {
            assert_eq!(
                parse(input),
                Err(MsError::InvalidFormat),
                "parse({input:?})"
            );
        }
        assert_eq!(parse(""), Err(MsError::InvalidLength));
        assert_eq!(parse(&"▲".repeat(101)), Err(MsError::InvalidLength));
    }

    #[test]
    fn parse_rejects_unicode_units_and_unknown_aliases() {
        for input in ["1ſ", "1ſecond", "1mſ", "1µs", "1fortnight", "1monthss"] {
            assert_eq!(parse(input), Err(MsError::InvalidFormat), "{input:?}");
        }
    }

    #[test]
    fn parse_supports_all_ascii_unit_aliases() {
        let cases: &[(&[&str], f64)] = &[
            (&["milliseconds", "millisecond", "msecs", "msec", "ms"], 1.0),
            (&["seconds", "second", "secs", "sec", "s"], 1_000.0),
            (&["minutes", "minute", "mins", "min", "m"], 60_000.0),
            (&["hours", "hour", "hrs", "hr", "h"], 3_600_000.0),
            (&["days", "day", "d"], 86_400_000.0),
            (&["weeks", "week", "w"], 604_800_000.0),
            (&["months", "month", "mo"], 2_629_800_000.0),
            (&["years", "year", "yrs", "yr", "y"], 31_557_600_000.0),
        ];
        for &(aliases, expected) in cases {
            for alias in aliases {
                for unit in [alias.to_string(), alias.to_ascii_uppercase()] {
                    let input = format!("1 {unit}");
                    assert_eq!(parse(&input), Ok(expected), "{input:?}");
                }
            }
        }
    }

    #[test]
    fn parse_length_limit_uses_utf16() {
        assert_eq!(parse(&format!("1{}ms", " ".repeat(97))), Ok(1.0));
        assert_eq!(
            parse(&format!("1{}ms", " ".repeat(98))),
            Err(MsError::InvalidLength)
        );
        // These are 100 and 102 UTF-16 code units, respectively.
        assert_eq!(parse(&"😀".repeat(50)), Err(MsError::InvalidFormat));
        assert_eq!(parse(&"😀".repeat(51)), Err(MsError::InvalidLength));
    }

    #[test]
    fn js_round_matches_javascript_at_float_boundaries() {
        let cases = [
            (0.499_999_999_999_999_94, 0.0),
            (0.5, 1.0),
            (-0.5, -0.0),
            (-0.500_000_000_000_000_1, -1.0),
            (1.499_999_999_999_999_8, 1.0),
            (1.5, 2.0),
            (-1.5, -1.0),
            (-1.500_000_000_000_000_2, -2.0),
            (4_503_599_627_370_497.0, 4_503_599_627_370_497.0),
            (-4_503_599_627_370_497.0, -4_503_599_627_370_497.0),
            (f64::MAX, f64::MAX),
            (-f64::MAX, -f64::MAX),
            (0.0, 0.0),
            (-0.0, -0.0),
            (-0.1, -0.0),
        ];
        for (input, expected) in cases {
            assert_eq!(js_round(input).to_bits(), expected.to_bits(), "{input}");
        }
    }

    #[test]
    fn format_normalizes_negative_zero() {
        assert_eq!(format(-0.0, Options::default()), Ok("0ms".into()));
        assert_eq!(format(-0.0, Options { long: true }), Ok("0 ms".into()));
        assert_eq!(format(-0.1, Options::default()), Ok("-0.1ms".into()));
    }

    #[test]
    fn format_numbers_like_javascript() {
        // Expected output checked with Node.js Number string conversion and Math.round.
        let cases = [
            (1e-7, "1e-7ms", "1e-7 ms"),
            (1e-6, "0.000001ms", "0.000001 ms"),
            (
                9.999_999_999_999_997e-7,
                "9.999999999999997e-7ms",
                "9.999999999999997e-7 ms",
            ),
            (f64::from_bits(1), "5e-324ms", "5e-324 ms"),
            (
                Y * 1e20,
                "100000000000000000000y",
                "100000000000000000000 years",
            ),
            (Y * 1e21, "1e+21y", "1e+21 years"),
            (
                Y * 1_000_000_000_000_000_100.0,
                "1000000000000000100y",
                "1000000000000000100 years",
            ),
            (
                f64::MAX,
                "5.696545792019405e+297y",
                "5.696545792019405e+297 years",
            ),
        ];
        for (input, short, long) in cases {
            for (options, expected) in [(Options::default(), short), (Options { long: true }, long)]
            {
                assert_eq!(format(input, options), Ok(expected.into()), "{input}");
                assert_eq!(
                    format(-input, options),
                    Ok(format!("-{expected}")),
                    "-{input}"
                );
            }
        }
    }

    #[test]
    fn format_short_and_long() {
        let short = Options::default();
        let long = Options { long: true };
        let cases = [
            (500.0, "500ms", "500 ms"),
            (-500.0, "-500ms", "-500 ms"),
            (1000.0, "1s", "1 second"),
            (1200.0, "1s", "1 second"),
            (10000.0, "10s", "10 seconds"),
            (-1000.0, "-1s", "-1 second"),
            (-1200.0, "-1s", "-1 second"),
            (-10000.0, "-10s", "-10 seconds"),
            (60.0 * 1000.0, "1m", "1 minute"),
            (1.2 * M, "1m", "1 minute"),
            (10.0 * M, "10m", "10 minutes"),
            (-M, "-1m", "-1 minute"),
            (-1.2 * M, "-1m", "-1 minute"),
            (-10.0 * M, "-10m", "-10 minutes"),
            (H, "1h", "1 hour"),
            (1.2 * H, "1h", "1 hour"),
            (10.0 * H, "10h", "10 hours"),
            (-H, "-1h", "-1 hour"),
            (-1.2 * H, "-1h", "-1 hour"),
            (-10.0 * H, "-10h", "-10 hours"),
            (D, "1d", "1 day"),
            (1.2 * D, "1d", "1 day"),
            (6.0 * D, "6d", "6 days"),
            (-D, "-1d", "-1 day"),
            (-1.2 * D, "-1d", "-1 day"),
            (-6.0 * D, "-6d", "-6 days"),
            (W, "1w", "1 week"),
            (2.0 * 7.0 * 24.0 * 60.0 * 60.0 * 1000.0, "2w", "2 weeks"),
            (-W, "-1w", "-1 week"),
            (-2.0 * W, "-2w", "-2 weeks"),
            (30.4375 * 24.0 * 60.0 * 60.0 * 1000.0, "1mo", "1 month"),
            (1.2 * MO, "1mo", "1 month"),
            (10.0 * MO, "10mo", "10 months"),
            (-MO, "-1mo", "-1 month"),
            (-1.2 * MO, "-1mo", "-1 month"),
            (-10.0 * MO, "-10mo", "-10 months"),
            (365.25 * 24.0 * 60.0 * 60.0 * 1000.0 + 1.0, "1y", "1 year"),
            (1.2 * Y + 1.0, "1y", "1 year"),
            (10.0 * Y + 1.0, "10y", "10 years"),
            (-Y - 1.0, "-1y", "-1 year"),
            (-1.2 * Y - 1.0, "-1y", "-1 year"),
            (-10.0 * Y - 1.0, "-10y", "-10 years"),
            (234_234_234.0, "3d", "3 days"),
            (-234_234_234.0, "-3d", "-3 days"),
        ];
        for (input, s, l) in cases {
            assert_eq!(format(input, short).unwrap(), s, "short {input}");
            assert_eq!(format(input, long).unwrap(), l, "long {input}");
        }
    }

    #[test]
    fn format_negative_half_day_rounds_toward_positive_infinity() {
        // JS Math.round(-1.5) == -1; Rust f64::round(-1.5) == -2.
        assert_eq!(format(-1.5 * D, Options::default()), Ok("-1d".into()));
        assert_eq!(
            format(-1.5 * D, Options { long: true }),
            Ok("-1 days".into())
        );
    }

    #[test]
    fn format_rejects_non_finite() {
        for v in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(format(v, Options::default()), Err(MsError::NotFinite));
        }
    }
}
