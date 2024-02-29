// [4.1 Character and Entity References](https://www.w3.org/TR/2006/REC-xml11-20060816/#sec-references)

use crate::characters::{char, is_char};
use crate::file_reader::FileReader;
use crate::syntactic_constructs::Name;
use crate::{cursor::Cursor, diagnostics::SvgParseError, SvgParseErrorMessage};
use std::iter::Peekable;

#[derive(PartialEq, Debug)]
pub enum Reference {
    Char(String),
    Entity(String),
    ParameterEntity(String),
}

impl Reference {
    pub fn unwrap(&self) -> String {
        match self {
            Self::Char(s) | Self::Entity(s) | Self::ParameterEntity(s) => s.into(),
        }
    }
}

pub fn reference(file_reader: &mut FileReader) -> Result<Reference, Box<SvgParseError>> {
    let mut text: String = "".into();
    let cursor_start = file_reader.get_cursor();
    let is_pe_ref = match file_reader.next() {
        Some('&') => {
            // [67]
            text.push('&');
            false
        }
        Some('%') => {
            // [69]
            text.push('%');
            true
        }
        Some(c) => Err(SvgParseError::new_curse(
            file_reader.get_cursor(),
            SvgParseErrorMessage::UnexpectedChar(c, "& or %".into()),
        ))?,
        None => Err(SvgParseError::new_curse(
            file_reader.get_cursor(),
            SvgParseErrorMessage::UnexpectedEndOfFile,
        ))?,
    };

    match file_reader.peek() {
        Some('#') if !is_pe_ref => {
            // [66]
            text.push('#')
        }
        Some(&c) => {
            char(file_reader, Some(c))?;
            let ref_name = Name::new(file_reader)?;
            text.push_str(ref_name.as_str());
            char(file_reader, Some(';'))?;
            text.push(';');
            return Ok(match is_pe_ref {
                // [69]
                true => Reference::ParameterEntity(text),
                // [68]
                false => Reference::Entity(text),
            });
        }
        None => Err(SvgParseError::new_curse(
            file_reader.get_cursor(),
            SvgParseErrorMessage::UnexpectedEndOfFile,
        ))?,
    };
    file_reader.next();

    let is_hex = match file_reader.next() {
        Some('x') => {
            text.push('x');
            true
        }
        Some(c) if c.is_numeric() => {
            text.push(c);
            false
        }
        Some(c) => Err(SvgParseError::new_curse(
            file_reader.get_cursor(),
            SvgParseErrorMessage::UnexpectedChar(c, "x or number".into()),
        ))?,
        None => Err(SvgParseError::new_curse(
            file_reader.get_cursor(),
            SvgParseErrorMessage::UnexpectedEndOfFile,
        ))?,
    };

    loop {
        match file_reader.next() {
            Some(';') => {
                text.push(';');
                break;
            }
            Some(c) if c.is_numeric() => text.push(c),
            Some(c) if is_hex && ('a'..='f').contains(&c) || ('A'..='F').contains(&c) => {
                text.push(c)
            }
            Some(c) => Err(SvgParseError::new_curse(
                file_reader.get_cursor(),
                SvgParseErrorMessage::UnexpectedChar(c, "number or hex".into()),
            ))?,
            None => Err(SvgParseError::new_curse(
                file_reader.get_cursor(),
                SvgParseErrorMessage::UnexpectedEndOfFile,
            ))?,
        };
    }

    let char = match u8::from_str_radix(&text[1..text.len() - 1], 16) {
        Ok(u) => char::from(u),
        Err(_) => Err(SvgParseError::new_span(
            cursor_start.as_span(text.len()),
            SvgParseErrorMessage::IllegalCharRef(text.clone()),
        ))?,
    };
    if !is_char(&char) {
        Err(SvgParseError::new_span(
            cursor_start.as_span(text.len()),
            SvgParseErrorMessage::IllegalCharRef(text.clone()),
        ))?;
    };
    Ok(Reference::Char(text))
}

pub const XML_ENTITIES: &[(&str, char)] = &[
    ("amp", '&'),
    ("gt", '>'),
    ("lt", '<'),
    ("quot", '"'),
    ("apos", '\''),
];

