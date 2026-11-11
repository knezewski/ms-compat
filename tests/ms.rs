use ms_compat::{Ms, MsError, ms};

#[test]
fn ms_dispatches_by_input_type() {
    let parsed: Result<f64, MsError> = ms("1.5h");
    let formatted: Result<String, MsError> = ms(5_400_000.0);

    assert_eq!(parsed, Ok(5_400_000.0));
    assert_eq!(formatted, Ok("2h".into()));
    assert_eq!("-2m".ms(), Ok(-120_000.0));
    assert_eq!(120_000.0_f64.ms(), Ok("2m".into()));
}

#[test]
fn ms_preserves_errors() {
    assert_eq!(ms(""), Err(MsError::InvalidLength));
    assert_eq!(ms("invalid"), Err(MsError::InvalidFormat));
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(ms(value), Err(MsError::NotFinite));
    }
}

#[test]
fn ms_accepts_owned_and_borrowed_strings() {
    let input = String::from("1.5h");
    assert_eq!(ms(&input), Ok(5_400_000.0));
    assert_eq!(Ms::ms(&input), Ok(5_400_000.0));
    assert_eq!(input, "1.5h"); // Borrowed conversions leave the input available.
    assert_eq!(ms(input), Ok(5_400_000.0));
    assert_eq!(String::from("-2m").ms(), Ok(-120_000.0));
}

#[test]
fn string_inputs_preserve_parse_errors() {
    for (input, expected) in [
        (String::new(), MsError::InvalidLength),
        (String::from("oops"), MsError::InvalidFormat),
        ("1".repeat(101), MsError::InvalidLength),
    ] {
        assert_eq!(ms(&input).as_ref(), Err(&expected));
        assert_eq!(ms(input), Err(expected));
    }
}

#[test]
fn errors_propagate_into_standard_error_types() {
    fn parse_duration(input: &str) -> Result<f64, Box<dyn std::error::Error>> {
        Ok(ms(input)?)
    }

    for (input, expected) in [
        ("", MsError::InvalidLength),
        ("oops", MsError::InvalidFormat),
    ] {
        let error = parse_duration(input).unwrap_err();
        assert_eq!(error.downcast_ref::<MsError>(), Some(&expected));
        assert_ne!(error.to_string(), "");
    }
    let error: &dyn std::error::Error = &MsError::NotFinite;
    assert_eq!(error.to_string(), "duration must be finite");
    assert!(error.source().is_none());
}
