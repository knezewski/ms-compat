# ms-compat

A Rust duration parser and formatter with JavaScript
[`ms`](https://github.com/vercel/ms)-compatible behavior.

Parse strings such as `"1.5 hours"` into milliseconds and format numeric
durations as `"2h"` or `"2 hours"`. Errors are returned as `Result` values.

## Installation

Add the package from [crates.io](https://crates.io/crates/ms-compat) to your `Cargo.toml`:

```toml
[dependencies]
ms-compat = "0.1"
```

This is a Rust library; JavaScript bindings and an npm package are not included.

## Usage

```rust
use ms_compat::{MsError, Options, format, parse};

fn main() -> Result<(), MsError> {
    let milliseconds = parse("1.5 hours")?;
    assert_eq!(milliseconds, 5_400_000.0);

    assert_eq!(format(milliseconds, Options::default())?, "2h");
    assert_eq!(format(milliseconds, Options { long: true })?, "2 hours");
    Ok(())
}
```

The `ms()` function chooses parsing or short formatting from the input type:

```rust
use ms_compat::{Ms, MsError, ms};

fn main() -> Result<(), MsError> {
    assert_eq!(ms("2h")?, 7_200_000.0);
    assert_eq!(ms(7_200_000.0)?, "2h");

    let input = String::from("30s");
    assert_eq!(ms(&input)?, 30_000.0); // Keep ownership of the string.
    assert_eq!(ms(input)?, 30_000.0);  // Consume the string.

    assert_eq!("30s".ms()?, 30_000.0);
    assert_eq!(30_000.0_f64.ms()?, "30s");
    Ok(())
}
```

## Parsing

Inputs may contain a minus sign, a decimal fraction, ASCII spaces after the
number, and an optional ASCII case-insensitive unit. A missing unit means
milliseconds. Examples: `"42"`, `"-.5 h"`, `"2 DAYS"`.

| Unit | Accepted names |
| --- | --- |
| Millisecond | `ms`, `msec`, `msecs`, `millisecond`, `milliseconds` |
| Second | `s`, `sec`, `secs`, `second`, `seconds` |
| Minute | `m`, `min`, `mins`, `minute`, `minutes` |
| Hour | `h`, `hr`, `hrs`, `hour`, `hours` |
| Day | `d`, `day`, `days` |
| Week | `w`, `week`, `weeks` |
| Month | `mo`, `month`, `months` |
| Year | `y`, `yr`, `yrs`, `year`, `years` |

Leading whitespace, whitespace after a unit, a leading plus, exponent
notation, and a trailing decimal point are rejected. Only ordinary ASCII
spaces are accepted between the number and unit.

## Errors

`MsError` implements `Display` and `std::error::Error`, so callers can propagate
errors with `?`.

| Error | Condition |
| --- | --- |
| `InvalidLength` | Empty input or more than 100 UTF-16 code units |
| `InvalidFormat` | Invalid numeric syntax or unsupported unit |
| `NotFinite` | Formatting NaN or either infinity |

Unlike the JS API, this crate reports invalid formats with an error instead
of `NaN`, and invalid lengths or non-finite numbers with an error instead of
throwing an exception.

## Formatting and limits

- Formatting selects one unit, from years down to seconds, using the absolute
  input. It preserves fractional milliseconds for magnitudes below one second.
- Rounding follows `Math.round`: ties go toward positive infinity. Negative
  one and a half days becomes `"-1d"`. In long form it becomes `"-1 days"`,
  because pluralization uses the absolute duration before rounding.
- Numeric strings use ECMAScript conversion, including `-0` becoming `0` and
  exponential notation for very small or very large nonzero values.
- A day is exactly 24 hours, a year 365.25 days, and a month 30.4375 days.
  These units do not account for calendar dates, daylight saving time, or leap seconds.
- Values use `f64`, with its finite precision. Formatting is lossy:
  `90_000.0` milliseconds becomes `"2m"`. Do not use the formatted output for
  lossless storage or assume `parse(format(value))` returns the original value.

## Verification

```sh
cargo test --locked
cargo clippy --all-targets -- -W clippy::pedantic -D warnings
cargo fmt --check
```

The separate compatibility test requires Node.js and runs without npm packages
or network access:

```sh
cargo test --locked --test js_compat -- --ignored --nocapture
```

It compares 100,793 numeric inputs in both formats and 20,017 parse inputs
against a vendored JS reference, using a fixed seed. The test is ignored by
default so that ordinary Rust tests do not require Node.js.
See [the compatibility test documentation](tests/js/README.md) for the pinned
upstream commit, protocol, and update procedure.

## License

MIT; see [LICENSE](LICENSE). The JS reference is derived from
[`vercel/ms`](https://github.com/vercel/ms) and retains its MIT license in
[tests/js/LICENSE.ms](tests/js/LICENSE.ms).
