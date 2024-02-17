use core::fmt;
use miette::{Diagnostic, NamedSource, Result, SourceOffset, SourceSpan};
use std::cell::RefCell;
use std::env;
use std::fmt::Display;
use std::fs;
use std::iter::Peekable;
use std::process;
use std::rc::Rc;
use thiserror::Error;

fn main() -> Result<()> {
    let config = Config::make(env::args()).unwrap_or_else(|error| {
        eprintln!("Invalid arguments: {error}");
        process::exit(1);
    });
    let file = fs::read_to_string(&config.path).expect("Unable to read file");
    let result = SvgDocument::parse(&file);
    match result {
        Ok(svg) => match &*svg.root.borrow() {
            Node::ContentNode(n) if n.2.tag_name.to_lowercase() == "svg" => Ok(()),
            Node::ContentNode((s_tag, ..)) => Err(SvgParseErrorProvider {
                span: Some(s_tag.borrow().span.as_source_span(&file)),
                src: NamedSource::new(config.path, file),
                error: vec![SvgParseErrorMessage::NoRootElement],
            })?,
            Node::EmptyNode(EmptyElemTag { span, .. }) => Err(SvgParseErrorProvider {
                span: Some(span.as_source_span(&file)),
                src: NamedSource::new(config.path, file),
                error: vec![SvgParseErrorMessage::NoRootElement],
            })?,
        },
        Err(error) => {
            let span: Option<SourceSpan> = match error.span {
                Some(span) => Some(span.as_source_span(&file)),
                None => error
                    .cursor
                    .map(|cursor| (cursor.as_source_offset(&file), 0).into()),
            };
            Err(SvgParseErrorProvider {
                src: NamedSource::new(config.path, file),
                span,
                error: vec![error.message],
            })?
        }
    }
}

struct Config {
    path: String,
}

impl Config {
    pub fn make(mut args: impl Iterator<Item = String>) -> Result<Config, &'static str> {
        args.next();

        let path = match args.next() {
            Some(arg) => arg,
            None => return Err("No path given"),
        };
        Ok(Config { path })
    }
}

#[derive(Debug, PartialEq)]
pub struct SvgDocument {
    preamble: Vec<Markup>,
    root: Rc<RefCell<Node>>,
    postamble: Vec<Markup>,
}

#[derive(Debug, Clone, PartialEq, Default)]
struct Span {
    start: Cursor,
    length: usize,
    source: Option<String>,
}

impl TryInto<SourceSpan> for Span {
    type Error = String;

    fn try_into(self) -> Result<SourceSpan, Self::Error> {
        match self.source {
            Some(s) => Ok((self.start.as_source_offset(s), self.length).into()),
            None => Err("No source found")?,
        }
    }
}

impl Span {
    pub fn as_source_span(&self, source: impl AsRef<str>) -> SourceSpan {
        (self.start.as_source_offset(source), self.length).into()
    }
}

#[derive(Debug, Copy, Clone, Default, PartialEq)]
struct Cursor {
    line: usize,
    column: usize,
}

impl Cursor {
    pub fn as_source_offset(&self, source: impl AsRef<str>) -> SourceOffset {
        SourceOffset::from_location(source, self.line + 1, self.column + 1)
    }

    pub fn next(&self) -> Self {
        Cursor {
            line: self.line,
            column: self.column + 1,
        }
    }

