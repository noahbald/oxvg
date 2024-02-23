use std::iter::Peekable;

use crate::{
    cursor::Cursor, diagnostics::SvgParseError, markup::Element,
    syntactic_constructs::Name, SvgParseErrorMessage,
};

pub enum Decoration {
    Decoration,
    Declaration,
}

pub fn decoration(
    partial: &mut Peekable<impl Iterator<Item = char>>,
    cursor: Cursor,
    form: Decoration,
) -> Result<(Cursor, Element), Box<SvgParseError>> {
    // NOTE: We are not processing the body of any decorations/declarations here
    println!("Warning: decorations and declarations are ignored (ie. `<!` and `<?`)");
    let start = match partial.next() {
        Some(c) => c,
        None => Err(SvgParseError::new_curse(
            cursor,
            SvgParseErrorMessage::UnexpectedEndOfFile,
        ))?,
    };
    let text = match start {
        // [15]
        '-' if matches!(form, Decoration::Decoration) => ("<!--", "--"),
        // [19], [21]
        '[' if matches!(form, Decoration::Decoration) && partial.peek() == Some(&'C')=> ("<![CDATA[", "]]"),
        // [61]
        '[' if matches!(form, Decoration::Decoration) => ("<![", "]]"),
        // [28]
        'D' if matches!(form, Decoration::Decoration) => ("<!DOCTYPE", ""),
        // [29], [45]
        'E' if matches!(form, Decoration::Decoration) && partial.peek() == Some(&'L') => {
            ("<!ELEMENT", "")
        }
        // [29], [52]
        'A' if matches!(form, Decoration::Decoration) => ("<!ATTLIST", ""),
        // [29], [70]
        'E' if matches!(form, Decoration::Decoration) && partial.peek() == Some(&'N') => {
            ("<!ENTITY", "")
        }
        // [29], [82]
        'N' if matches!(form, Decoration::Decoration) => ("<!NOTATION", ""),
        // As per https://www.w3.org/TR/2006/REC-xml11-20060816/#NT-TextDecl
        // As per https://www.w3.org/TR/2006/REC-xml11-20060816/#NT-XMLDecl
        // [23], [33], [77]
        'X' | 'x' if matches!(form, Decoration::Declaration) => ("<?xml", "?"),
        // FIXME: This match will fail for any 'x' prefix, eg <?Xylophone ?>
        // [16]
        c if matches!(form, Decoration::Declaration) && Name::is_name_start_char(&c) => ("<?", "?"),
        c => Err(SvgParseError::new_curse( 
            cursor,
            SvgParseErrorMessage::UnexpectedChar(
                c,
                match form {
                    Decoration::Decoration => {
                        "a matching char for a comment (`<!--`), doctype (`<!DOCTYPE`), or conditional section (`<![...[`)".into()
                    }
                    Decoration::Declaration => "a matching char for `<?xml` or `<?...`".into(),
                },
            ),
        ))?,
    };
    let end: String = text.1.into();
    let mut text: String = text.0.into();
    let _ = partial.take(text.len() - 3).collect::<String>();
    let mut cursor = cursor.advance_by(text.len() - 2);

    for char in partial.by_ref() {
        // Naiively push the character, as we don't care about this content for SVGs
        text.push(char);

        cursor = cursor.advance();
        match char {
            '\n' => {
                cursor = cursor.newline();
            }
            '>' if end == text[text.len() - 2..] => break,
            _ => {}
        }
    }

    let element = match start {
        '-' => Element::Comment(text),
        '[' => Element::CData(text),
        'D' => Element::DocType(text),
        'x' => Element::XMLDeclaration(text),
        _ => Element::ProcessingInstructions(text),
    };
    Ok((cursor, element))
}

#[test]
fn test_decoration() {
    let mut comment = "-- Hello, world -->".chars().peekable();
    assert_eq!(
        decoration(&mut comment, Cursor::default(), Decoration::Decoration),
        Ok((Cursor::default().advance_by(19), Element::Comment("<!-- Hello, world -->".into())))
    );

    let mut cdata = "[CDATA[ Hello, world ]]>".chars().peekable();
    assert_eq!(
        decoration(&mut cdata, Cursor::default(), Decoration::Decoration),
        Ok((Cursor::default().advance_by(24), Element::CData("<![CDATA[ Hello, world ]]>".into())))
    );

    let mut doctype = "DOCTYPE Hello, world>".chars().peekable();
    assert_eq!(
        decoration(&mut doctype, Cursor::default(), Decoration::Decoration),
        Ok((Cursor::default().advance_by(21), Element::DocType("<!DOCTYPE Hello, world>".into())))
    );

    let mut xml_declaration = "xml Hello, world ?>".chars().peekable();
    assert_eq!(
        decoration(&mut xml_declaration, Cursor::default(), Decoration::Declaration),
        Ok((Cursor::default().advance_by(19), Element::XMLDeclaration("<?xml Hello, world ?>".into())))
    );

    let mut processing_instructions = "ELEMENT Hello, world>".chars().peekable();
    assert_eq!(
        decoration(&mut processing_instructions, Cursor::default(), Decoration::Decoration),
        Ok((Cursor::default().advance_by(21), Element::ProcessingInstructions("<!ELEMENT Hello, world>".into())))
    );
}
