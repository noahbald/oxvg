// [3.1 Start-Tags, End-Tags, and Empty-Element Tags](https://www.w3.org/TR/2006/REC-xml11-20060816/#sec-starttags)

use crate::{
    char,
    cursor::Span,
    diagnostics::SvgParseError,
    syntactic_constructs::{literal, whitespace, Literal, LiteralValue, Name},
    Cursor,
};
use std::iter::Peekable;

#[derive(Debug, Clone, PartialEq)]
pub struct Attribute {
    name: Name,
    value: LiteralValue,
}

pub fn attributes(
    partial: &mut Peekable<impl Iterator<Item = char>>,
    cursor: Cursor,
) -> Result<(Cursor, Vec<Attribute>), Box<SvgParseError>> {
    let mut entries: Vec<Attribute> = Vec::new();
    let mut cursor_end = cursor;

    loop {
        match partial.peek() {
            Some('/') => break,
            Some('>') => break,
            None => break,
            _ => {}
        };
        let (c, attr) = attribute(partial, cursor)?;
        cursor_end = c;
        entries.push(attr);
    }

    Ok((cursor_end, entries))
}

pub fn attribute(
    partial: &mut Peekable<impl Iterator<Item = char>>,
    cursor: Cursor,
) -> Result<(Cursor, Attribute), Box<SvgParseError>> {
    // [41]
    let (cursor, name) = Name::new(partial, cursor)?;
    let cursor = char(partial, cursor, Some('='))?;
    let (cursor, value) = literal(partial, cursor, Literal::AttValue)?;
    let cursor = whitespace(partial, cursor, false)?;
    Ok((cursor, Attribute { name, value }))
}

#[test]
fn test_attributes() {
    let mut single = "fill='none'".chars().peekable();
    assert_eq!(
        attributes(&mut single, Cursor::default()),
        Ok((
            Cursor::default().advance_by(11),
            vec![Attribute {
                name: "fill".into(),
                value: LiteralValue::new(Cursor::default().advance_by(5), "'none'".into())
            }]
        ))
    );

    let mut double = "d=\"M12 3c7.2-1\"  transform=\"matrix('5.2 0 0 0 -1.1 1')\"  />"
        .chars()
        .peekable();
    assert_eq!(
        attributes(&mut double, Cursor::default()),
        Ok((
            Cursor::default().advance_by(47),
            vec![
                Attribute {
                    name: "d".into(),
                    value: LiteralValue::new(
                        Cursor::default().advance_by(2),
                        "\"M12 3c7.2-1\"".into()
                    )
                },
                Attribute {
                    name: "transform".into(),
                    value: LiteralValue::new(
                        Cursor::default(),
                        "\"matrix('5.2 0 0 0 -1.1 1')\"".into()
                    )
                }
            ]
        ))
    );
}
