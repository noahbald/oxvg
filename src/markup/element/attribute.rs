// [3.1 Start-Tags, End-Tags, and Empty-Element Tags](https://www.w3.org/TR/2006/REC-xml11-20060816/#sec-starttags)

use crate::{
    char,
    cursor::Span,
    diagnostics::SVGError,
    file_reader::FileReader,
    syntactic_constructs::{literal, whitespace, Literal, LiteralValue, Name},
    Cursor,
};
use std::{collections::HashMap, iter::Peekable};

#[derive(Debug, Clone, PartialEq)]
pub struct Attribute {
    name: String,
    value: String,
}

#[derive(Default, Debug, Clone)]
pub struct Attributes {
    pub map: HashMap<String, String>,
}

impl Attributes {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub fn insert(&mut self, key: String, value: String) {
        self.map.insert(key, value);
    }
}

pub fn attributes(file_reader: &mut FileReader) -> Result<HashMap<String, String>, Box<SVGError>> {
    let mut entries = HashMap::new();

    loop {
        match file_reader.peek() {
            Some('/') => break,
            Some('>') => break,
            None => break,
            _ => {}
        };
        let attr = attribute(file_reader)?;
        entries.insert(attr.name, attr.value);
    }

    Ok(entries)
}

pub fn attribute(file_reader: &mut FileReader) -> Result<Attribute, Box<SVGError>> {
    // [41]
    let name = Name::new(file_reader)?;
    char(file_reader, Some('='))?;
    let value = literal(file_reader, Literal::AttValue)?;
    whitespace(file_reader, false)?;
    Ok(Attribute { name, value })
}

#[test]
fn test_attributes() {
    let mut single = FileReader::new("fill='none'");
    dbg!(attributes(&mut single),);

    let mut double =
        FileReader::new("d=\"M12 3c7.2-1\"  transform=\"matrix('5.2 0 0 0 -1.1 1')\"  />");
    dbg!(attributes(&mut double),);
}
