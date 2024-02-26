// [3.1 Start-Tags, End-Tags, and Empty-Element Tags](https://www.w3.org/TR/2006/REC-xml11-20060816/#sec-starttags)

use crate::{
    char,
    cursor::Span,
    diagnostics::SvgParseError,
    file_reader::FileReader,
    syntactic_constructs::{literal, whitespace, Literal, LiteralValue, Name},
    Cursor,
};
use std::iter::Peekable;

#[derive(Debug, Clone, PartialEq)]
pub struct Attribute {
    name: Name,
    value: LiteralValue,
}

pub fn attributes(file_reader: &mut FileReader) -> Result<Vec<Attribute>, Box<SvgParseError>> {
    let mut entries: Vec<Attribute> = Vec::new();

    loop {
        match file_reader.peek() {
            Some('/') => break,
            Some('>') => break,
            None => break,
            _ => {}
        };
        let attr = attribute(file_reader)?;
        entries.push(attr);
    }

    Ok(entries)
}

pub fn attribute(file_reader: &mut FileReader) -> Result<Attribute, Box<SvgParseError>> {
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
    assert_eq!(
        attributes(&mut single),
        Ok(vec![Attribute {
            name: "fill".into(),
            value: LiteralValue::new(Cursor::default().advance_by(5), "'none'".into())
        }])
    );

    let mut double =
        FileReader::new("d=\"M12 3c7.2-1\"  transform=\"matrix('5.2 0 0 0 -1.1 1')\"  />");
    assert_eq!(
        attributes(&mut double),
        Ok(vec![
            Attribute {
                name: "d".into(),
                value: LiteralValue::new(Cursor::default().advance_by(2), "\"M12 3c7.2-1\"".into())
            },
            Attribute {
                name: "transform".into(),
                value: LiteralValue::new(
                    Cursor::default(),
                    "\"matrix('5.2 0 0 0 -1.1 1')\"".into()
                )
            }
        ])
    );
}
