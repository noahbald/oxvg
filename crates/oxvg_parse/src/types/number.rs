//! Parsing for number values
use crate::{error::Error, Parse, Parser};

impl<'input> Parse<'input> for f64 {
    fn parse(input: &mut Parser<'input>) -> Result<Self, Error<'input>> {
        input.skip_whitespace();

        let cursor = input.cursor();
        input.skip_matches(|char| matches!(char, '-' | '+'));
        input.skip_matches(|char| char.is_ascii_digit());
        input.skip_char('.');
        input.skip_matches(|char| char.is_ascii_digit());

        if let Ok('e' | 'E') = input.current() {
            input.advance();
            if let Ok('x' | 'm') = input.current() {
                input.rewind(1);
            } else {
                let exponent_cursor = input.cursor();
                input.skip_matches(|char| matches!(char, '-' | '+'));
                input.skip_matches(|char| char.is_ascii_digit());
                input.skip_char('.');
                input.skip_matches(|char| char.is_ascii_digit());
                if exponent_cursor == input.cursor() {
                    return Err(Error::InvalidNumber);
                }
            }
        }

        let number: f64 = input
            .slice_from(cursor)
            .parse()
            .map_err(|_| Error::InvalidNumber)?;
        if number.is_finite() {
            Ok(number)
        } else {
            Err(Error::InvalidNumber)
        }
    }
}

impl<'input> Parse<'input> for f32 {
    fn parse(input: &mut Parser<'input>) -> Result<Self, Error<'input>> {
        Ok(f64::parse(input)? as f32)
    }
}

impl<'input> Parse<'input> for i64 {
    fn parse(input: &mut Parser<'input>) -> Result<Self, Error<'input>> {
        input.skip_whitespace();

        let cursor = input.cursor();
        input.skip_matches(|char| matches!(char, '-' | '+'));
        input.skip_matches(|char| char.is_ascii_digit());

        let number: i64 = input
            .slice_from(cursor)
            .parse()
            .map_err(|_| Error::InvalidNumber)?;
        Ok(number)
    }
}

impl<'input> Parse<'input> for i32 {
    fn parse(input: &mut Parser<'input>) -> Result<Self, Error<'input>> {
        Ok(i64::parse(input)? as i32)
    }
}

#[test]
fn float() {
    assert_eq!(f64::parse_string("0"), Ok(0.0));
    assert_eq!(f64::parse_string("1"), Ok(1.0));
    assert_eq!(f64::parse_string("-1"), Ok(-1.0));
    assert_eq!(f64::parse_string(" -1 "), Ok(-1.0));
    assert_eq!(f64::parse_string("  1  "), Ok(1.0));
    assert_eq!(f64::parse_string(".4"), Ok(0.4));
    assert_eq!(f64::parse_string("-.4"), Ok(-0.4));
    assert_eq!(f64::parse_string(".0000000000008"), Ok(0.000_000_000_000_8));
    assert_eq!(f64::parse_string("1000000000000"), Ok(1_000_000_000_000.0));
    assert_eq!(f64::parse_string("123456.123456"), Ok(123_456.123_456));
    assert_eq!(f64::parse_string("+10"), Ok(10.0));
    assert_eq!(f64::parse_string("1e2"), Ok(100.0));
    assert_eq!(f64::parse_string("1e+2"), Ok(100.0));
    assert_eq!(f64::parse_string("1E2"), Ok(100.0));
    assert_eq!(f64::parse_string("1e-2"), Ok(0.01));
    assert_eq!(
        f64::parse_string("12345678901234567890"),
        Ok(12_345_678_901_234_567_000.0)
    );
    assert_eq!(f64::parse_string("0."), Ok(0.0));
    assert_eq!(f64::parse_string("1.3e-2"), Ok(0.013));

    assert_eq!(f64::parse_string("-.4text"), Err(Error::ExpectedDone));
    assert_eq!(f64::parse_string("-.01 text"), Err(Error::ExpectedDone));
    assert_eq!(f64::parse_string("-.01 4"), Err(Error::ExpectedDone));
    assert_eq!(f64::parse_string("1ex"), Err(Error::ExpectedDone));
    assert_eq!(f64::parse_string("1em"), Err(Error::ExpectedDone));
    assert_eq!(f64::parse_string("q"), Err(Error::InvalidNumber));
    assert_eq!(f64::parse_string(""), Err(Error::InvalidNumber));
    assert_eq!(f64::parse_string("-"), Err(Error::InvalidNumber));
    assert_eq!(f64::parse_string("+"), Err(Error::InvalidNumber));
    assert_eq!(f64::parse_string("-q"), Err(Error::InvalidNumber));
    assert_eq!(f64::parse_string("."), Err(Error::InvalidNumber));
    assert_eq!(
        f64::parse_string("99999999e99999999"),
        Err(Error::InvalidNumber)
    );
    assert_eq!(
        f64::parse_string("-99999999e99999999"),
        Err(Error::InvalidNumber)
    );
}
