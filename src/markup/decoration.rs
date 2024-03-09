use std::iter::Peekable;

use crate::{
    cursor::Cursor, diagnostics::SVGError, file_reader::FileReader, markup::Element, syntactic_constructs::Name, SVGErrorLabel
};

pub enum Decoration {
    Decoration,
    Declaration,
}

pub fn decoration(
    file_reader: &mut FileReader,
    form: Decoration,
) -> Result<Element, Box<SVGError>> {
    // NOTE: We are not processing the body of any decorations/declarations here
    println!("Warning: decorations and declarations are ignored (ie. `<!` and `<?`)");
    let start = match file_reader.next() {
        Some(c) => c,
        None => Err(SVGError::new_curse(
            file_reader.get_cursor(),
            SVGErrorLabel::UnexpectedEndOfFile,
        ))?,
    };
    let text = match start {
        // [15]
        '-' if matches!(form, Decoration::Decoration) => ("<!--", "--"),
        // [19], [21]
        '[' if matches!(form, Decoration::Decoration) && file_reader.peek() == Some(&'C')=> ("<![CDATA[", "]]"),
        // [61]
        '[' if matches!(form, Decoration::Decoration) => ("<![", "]]"),
        // [28]
        'D' if matches!(form, Decoration::Decoration) => ("<!DOCTYPE", ""),
        // [29], [45]
        'E' if matches!(form, Decoration::Decoration) && file_reader.peek() == Some(&'L') => {
            ("<!ELEMENT", "")
        }
        // [29], [52]
        'A' if matches!(form, Decoration::Decoration) => ("<!ATTLIST", ""),
        // [29], [70]
        'E' if matches!(form, Decoration::Decoration) && file_reader.peek() == Some(&'N') => {
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
        c => Err(SVGError::new_curse( 
            file_reader.get_cursor(),
            SVGErrorLabel::UnexpectedChar(
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
    let _ = file_reader.take(text.len() - 3).collect::<String>();

    for char in file_reader.by_ref() {
        // Naiively push the character, as we don't care about this content for SVGs
        text.push(char);

        match char {
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
    Ok(element)
}

#[test]
fn test_decoration() {
    let mut comment = "-- Hello, world -->";
    assert_eq!(
        decoration(&mut FileReader::new(comment), Decoration::Decoration),
        Ok(Element::Comment("<!-- Hello, world -->".into()))
    );

    let mut cdata = "[CDATA[ Hello, world ]]>";
    assert_eq!(
        decoration(&mut FileReader::new(cdata), Decoration::Decoration),
        Ok(Element::CData("<![CDATA[ Hello, world ]]>".into()))
    );

    let mut doctype = "DOCTYPE Hello, world>";
    assert_eq!(
        decoration(&mut FileReader::new(doctype), Decoration::Decoration),
        Ok(Element::DocType("<!DOCTYPE Hello, world>".into()))
    );

    let mut xml_declaration = "xml Hello, world ?>";
    assert_eq!(
        decoration(&mut FileReader::new(xml_declaration), Decoration::Declaration),
        Ok(Element::XMLDeclaration("<?xml Hello, world ?>".into()))
    );

    let mut processing_instructions = "ELEMENT Hello, world>";
    assert_eq!(
        decoration(&mut FileReader::new(processing_instructions), Decoration::Decoration),
        Ok(Element::ProcessingInstructions("<!ELEMENT Hello, world>".into()))
    );
}
