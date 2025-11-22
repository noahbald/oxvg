//! Primitives for parsing XML values

use error::Error;
mod types;

pub mod error;

/// A parser containing state for the active parsing of an SVG value
pub struct Parser<'input> {
    input: &'input str,
    cursor: usize,
}

impl<'input> Parser<'input> {
    /// Create a new parser with the input
    pub fn new(input: &'input str) -> Self {
        Self { input, cursor: 0 }
    }

    /// Returns the current position in the input being read
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Try reading the next value
    ///
    /// # Errors
    ///
    /// If the input has ended
    pub fn read(&mut self) -> Result<char, Error<'input>> {
        let current = self.current()?;
        self.advance();
        Ok(current)
    }

    /// Go to the next value without reading
    pub fn advance(&mut self) {
        self.cursor += 1;
    }

    /// Move backwards without reading
    pub fn rewind(&mut self, n: usize) {
        self.cursor -= n;
    }

    /// Skip remaining input
    pub fn done(&mut self) {
        self.cursor = self.input.len();
    }

    /// Try parsing a portion of the input, reverting to the original state if failed
    ///
    /// # Errors
    ///
    /// If the attempted parsing fails
    pub fn try_parse<T, E, F: FnOnce(&mut Self) -> Result<T, E>>(&mut self, f: F) -> Result<T, E> {
        let cursor = self.cursor;
        let result = f(self);
        if result.is_err() {
            self.cursor = cursor;
        }
        result
    }

    /// Get remaining slice of input
    pub fn slice(&self) -> &'input str {
        &self.input[self.cursor..]
    }

    /// Get remaining slice of input and advance to the end of the input
    pub fn take_slice(&mut self) -> &'input str {
        let slice = &self.input[self.cursor..];
        self.done();
        slice
    }

    /// Get slice from start position to current position
    pub fn slice_from(&self, start: usize) -> &'input str {
        let end = self.cursor.min(self.input.len());
        &self.input[start..end]
    }

    /// Get the length of the remaining input
    pub fn len(&self) -> usize {
        self.input.len() - self.cursor
    }

    /// Returns whether the remaining input is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Gets the current character of the input
    ///
    /// # Errors
    ///
    /// If reached the end of input
    pub fn current(&self) -> Result<char, Error<'input>> {
        self.slice().chars().next().ok_or(Error::EndOfInput)
    }

    /// Move the cursor forward while the characters match the given predicate
    ///
    /// Returns the skipped content as a slice
    pub fn take_matches<F: FnMut(char) -> bool>(&mut self, f: F) -> &'input str {
        let cursor = self.cursor();
        self.skip_matches(f);
        let result = self.slice_from(cursor);
        result
    }

    /// Moves the cursor forward the number of matching characters
    pub fn skip_matches<F: FnMut(char) -> bool>(&mut self, pat: F) {
        self.skip_internal(self.slice().trim_start_matches(pat).len());
    }

    /// Moves the cursor forward the number of matching characters
    pub fn skip_char(&mut self, char: char) {
        self.skip_internal(self.slice().trim_matches(char).len());
    }

    /// Moves the cursor forward the number of whitespace characters
    pub fn skip_whitespace(&mut self) {
        self.skip_matches(char::is_whitespace);
    }

    fn skip_internal(&mut self, trim: usize) {
        let offset = self.len() - trim;
        self.cursor += offset;
    }

    /// Asserts the end of the input was reached
    ///
    /// # Errors
    ///
    /// When the cursor is prior to the end of the string
    pub fn expect_done(&self) -> Result<(), Error<'input>> {
        if self.cursor < self.input.len() {
            Err(Error::ExpectedDone)
        } else {
            Ok(())
        }
    }

    /// Read and assert the next character matches the expected character
    ///
    /// # Errors
    ///
    /// If the end of the input is reached, or the character does not match
    pub fn expect_char(&mut self, expected: char) -> Result<(), Error<'input>> {
        let received = self.read()?;
        if received == expected {
            Ok(())
        } else {
            Err(Error::ExpectedChar { expected, received })
        }
    }

    /// Read and assert a set of characters matches the expected pattern.
    ///
    /// # Errors
    ///
    /// - If the end of the input is reached
    /// - If none of the characters match the expected pattern
    /// - If the patterns matcher asserts an error
    pub fn expect_matches<F: Fn(char) -> Result<bool, &'static str>>(
        &mut self,
        expected: &'static str,
        f: F,
    ) -> Result<&'input str, Error<'input>> {
        let cursor = self.cursor;
        let mut result = Ok(());
        self.skip_matches(|char| match f(char) {
            Ok(bool) => bool,
            Err(expected) => {
                result = Err(expected);
                false
            }
        });
        match result {
            Ok(()) => match self.slice_from(cursor) {
                "" => Err(Error::ExpectedMatch {
                    expected,
                    received: "nothing",
                }),
                result => Ok(result),
            },
            Err(expected) => Err(Error::ExpectedMatch {
                expected,
                received: self.slice_from(cursor),
            }),
        }
    }

    /// Read and assert a set of characters is whitespace
    ///
    /// # Errors
    ///
    /// If none of the next characters are whitespace
    pub fn expect_whitespace(&mut self) -> Result<(), Error<'input>> {
        self.expect_matches("a whitespace character", |char| Ok(char.is_whitespace()))?;
        Ok(())
    }

    /// Read and assert a set of characters matches the given string
    ///
    /// # Errors
    ///
    /// If the next set of characters does not match the given string
    pub fn expect_str(&mut self, expected: &'static str) -> Result<(), Error<'input>> {
        let cursor = self.cursor;
        self.cursor += expected.len();
        let received = self.slice_from(cursor);
        if received == expected {
            Ok(())
        } else {
            Err(Error::ExpectedString { expected, received })
        }
    }

    /// Read and assert a set of characters matches some ident
    ///
    /// # Errors
    ///
    /// When an invalid ident is received
    pub fn expect_ident(&mut self) -> Result<&'input str, Error<'input>> {
        let cursor = self.cursor;
        let name_start_char = self.read()?;
        if !is_name_start_char(name_start_char) {
            return Err(Error::ExpectedIdent {
                expected: "valid ident starting character",
                received: self.slice_from(cursor),
            });
        }
        self.skip_matches(is_name_char);
        Ok(self.slice_from(cursor))
    }

    /// Read and assert a set of characters matches the given identifier
    ///
    /// # Errors
    ///
    /// If the next set of characters does not match the given identifier
    pub fn expect_ident_matching(&mut self, expected: &'static str) -> Result<(), Error<'input>> {
        let received = self.expect_ident()?;
        if received == expected {
            Ok(())
        } else {
            Err(Error::ExpectedIdent { expected, received })
        }
    }
}
fn is_name_start_char(char: char) -> bool {
    char.is_ascii_alphabetic()
        || matches!(char, ':' | '_' | '\u{C0}'..'\u{D6}' | '\u{D8}'..='\u{F6}' | '\u{F8}'..='\u{2FF}' | '\u{370}'..='\u{37D}' | '\u{37F}'..='\u{1FFF}' | '\u{200C}'..='\u{200D}' | '\u{2070}'..='\u{218F}' | '\u{2C00}'..='\u{2FEF}' | '\u{3001}'..='\u{D7FF}' | '\u{F900}'..='\u{FDCF}' | '\u{FDF0}'..='\u{FFFD}' | '\u{10000}'..='\u{EFFFF}')
}
fn is_name_char(char: char) -> bool {
    is_name_start_char(char)
        || char.is_ascii_digit()
        || matches!(char, '-' | '.' | '\u{B7}' | '\u{0300}'..='\u{036F}' | '\u{203F}'..='\u{2040}')
}

/// A trait for things that can be parsed from CSS or attribute values.
pub trait Parse<'input>: Sized {
    /// Parse this value using an existing parser.
    ///
    /// # Errors
    /// If parsing fails
    fn parse(input: &mut Parser<'input>) -> Result<Self, Error<'input>>;

    /// Parse a value from a string
    ///
    /// # Errors
    /// If parsing fails
    fn parse_string(input: &'input str) -> Result<Self, Error<'input>> {
        let parser = &mut Parser::new(input);
        parser.skip_whitespace();
        let result = Self::parse(parser)?;
        parser.skip_whitespace();
        parser.expect_done()?;
        Ok(result)
    }
}
