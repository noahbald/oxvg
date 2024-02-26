use miette::SourceOffset;
use std::{iter::Peekable, str::Chars};

use crate::cursor::Cursor;

/// A sax style parser for XML written for SVG
pub struct FileReader<'a> {
    file: &'a str,
    peekable: Peekable<Chars<'a>>,
    cursor: Cursor,
    offset: usize,
}

impl<'a> FileReader<'a> {
    pub fn new(file: &'a str) -> Self {
        FileReader {
            file,
            peekable: file.chars().peekable(),
            cursor: Cursor::default(),
            offset: 0,
        }
    }

    pub fn peek(&mut self) -> Option<&char> {
        self.peekable.peek()
    }

    pub fn get_cursor(&self) -> Cursor {
        self.cursor.clone()
    }
}

impl<'a> Iterator for FileReader<'a> {
    type Item = char;

    fn next(&mut self) -> Option<char> {
        let char = self.peekable.next();

        self.offset += 1;
        if Some('\n') == char {
            self.cursor.mut_newline();
        } else {
            self.cursor.mut_advance();
        }

        char
    }
}