pub const ENTITIES: &[(&str, char)] = &[
    ("AElig", 'Æ'),
    ("Agrave", 'À'),
    ("Aacute", 'Á'),
    ("Acirc", 'Â'),
    ("Atilde", 'Ã'),
    ("Auml", 'Ä'),
    ("Ccedil", 'Ç'),
    ("ETH", 'Ð'),
    ("Eacute", 'É'),
    ("Ecirc", 'Ê'),
    ("Egrave", 'È'),
    ("Euml", 'Ë'),
    ("Iacute", 'Í'),
    ("Aring", 'Å'),
    ("Icirc", 'Î'),
    ("Igrave", 'Ì'),
    ("Iuml", 'Ï'),
    ("Ntilde", 'Ñ'),
    ("Oacute", 'Ó'),
    ("Ocirc", 'Ô'),
    ("Ograve", 'Ò'),
    ("Oslash", 'Ø'),
    ("Otilde", 'Õ'),
    ("Ouml", 'Ö'),
    ("THORN", 'Þ'),
    ("Uacute", 'Ú'),
    ("Ucirc", 'Û'),
    ("Ugrave", 'Ù'),
    ("Uuml", 'Ü'),
    ("Yacute", 'Ý'),
    ("aacute", 'á'),
    ("acirc", 'â'),
    ("aelig", 'æ'),
    ("agrave", 'à'),
    ("aring", 'å'),
    ("atilde", 'ã'),
    ("auml", 'ä'),
    ("ccedil", 'ç'),
    ("eacute", 'é'),
    ("ecirc", 'ê'),
    ("egrave", 'è'),
    ("eth", 'ð'),
    ("euml", 'ë'),
    ("iacute", 'í'),
    ("icirc", 'î'),
    ("igrave", 'ì'),
    ("iuml", 'ï'),
    ("ntilde", 'ñ'),
    ("oacute", 'ó'),
    ("ocirc", 'ô'),
    ("ograve", 'ò'),
    ("oslash", 'ø'),
    ("otilde", 'õ'),
    ("ouml", 'ö'),
    ("szlig", 'ß'),
    ("thorn", 'þ'),
    ("uacute", 'ú'),
    ("ucirc", 'û'),
    ("ugrave", 'ù'),
    ("uuml", 'ü'),
    ("yacute", 'ý'),
    ("yuml", 'ÿ'),
    ("copy", '©'),
    ("reg", '®'),
    ("nbsp", ' '),
    ("iexcl", '¡'),
    ("cent", '¢'),
    ("pound", '£'),
    ("curren", '¤'),
    ("yen", '¥'),
    ("brvbar", '¦'),
    ("sect", '§'),
    ("uml", '¨'),
    ("ordf", 'ª'),
    ("laquo", '«'),
    ("not", '¬'),
    ("shy", '­'),
    ("macr", '¯'),
    ("deg", '°'),
    ("plusmn", '±'),
    ("sup1", '¹'),
    ("sup2", '²'),
    ("sup3", '³'),
    ("acute", '´'),
    ("micro", 'µ'),
    ("para", '¶'),
    ("middot", '·'),
    ("cedil", '¸'),
    ("ordm", 'º'),
    ("raquo", '»'),
    ("frac14", '¼'),
    ("frac12", '½'),
    ("frac34", '¾'),
    ("iquest", '¿'),
    ("times", '×'),
    ("divide", '÷'),
    ("OElig", 'Œ'),
    ("oelig", 'œ'),
    ("Scaron", 'Š'),
    ("scaron", 'š'),
    ("Yuml", 'Ÿ'),
    ("fnof", 'ƒ'),
    ("circ", 'ˆ'),
    ("tilde", '˜'),
    ("Alpha", 'Α'),
    ("Beta", 'Β'),
    ("Gamma", 'Γ'),
    ("Delta", 'Δ'),
    ("Epsilon", 'Ε'),
    ("Zeta", 'Ζ'),
    ("Eta", 'Η'),
    ("Theta", 'Θ'),
    ("Iota", 'Ι'),
    ("Kappa", 'Κ'),
    ("Lambda", 'Λ'),
    ("Mu", 'Μ'),
    ("Nu", 'Ν'),
    ("Xi", 'Ξ'),
    ("Omicron", 'Ο'),
    ("Pi", 'Π'),
    ("Rho", 'Ρ'),
    ("Sigma", 'Σ'),
    ("Tau", 'Τ'),
    ("Upsilon", 'Υ'),
    ("Phi", 'Φ'),
    ("Chi", 'Χ'),
    ("Psi", 'Ψ'),
    ("Omega", 'Ω'),
    ("alpha", 'α'),
    ("beta", 'β'),
    ("gamma", 'γ'),
    ("delta", 'δ'),
    ("epsilon", 'ε'),
    ("zeta", 'ζ'),
    ("eta", 'η'),
    ("theta", 'θ'),
    ("iota", 'ι'),
    ("kappa", 'κ'),
    ("lambda", 'λ'),
    ("mu", 'μ'),
    ("nu", 'ν'),
    ("xi", 'ξ'),
    ("omicron", 'ο'),
    ("pi", 'π'),
    ("rho", 'ρ'),
    ("sigmaf", 'ς'),
    ("sigma", 'σ'),
    ("tau", 'τ'),
    ("upsilon", 'υ'),
    ("phi", 'φ'),
    ("chi", 'χ'),
    ("psi", 'ψ'),
    ("omega", 'ω'),
    ("thetasym", 'ϑ'),
    ("upsih", 'ϒ'),
    ("piv", 'ϖ'),
    ("ensp", ' '),
    ("emsp", ' '),
    ("thinsp", ' '),
    ("zwnj", '‌'),
    ("zwj", '‍'),
    ("lrm", '‎'),
    ("rlm", '‏'),
    ("ndash", '–'),
    ("mdash", '—'),
    ("lsquo", '‘'),
    ("rsquo", '’'),
    ("sbquo", '‚'),
    ("ldquo", '“'),
    ("rdquo", '”'),
    ("bdquo", '„'),
    ("dagger", '†'),
    ("Dagger", '‡'),
    ("bull", '•'),
    ("hellip", '…'),
    ("permil", '‰'),
    ("prime", '′'),
    ("Prime", '″'),
    ("lsaquo", '‹'),
    ("rsaquo", '›'),
    ("oline", '‾'),
    ("frasl", '⁄'),
    ("euro", '€'),
    ("image", 'ℑ'),
    ("weierp", '℘'),
    ("real", 'ℜ'),
    ("trade", '™'),
    ("alefsym", 'ℵ'),
    ("larr", '←'),
    ("uarr", '↑'),
    ("rarr", '→'),
    ("darr", '↓'),
    ("harr", '↔'),
    ("crarr", '↵'),
    ("lArr", '⇐'),
    ("uArr", '⇑'),
    ("rArr", '⇒'),
    ("dArr", '⇓'),
    ("hArr", '⇔'),
    ("forall", '∀'),
    ("part", '∂'),
    ("exist", '∃'),
    ("empty", '∅'),
    ("nabla", '∇'),
    ("isin", '∈'),
    ("notin", '∉'),
    ("ni", '∋'),
    ("prod", '∏'),
    ("sum", '∑'),
    ("minus", '−'),
    ("lowast", '∗'),
    ("radic", '√'),
    ("prop", '∝'),
    ("infin", '∞'),
    ("ang", '∠'),
    ("and", '∧'),
    ("or", '∨'),
    ("cap", '∩'),
    ("cup", '∪'),
    ("int", '∫'),
    ("there4", '∴'),
    ("sim", '∼'),
    ("cong", '≅'),
    ("asymp", '≈'),
    ("ne", '≠'),
    ("equiv", '≡'),
    ("le", '≤'),
    ("ge", '≥'),
    ("sub", '⊂'),
    ("sup", '⊃'),
    ("nsub", '⊄'),
    ("sube", '⊆'),
    ("supe", '⊇'),
    ("oplus", '⊕'),
    ("otimes", '⊗'),
    ("perp", '⊥'),
    ("sdot", '⋅'),
    ("lceil", '⌈'),
    ("rceil", '⌉'),
    ("lfloor", '⌊'),
    ("rfloor", '⌋'),
    ("lang", '〈'),
    ("rang", '〉'),
    ("loz", '◊'),
    ("spades", '♠'),
    ("clubs", '♣'),
    ("hearts", '♥'),
    ("diams", '♦'),
];