    pub fn newline(&self) -> Self {
        Cursor {
            line: self.line + 1,
            column: self.column,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
enum TagType {
    SelfClosing,
    Any,
}

impl Display for TagType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let output = match self {
            TagType::SelfClosing => "<self-closing/>",
            TagType::Any => "<opening>, </closing>, or <self-closing />",
        };
        write!(f, "{:?}", output)
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Tag {
    span: Span,
    name: String,
    attributes: Vec<Attribute>,
    tag_type: TagType,
}

#[derive(Debug, Clone, PartialEq)]
struct Attribute {
    name: String,
    value: String,
}

#[derive(Debug, Error, Diagnostic)]
#[error("Error parsing SVG!")]
struct SvgParseErrorProvider {
    #[source_code]
    src: NamedSource<String>,
    #[label]
    span: Option<SourceSpan>,
    #[related]
    error: Vec<SvgParseErrorMessage>,
}

#[derive(Debug, PartialEq)]
pub struct SvgParseError {
    span: Option<Span>,
    cursor: Option<Cursor>,
    message: SvgParseErrorMessage,
}

#[derive(Debug, Error, Diagnostic, PartialEq)]
enum SvgParseErrorMessage {
    #[error("The file ended before the expected closing svg tag")]
    UnexpectedEndOfFile,
    #[error("Expected the file to end after the closing svg tag")]
    ExpectedEndOfFile,
    #[error("Unexpected character found")]
    #[diagnostic(
        severity(error),
        help("Unexpected `{0}` found, expected `{1}` instead")
    )]
    UnexpectedChar(char, String),
    #[error("Found newline at unexpected place")]
    UnexpectedNewline,
    #[error("Expected a word here but found a symbol instead")]
    ExpectedWord,
    #[error("Expected whitespace here but found a symbol instead")]
    ExpectedWhitespace,
    #[error("Unexpected {0} tag found")]
    UnexpectedTagType(TagType),
    #[error("The file doesn't contain a root svg element")]
    NoRootElement,
    #[error("The file contains more than 1 root element")]
    MultipleRootElements,
    #[error("Unexpected </{0}>, expected </{1}>")]
    UnmatchedTag(String, String),
}

impl SvgDocument {
    pub fn parse(svg: &str) -> Result<SvgDocument, Box<SvgParseError>> {
        let mut chars = svg.chars().peekable();
        let mut cursor = Cursor::default();

        let mut preamble = Vec::new();
        let root_start = Rc::new(RefCell::new(STag::default()));
        loop {
            let (c, item) = markup(&mut chars, cursor, None)?;
            cursor = c;
            match item {
                Markup::Element(e) => match e {
                    Element::StartTag(e) => {
                        root_start.replace(e);
                        break;
                    }
                    Element::EmptyTag(EmptyElemTag { span, .. })
                    | Element::EndTag(ETag { span, .. }) => {
                        Err(SvgParseError {
                            span: Some(span),
                            cursor: None,
                            message: SvgParseErrorMessage::UnexpectedTagType(TagType::SelfClosing),
                        })?;
                    }
                    e => preamble.push(Markup::Element(e)),
                },
                m => preamble.push(m),
            };
        }
        let (mut cursor, root) = node(&mut chars, cursor, root_start)?;

        let mut postamble = Vec::new();
        loop {
            let (c, item) = markup(&mut chars, cursor, None)?;
            cursor = c;
            match item {
                Markup::Element(e) => match e {
                    Element::EndOfFile => break,
                    Element::StartTag(STag { span, .. }) => Err(SvgParseError {
                        span: Some(span),
                        cursor: None,
                        message: SvgParseErrorMessage::MultipleRootElements,
                    })?,
                    Element::EmptyTag(EmptyElemTag { span, .. })
                    | Element::EndTag(ETag { span, .. }) => Err(SvgParseError {
                        span: Some(span),
                        cursor: None,
                        message: SvgParseErrorMessage::UnexpectedTagType(TagType::Any),
                    })?,
                    e => postamble.push(Markup::Element(e)),
                },
                m => postamble.push(m),
            };
        }
        Ok(SvgDocument {
            preamble,
            root,
            postamble,
        })
    }
}

#[derive(PartialEq, Debug)]
enum Markup {
    Element(Element),
    Reference(Reference),
    CharData(String),
}

fn markup(
    partial: &mut Peekable<impl Iterator<Item = char>>,
    cursor: Cursor,
    parent: Option<Rc<RefCell<Node>>>,
) -> Result<(Cursor, Markup), Box<SvgParseError>> {
    match partial.peek() {
        Some('<') => {
            // start-tag, end-tag, empty-element-tag, comment, cdata, doctype, processing-data,
            // xml-declaration, text-declaration
            partial.next();
            element(partial, cursor, parent).map(|(c, e)| (c, Markup::Element(e)))
        }
        Some(&c) if c == '&' || c == '%' => {
            // reference
            reference(partial, cursor).map(|(c, r)| (c, Markup::Reference(r)))
        }
        Some(_) => char_data(partial, cursor).map(|(c, s)| (c, Markup::CharData(s))),
        None => Err(SvgParseError {
            span: None,
            cursor: Some(cursor),
            message: SvgParseErrorMessage::UnexpectedEndOfFile,
        })?,
    }
}

#[derive(PartialEq, Debug)]
enum Element {
    StartTag(STag),
    EndTag(ETag),
    EmptyTag(EmptyElemTag),
    Comment(String),
    CData(String),
    DocType(String),
    ProcessingInstructions(String),
    XMLDeclaration(String),
    EndOfFile,
}

enum Decoration {
    Decoration,
    Declaration,
}

fn element(
    partial: &mut Peekable<impl Iterator<Item = char>>,
    cursor: Cursor,
    parent: Option<Rc<RefCell<Node>>>,
) -> Result<(Cursor, Element), Box<SvgParseError>> {
    match partial.next() {
        // comment, cdata, doctype
        Some('!') => Ok(decoration(partial, cursor.next(), Decoration::Decoration)?),
        // processing-data, cml-declaration, text-declaration
        Some('?') => Ok(decoration(partial, cursor.next(), Decoration::Declaration)?),
        // open-tag, close-tag, empty-tag
        Some(_) => Ok(tag_type(partial, cursor, parent)?),
        None => Ok((cursor, Element::EndOfFile)),
    }
}

fn decoration(
    partial: &mut Peekable<impl Iterator<Item = char>>,
    cursor: Cursor,
    form: Decoration,
) -> Result<(Cursor, Element), Box<SvgParseError>> {
    let start = match partial.next() {
        Some(c) => c,
        None => Err(SvgParseError {
            span: None,
            cursor: Some(cursor),
            message: SvgParseErrorMessage::UnexpectedEndOfFile,
        })?,
    };
    let text = match start {
        // As per https://www.w3.org/TR/2006/REC-xml11-20060816/#dt-comment
        '-' if matches!(form, Decoration::Decoration) => ("--", "--"),
        // As per https://www.w3.org/TR/2006/REC-xml11-20060816/#dt-cdsection
        '[' if matches!(form, Decoration::Decoration) => ("![CDATA[", "]]"),
        // As per https://www.w3.org/TR/2006/REC-xml11-20060816/#dt-doctype
        'D' if matches!(form, Decoration::Decoration) => ("!DOCTYPE", ""),
        'A' if matches!(form, Decoration::Decoration) => ("!ATTLIST", ""),
        'E' if matches!(form, Decoration::Decoration) && partial.peek() == Some(&'L') => {
            ("!ELEMENT", "")
        }
        'E' if matches!(form, Decoration::Decoration) && partial.peek() == Some(&'N') => {
            ("!ENTITY", "")
        }
        // As per https://www.w3.org/TR/2006/REC-xml11-20060816/#NT-TextDecl
        // As per https://www.w3.org/TR/2006/REC-xml11-20060816/#NT-XMLDecl
        'x' if matches!(form, Decoration::Declaration) => ("xml", "?"),
        // As per https://www.w3.org/TR/2006/REC-xml11-20060816/#dt-pi
        c if matches!(form, Decoration::Declaration) && is_name_start_char(&c) => ("", "?"),
        c => Err(SvgParseError {
            span: None,
            cursor: Some(cursor),
            message: SvgParseErrorMessage::UnexpectedChar(
                c,
                match form {
                    Decoration::Decoration => {
                        "a matching char for a comment (`<!--`), doctype (`<!DOCTYPE`), or CData (`![CDATA[`)".into()
                    }
                    Decoration::Declaration => "a matching char for `<?xml` or `<?...`".into(),
                },
            ),
        })?,
    };
    let end: String = text.1.into();
    let mut text: String = text.0.into();

    let mut cursor = Cursor {
        line: cursor.line,
        column: cursor.column + text.len(),
    };

    for char in partial.by_ref() {
        // Naiively push the character, as we don't care about this content for SVGs
        text.push(char);

        cursor = cursor.next();
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

#[derive(PartialEq, Debug)]
enum Reference {
    Char(String),
    Entity(String),
    ParameterEntity(String),
}

fn reference(
    partial: &mut Peekable<impl Iterator<Item = char>>,
    cursor: Cursor,
) -> Result<(Cursor, Reference), Box<SvgParseError>> {
    let mut text: String = "".into();
    let cursor = cursor.next();
    let is_pe_ref = match partial.next() {
        Some('&') => {
            text.push('&');
            false
        }
        Some('%') => {
            text.push('%');
            true
        }
        Some(c) => Err(SvgParseError {
            span: None,
            cursor: Some(cursor),
            message: SvgParseErrorMessage::UnexpectedChar(c, "& or %".into()),
        })?,
        None => Err(SvgParseError {
            span: None,
            cursor: Some(cursor),
            message: SvgParseErrorMessage::UnexpectedEndOfFile,
        })?,
    };

    match partial.peek() {
        Some('#') => text.push('#'),
        Some(&c) => {
            let cursor = char(partial, cursor, Some(c))?;
            let (cursor, ref_name) = name(partial, cursor)?;
            text.push_str(&ref_name);
            let cursor = char(partial, cursor, Some(';'))?;
            text.push(';');
            return Ok((
                cursor,
                match is_pe_ref {
                    true => Reference::ParameterEntity(text),
                    false => Reference::Entity(text),
                },
            ));
        }
        None => Err(SvgParseError {
            span: None,
            cursor: Some(cursor),
            message: SvgParseErrorMessage::UnexpectedEndOfFile,
        })?,
    };
    let cursor = cursor.next();
    partial.next();

    let cursor = cursor.next();
    let is_hex = match partial.next() {
        Some('x') => {
            text.push('x');
            true
        }
        Some(c) if c.is_numeric() => {
            text.push(c);
            false
        }
        Some(c) => Err(SvgParseError {
            span: None,
            cursor: Some(cursor),
            message: SvgParseErrorMessage::UnexpectedChar(c, "x or number".into()),
        })?,
        None => Err(SvgParseError {
            span: None,
            cursor: Some(cursor),
            message: SvgParseErrorMessage::UnexpectedEndOfFile,
        })?,
    };

    loop {
        let cursor = cursor.next();
        match partial.next() {
            Some(';') => {
                text.push(';');
                break;
            }
            Some(c) if c.is_numeric() => text.push(c),
            Some(c) if is_hex && ('a'..='f').contains(&c) || ('A'..='F').contains(&c) => {
                text.push(c)
            }
            Some(c) => Err(SvgParseError {
                span: None,
                cursor: Some(cursor),
                message: SvgParseErrorMessage::UnexpectedChar(c, "number or hex".into()),
            })?,
            None => Err(SvgParseError {
                span: None,
                cursor: Some(cursor),
                message: SvgParseErrorMessage::UnexpectedEndOfFile,
            })?,
        };
    }

    Ok((cursor, Reference::Char(text)))
}

fn char_data(
    partial: &mut Peekable<impl Iterator<Item = char>>,
    cursor: Cursor,
) -> Result<(Cursor, String), Box<SvgParseError>> {
    // As per https://www.w3.org/TR/2006/REC-xml11-20060816/#IDAABES
    let mut text: String = "".into();
    while let Some(&char) = partial.peek() {
        match char {
            '&' => break,
            '<' => break,
            _ => {}
        }
        text.push(char);
        cursor.next();
        partial.next();
    }

    Ok((cursor, text))
}

fn tag_type(
    partial: &mut Peekable<impl Iterator<Item = char>>,
    cursor: Cursor,
    parent: Option<Rc<RefCell<Node>>>,
) -> Result<(Cursor, Element), Box<SvgParseError>> {
    let cursor_start = cursor;
    if let Some('/') = partial.peek() {
        partial.next();
        let cursor = cursor.next();
        let (cursor, tag_name) = name(partial, cursor)?;
        let length = tag_name.len() + 2;
        let cursor = whitespace(partial, cursor, false)?;
        let cursor = char(partial, cursor, Some('>'))?;
        return Ok((
            cursor,
            Element::EndTag(ETag {
                start_tag: Rc::new(RefCell::new(STag::default())),
                tag_name,
                span: Span {
                    start: cursor_start,
                    length,
                    source: None,
                },
            }),
        ));
    };

    let (cursor, tag_name) = name(partial, cursor)?;
    let cursor = whitespace(partial, cursor, true)?;
    let (cursor, attributes) = attributes(partial, cursor)?;

    let cursor = cursor.next();
    match partial.next() {
        Some('/') => {
            let cursor = char(partial, cursor, Some('>'))?;
            let length = tag_name.len() + 1;
            Ok((
                cursor,
                Element::EmptyTag(EmptyElemTag {
                    parent,
                    tag_name,
                    attributes,
                    span: Span {
                        start: cursor_start,
                        length,
                        source: None,
                    },
                }),
            ))
        }
        Some('>') => Ok((
            cursor,
            Element::StartTag(STag {
                parent,
                tag_name: tag_name.clone(),
                attributes,
                span: Span {
                    start: cursor_start,
                    length: tag_name.len() + 1,
                    source: None,
                },
            }),
        )),
        Some(c) => Err(SvgParseError {
            span: None,
            cursor: Some(cursor),
            message: SvgParseErrorMessage::UnexpectedChar(c, "> or />".into()),
        })?,
        None => Err(SvgParseError {
            span: None,
            cursor: Some(cursor),
            message: SvgParseErrorMessage::UnexpectedEndOfFile,
        })?,
    }
}

fn content(
    partial: &mut Peekable<impl Iterator<Item = char>>,
    cursor: Cursor,
    parent: Rc<RefCell<Node>>,
) -> Result<(Cursor, Vec<NodeContent>, ETag), Box<SvgParseError>> {
    // As per https://www.w3.org/TR/2006/REC-xml11-20060816/#NT-content
    let mut cursor = cursor;
    let mut content = Vec::new();
    let tag_name = match &*parent.borrow() {
        Node::ContentNode((span, ..)) => span.borrow().tag_name.clone(),
        Node::EmptyNode(_) => {
            unreachable!(
                "Error: Attempted to parse content of empty tag. Please raise a bugfix request"
            )
        }
    };
    loop {
        let (c, item) = markup(partial, cursor, Some(Rc::clone(&parent)))?;
        cursor = c;
        match item {
            Markup::Element(e) => match e {
                Element::StartTag(t) => {
                    let (c, node) = node(partial, cursor, Rc::new(RefCell::new(t)))?;
                    cursor = c;
                    content.push(NodeContent::Node(node));
                }
                Element::EmptyTag(t) => {
                    content.push(NodeContent::Node(Rc::new(RefCell::new(Node::EmptyNode(t)))))
                }
                Element::EndTag(t) if t.tag_name == tag_name => return Ok((cursor, content, t)),
                Element::EndTag(t) => Err(SvgParseError {
                    span: Some(t.span),
                    cursor: None,
                    message: SvgParseErrorMessage::UnmatchedTag(t.tag_name, tag_name.clone()),
                })?,
                Element::EndOfFile => Err(SvgParseError {
                    span: None,
                    cursor: Some(cursor),
                    message: SvgParseErrorMessage::ExpectedEndOfFile,
                })?,
                e => content.push(NodeContent::Element(e)),
            },
            m => content.push(NodeContent::Markup(m)),
        }
    }
}

// https://www.w3.org/TR/2006/REC-xml11-20060816/#NT-EmptyElemTag
#[derive(PartialEq, Debug)]
struct EmptyElemTag {
    parent: Option<Rc<RefCell<Node>>>,
    tag_name: String,
    attributes: Vec<Attribute>,
    span: Span,
}

// https://www.w3.org/TR/2006/REC-xml11-20060816/#NT-STag
#[derive(PartialEq, Default, Debug)]
struct STag {
    parent: Option<Rc<RefCell<Node>>>,
    tag_name: String,
    attributes: Vec<Attribute>,
    span: Span,
}

// https://www.w3.org/TR/2006/REC-xml11-20060816/#NT-ETag
#[derive(PartialEq, Default, Debug)]
struct ETag {
    start_tag: Rc<RefCell<STag>>,
    tag_name: String,
    span: Span,
}

#[derive(PartialEq, Debug)]
enum NodeContent {
    Element(Element),
    Node(Rc<RefCell<Node>>),
    Markup(Markup),
}

#[derive(PartialEq, Debug)]
enum Node {
    EmptyNode(EmptyElemTag),
    ContentNode((Rc<RefCell<STag>>, Vec<NodeContent>, ETag)),
}

fn node(
    partial: &mut Peekable<impl Iterator<Item = char>>,
    cursor: Cursor,
    start_tag: Rc<RefCell<STag>>,
) -> Result<(Cursor, Rc<RefCell<Node>>), Box<SvgParseError>> {
    let node = Rc::new(RefCell::new(Node::ContentNode((
        Rc::clone(&start_tag),
        Vec::new(),
        ETag::default(),
    ))));
    let (cursor, content, end_tag) = content(partial, cursor, Rc::clone(&node))?;
    match &mut *node.borrow_mut() {
        Node::ContentNode((_, ref mut c, ref mut e)) => {
            *c = content;
            e.start_tag = start_tag;
            *e = end_tag;
        }
        _ => unreachable!(),
    }
    Ok((cursor, node))
}

fn is_name_start_char(char: &char) -> bool {
    // As per https://www.w3.org/TR/2006/REC-xml11-20060816/#sec-common-syn [4]
    if match char {
        '_' => true,
        ':' => true,
        c if c.is_uppercase() => true,
        c if c.is_lowercase() => true,
        _ => false,
    } {
        return true;
    }

    let mut utf16 = [0; 2];
    char.encode_utf16(&mut utf16);
    let utf16 = utf16[0] as u32 | (utf16[1] as u32) << 16;
    (0xC0..=0xD6).contains(&utf16)
        || (0xD8..=0xF6).contains(&utf16)
        || (0xF8..=0x2FF).contains(&utf16)
        || (0x370..=0x37D).contains(&utf16)
        || (0x37F..=0x1FFF).contains(&utf16)
        || (0x200C..=0x200D).contains(&utf16)
        || (0x2070..=0x218F).contains(&utf16)
        || (0x2C00..=0x2FEF).contains(&utf16)
        || (0x3001..=0xD7FF).contains(&utf16)
        || (0xF900..=0xFDCF).contains(&utf16)
        || (0xFDF0..=0xFFFD).contains(&utf16)
        || (0x10000..=0xEFFFF).contains(&utf16)
}

fn is_name_char(char: &char) -> bool {
    // As per https://www.w3.org/TR/2006/REC-xml11-20060816/#sec-common-syn [4a]
    if match char {
        c if is_name_start_char(c) => true,
        '-' => true,
        '.' => true,
        c if c.is_numeric() => true,
        _ => false,
    } {
        return true;
    }

    let mut utf16 = [0; 2];
    char.encode_utf16(&mut utf16);
    let utf16 = utf16[0];
    utf16 == 0xB7 || (0x0300..0x036F).contains(&utf16) || (0x203F..0x2040).contains(&utf16)
}

static NAME_EXPECTED: &str = "valid starting name character";

fn name(
    partial: &mut Peekable<impl Iterator<Item = char>>,
    cursor: Cursor,
) -> Result<(Cursor, String), Box<SvgParseError>> {
    let mut column = cursor.column;
    let mut text = "".to_string();
    match partial.peek() {
        Some(c) if is_name_start_char(c) => {}
        Some(&c) => Err(SvgParseError {
            span: None,
            cursor: Some(Cursor {
                line: cursor.line,
                column: column + 1,
            }),
            message: SvgParseErrorMessage::UnexpectedChar(c, NAME_EXPECTED.into()),
        })?,
        None => Err(SvgParseError {
            span: None,
            cursor: Some(Cursor {
                line: cursor.line,
                column,
            }),
            message: SvgParseErrorMessage::UnexpectedEndOfFile,
        })?,
    }

    while let Some(next_char) = partial.peek() {
        if !is_name_char(next_char) {
            break;
        }

        column += 1;
        text.push(partial.next().unwrap());
    }

    if text.is_empty() {
        Err(SvgParseError {
            span: None,
            cursor: Some(Cursor {
                line: cursor.line,
                column: column + 1,
            }),
            message: SvgParseErrorMessage::ExpectedWord,
        })?
    }

    Ok((
        Cursor {
            line: cursor.line,
            column,
        },
        text,
    ))
}

#[test]
fn test_name() {
    let mut word = "Hello, world!".chars().peekable();
    assert_eq!(
        name(&mut word, Cursor::default()),
        Ok((Cursor { line: 0, column: 5 }, "Hello".into())),
    );
    assert_eq!(word.next(), Some(','));

    let mut no_word = "".chars().peekable();
    assert_eq!(
        name(&mut no_word, Cursor::default()),
        Err(Box::new(SvgParseError {
            span: None,
            cursor: Some(Cursor::default()),
            message: SvgParseErrorMessage::UnexpectedEndOfFile,
        }))
    );

    let mut leading_whitespace = " Hello, world!".chars().peekable();
    assert_eq!(
        name(&mut leading_whitespace, Cursor::default()),
        Err(Box::new(SvgParseError {
            span: None,
            cursor: Some(Cursor { line: 0, column: 1 }),
            message: SvgParseErrorMessage::UnexpectedChar(' ', NAME_EXPECTED.into())
        }))
    );
    assert_eq!(leading_whitespace.next(), Some(' '));

    let mut includes_permitted_name_chars = ":_-.Aa ".chars().peekable();
    assert_eq!(
        name(&mut includes_permitted_name_chars, Cursor::default()),
        Ok((Cursor { line: 0, column: 6 }, ":_-.Aa".into()))
    );
    assert_eq!(includes_permitted_name_chars.next(), Some(' '));
}

fn whitespace(
    partial: &mut Peekable<impl Iterator<Item = char>>,
    cursor: Cursor,
    required: bool,
) -> Result<Cursor, Box<SvgParseError>> {
    let Cursor {
        mut line,
        mut column,
    } = cursor;
    let is_whitespace = partial.peek().is_some_and(|x| x.is_whitespace());

    if required && !is_whitespace {
        Err(SvgParseError {
            span: None,
            cursor: Some(cursor),
            message: SvgParseErrorMessage::ExpectedWhitespace,
        })?;
    }

    while let Some(&x) = partial.peek() {
        if !x.is_whitespace() {
            break;
        }

        column += 1;
        if x == '\n' {
            line += 1;
            column = 0;
        }

        partial.next();
    }

    Ok(Cursor { line, column })
}

#[test]
fn test_whitespace() {
    let mut file_empty = "".chars().peekable();
    assert_eq!(
        whitespace(&mut file_empty, Cursor::default(), false),
        Ok(Cursor::default()),
        "expect empty string to move cursor by 0"
    );

    let mut file_empty = "".chars().peekable();
    assert_eq!(
        whitespace(&mut file_empty, Cursor::default(), true),
        Err(Box::new(SvgParseError {
            span: None,
            cursor: Some(Cursor::default()),
            message: SvgParseErrorMessage::ExpectedWhitespace
        })),
        "expect required whitespace in empty string to fail"
    );

    let mut file_whitespace = "  Hello, world!".chars().peekable();
    assert_eq!(
        whitespace(&mut file_whitespace, Cursor::default(), true),
        Ok(Cursor { line: 0, column: 2 }),
        "expect string to move cursor by two column"
    );
    assert_eq!(file_whitespace.next(), Some('H'));

    let mut file_whitespace_end = "  Hello, world!  ".chars().peekable();
    assert_eq!(
        whitespace(&mut file_whitespace_end, Cursor::default(), true),
        Ok(Cursor { line: 0, column: 2 }),
        "expect string to move cursor by two columns"
    );
    assert_eq!(file_whitespace_end.next(), Some('H'));

    let mut file_newline = "\nHello, world!".chars().peekable();
    assert_eq!(
        whitespace(&mut file_newline, Cursor::default(), true),
        Ok(Cursor { line: 1, column: 0 }),
        "expect string to move cursor to next line"
    );
    assert_eq!(file_newline.next(), Some('H'));

    let mut file_newline_and_space = "  \n Hello, world!".chars().peekable();
    assert_eq!(
        whitespace(&mut file_newline_and_space, Cursor::default(), true),
        Ok(Cursor { line: 1, column: 1 }),
        "expect string to move cursor to next line and 1 column"
    );
    assert_eq!(file_newline_and_space.next(), Some('H'));
}

fn char(
    partial: &mut Peekable<impl Iterator<Item = char>>,
    cursor: Cursor,
    expected: Option<char>,
) -> Result<Cursor, Box<SvgParseError>> {
    let char = match partial.next() {
        Some(x) => x,
        None => Err(SvgParseError {
            span: None,
            cursor: Some(cursor),
            message: SvgParseErrorMessage::UnexpectedEndOfFile,
        })?,
    };

    match expected {
        Some(x) if x != char => Err(SvgParseError {
            span: None,
            cursor: Some(cursor),
            message: SvgParseErrorMessage::UnexpectedChar(x, char.into()),
        })?,
        _ => {}
    };
    Ok(Cursor {
        line: cursor.line,
        column: cursor.column + 1,
    })
}

fn word(
    partial: &mut Peekable<impl Iterator<Item = char>>,
    cursor: Cursor,
) -> Result<(Cursor, String), Box<SvgParseError>> {
    let word_chars = partial.take_while(|x| !x.is_alphabetic());
    let word: String = word_chars.collect();

    if word.is_empty() {
        Err(SvgParseError {
            span: None,
            cursor: Some(cursor),
            message: SvgParseErrorMessage::ExpectedWord,
        })?;
    }

    Ok((
        Cursor {
            line: cursor.line,
            column: cursor.column + word.len(),
        },
        word.to_lowercase(),
    ))
}

fn entity_value(
    partial: &mut Peekable<impl Iterator<Item = char>>,
    cursor: Cursor,
) -> Result<(Cursor, String), Box<SvgParseError>> {
    let quote_style = match partial.next() {
        Some(x) if x == '"' || x == '\'' => x,
        Some(x) => Err(SvgParseError {
            span: None,
            cursor: Some(cursor),
            message: SvgParseErrorMessage::UnexpectedChar('"', x.into()),
        })?,
        None => Err(SvgParseError {
            span: None,
            cursor: Some(cursor),
            message: SvgParseErrorMessage::UnexpectedEndOfFile,
        })?,
    };
    let contents: String = partial.take_while(|&x| x != quote_style).collect();
    let cursor = char(partial, cursor, Some(quote_style))?;

    if contents.contains('\n') {
        Err(SvgParseError {
            span: None,
            cursor: Some(cursor),
            message: SvgParseErrorMessage::UnexpectedNewline,
        })?;
    }
    Ok((
        Cursor {
            line: cursor.line,
            column: cursor.column + contents.len() + 2,
        },
        contents,
    ))
}

fn attributes(
    partial: &mut Peekable<impl Iterator<Item = char>>,
    cursor: Cursor,
) -> Result<(Cursor, Vec<Attribute>), Box<SvgParseError>> {
    let mut entries: Vec<Attribute> = Vec::new();
    let mut cursor_end = cursor;

    loop {
        match partial.peek() {
            Some('/') => break,
            Some('>') => break,
            _ => {}
        };
        partial.next();
        let cursor = cursor.next();

        let (cursor, name) = word(partial, cursor)?;
        let cursor = char(partial, cursor, Some('='))?;
        let (cursor, value) = entity_value(partial, cursor)?;
        let cursor = whitespace(partial, cursor, false)?;
        cursor_end = cursor;
        entries.push(Attribute { name, value });
    }

    Ok((cursor_end, entries))
}
