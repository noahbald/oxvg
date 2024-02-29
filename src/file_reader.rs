use std::{
    borrow::Borrow, cell::RefCell, char::from_u32, collections::HashMap, iter::Peekable, rc::Rc,
    str::Chars,
};

use crate::{
    cursor::{Cursor, Span},
    diagnostics::{SvgParseError, SvgParseErrorMessage},
    document::Node,
    markup::{ETag, EmptyElemTag, STag},
    references::{ENTITIES, XML_ENTITIES},
    syntactic_constructs::{is_whitespace, Name},
};

/// A sax style parser for XML written for SVG
///
/// Some content is derived from [svg/sax](github.com/svg/sax)
/// Copyright (c) Isaac Z. Schlueter and Contributors
pub struct FileReader<'a> {
    file: &'a str,
    peekable: Peekable<Chars<'a>>,
    cursor: Cursor,
    offset: usize,
    errors: Vec<SvgParseError>,
    options: SAXOptions,
    sax: SAXState,
    prev_state: SAXCollectedState,
}

#[derive(Default)]
struct SAXPositions {
    state_start: Cursor,
    state_offset_start: usize,
}

#[derive(Default)]
struct SAXOptions {
    pub strict: bool,
    pub xmlns: bool,
}

#[derive(Default)]
struct SAXState {
    state: State,
    position: SAXPositions,
    start_tag_position: Cursor,
    tags: Vec<Rc<RefCell<Node>>>,
    tag: Option<Node>,
    attribute_map: HashMap<String, String>,
    attribute_name: String,
    attribute_value: String,
    ordered_attribute_names: Vec<String>,
    text_node: String,
    saw_root: bool,
    closed_root: bool,
    script: String,
    sgml_declaration: String,
    tag_name: String,
    processing_instruction_name: String,
    processing_instruction_body: String,
    cdata: String,
    comment: String,
    doctype: bool,
    quote: Option<char>,
    entity: String,
    entity_map: HashMap<String, char>,
}

#[derive(Default)]
pub struct SAXCollectedState {
    pub contents: String,
    pub span: Span,
    pub state_collected: State,
    next_char: Option<char>,
}

impl<'a> Default for FileReader<'a> {
    fn default() -> Self {
        Self {
            file: "",
            peekable: "".chars().peekable(),
            cursor: Cursor::default(),
            offset: 0,
            errors: Vec::new(),
            options: SAXOptions::default(),
            sax: SAXState::default(),
            prev_state: SAXCollectedState::default(),
        }
    }
}

impl<'a> FileReader<'a> {
    pub fn new(file: &'a str) -> Self {
        FileReader {
            file,
            peekable: file.chars().peekable(),
            ..FileReader::default()
        }
    }

    pub fn peek(&mut self) -> Option<&char> {
        self.peekable.peek()
    }

    pub fn get_cursor(&self) -> Cursor {
        self.cursor.clone()
    }

    pub fn get_span(&self) -> Span {
        self.sax
            .position
            .state_start
            .as_span(self.offset - self.sax.position.state_offset_start)
    }

    pub fn collect_state(&mut self) -> Option<&SAXCollectedState> {
        self.prev_state.state_collected = self.sax.state.clone();
        let mut contents = match self.prev_state.next_char {
            Some(char) => String::from(char),
            None => String::new(),
        };
        loop {
            let char = self.next();
            match char {
                Some(char) if self.sax.state == self.prev_state.state_collected => {
                    contents.push(char)
                }
                Some(_) => {
                    self.prev_state.contents = contents;
                    self.prev_state.next_char = char;
                    return Some(&self.prev_state);
                }
                None => return None,
            };
        }
    }

    fn set_state(&mut self, state: State) {
        self.prev_state.span = self
            .sax
            .position
            .state_start
            .as_span(self.offset - self.sax.position.state_offset_start);
        self.prev_state.state_collected = self.sax.state.clone();
        self.sax.state = state;
        self.sax.position.state_start = self.get_cursor();
        self.sax.position.state_offset_start = self.offset;
    }
}

impl<'a> Iterator for FileReader<'a> {
    type Item = char;

    /// Advances the file reader and returns the next value.
    ///
    /// The file reader is a state machine, and consuming next will transition it's state.
    /// Returns `None` when the iterator is finished.
    ///
    /// # Examples
    ///
    /// Basic usage:
    /// ```
    /// let file_reader = FileReader::new("<svg></svg>");
    ///
    /// // A call to next() returns the next value...
    /// assert_eq!(Some('<'), file_reader.next());
    /// assert_eq!(Some('s'), file_reader.next());
    /// assert_eq!(Some('v'), file_reader.next());
    ///
    /// // ... and then None once it's over.
    /// assert_eq!(None, file_reader.next());
    ///
    /// // More calls may or may not return `None`. Here, they always will.
    /// assert_eq!(None, file_reader.next());
    ///
    /// assert_eq!(None, file_reader.next());
    /// ```
    fn next(&mut self) -> Option<char> {
        let char = self.peekable.next();

        self.offset += 1;
        if Some('\n') == char {
            self.cursor.mut_newline();
        } else {
            self.cursor.mut_advance();
        }

        if let Some(char) = char {
            self.state(&char);
        }
        char
    }
}

impl<'a> FileReader<'a> {
    fn state(&mut self, char: &char) {
        match self.sax.state {
            State::Begin => {
                self.set_state(State::BeginWhitespace);
                if char != &'\u{FEFF}' {
                    self.begin_whitespace(char)
                }
            }
            State::BeginWhitespace => self.begin_whitespace(char),
            State::Text => self.text(char),
            State::Script if char == &'<' => self.set_state(State::ScriptEnding),
            State::Script => self.sax.script.push(char.clone()),
            State::ScriptEnding if char == &'/' => self.set_state(State::CloseTag),
            State::ScriptEnding => {
                self.sax.script.push('<');
                self.sax.script.push(char.clone());
                self.set_state(State::Script);
            }
            State::OpenWaka if char == &'!' => {
                self.set_state(State::SGMLDeclaration);
                self.sax.sgml_declaration = String::new();
            }
            State::OpenWaka if is_whitespace(char) => {}
            State::OpenWaka if Name::is_name_start_char(char) => {
                self.set_state(State::OpenTag);
                self.sax.tag_name = String::from(char.clone());
            }
            State::OpenWaka if char == &'/' => {
                self.set_state(State::CloseTag);
                self.sax.tag_name = String::new();
            }
            State::OpenWaka if char == &'?' => {
                self.set_state(State::ProcessingInstruction);
                self.sax.processing_instruction_body = String::new();
            }
            State::OpenWaka => {
                if self.options.strict {
                    self.errors.push(SvgParseError::new_curse(
                        self.get_cursor(),
                        SvgParseErrorMessage::UnencodedLt,
                    ))
                }
                self.sax.text_node.push('<');
                self.sax.text_node.push(char.clone());
                self.set_state(State::Text);
            }
            State::SGMLDeclaration if self.sax.sgml_declaration.to_uppercase() == "[CDATA[" => {
                self.set_state(State::CData);
                self.sax.sgml_declaration = String::default();
                self.sax.cdata = String::default();
            }
            State::SGMLDeclaration if self.sax.sgml_declaration == "-" && char == &'-' => {
                self.set_state(State::Comment);
                self.sax.comment = String::default();
                self.sax.sgml_declaration = String::default();
            }
            State::SGMLDeclaration if self.sax.sgml_declaration.to_uppercase() == "DOCTYPE" => {
                if !self.sax.doctype || self.sax.saw_root {
                    self.errors.push(SvgParseError::new_curse(
                        self.get_cursor(),
                        SvgParseErrorMessage::InappropriateDoctype,
                    ));
                }
                self.set_state(State::Doctype);
                self.sax.sgml_declaration = String::default();
            }
            State::SGMLDeclaration if char == &'>' => {
                self.set_state(State::Text);
                self.sax.sgml_declaration = String::default();
            }
            State::SGMLDeclaration if char == &'"' || char == &'\'' => {
                self.set_state(State::SGMLDeclarationQuoted);
                self.sax.sgml_declaration.push(char.clone());
                self.sax.quote = Some(char.clone());
            }
            State::SGMLDeclaration => {
                self.sax.sgml_declaration.push(char.clone());
            }
            State::SGMLDeclarationQuoted => {
                if Some(char.clone()) == self.sax.quote {
                    self.set_state(State::SGMLDeclaration);
                    self.sax.quote = None;
                }
                self.sax.sgml_declaration.push(char.clone());
            }
            State::Doctype if char == &'>' => {
                self.set_state(State::Text);
                self.sax.doctype = true;
            }
            State::Doctype if char == &'[' => {
                self.set_state(State::DoctypeDTD);
            }
            State::Doctype if char == &'"' || char == &'\'' => {
                self.set_state(State::DoctypeQuoted);
                self.sax.quote = Some(char.clone());
            }
            State::DoctypeQuoted if Some(char.clone()) == self.sax.quote => {
                self.set_state(State::Doctype);
                self.sax.quote = None;
            }
            State::DoctypeDTD if char == &']' => {
                self.set_state(State::Doctype);
            }
            State::DoctypeDTD if char == &'"' || char == &'\'' => {
                self.set_state(State::DoctypeDTDQuoted);
                self.sax.quote = Some(char.clone());
            }
            State::DoctypeDTDQuoted if Some(char.clone()) == self.sax.quote => {
                self.set_state(State::DoctypeDTD);
                self.sax.quote = None;
            }
            State::Doctype | State::DoctypeQuoted | State::DoctypeDTD | State::DoctypeDTDQuoted => {
            }
            State::Comment if char == &'-' => self.set_state(State::CommentEnding),
            State::Comment => self.sax.comment.push(char.clone()),
            State::CommentEnding if char == &'-' => {
                self.set_state(State::CommentEnded);
                self.sax.comment = String::default();
            }
            State::CommentEnding => {
                self.set_state(State::Comment);
                self.sax.comment.push('-');
                self.sax.comment.push(char.clone());
            }
            State::CommentEnded if char == &'>' => self.set_state(State::Text),
            State::CommentEnded => {
                if self.options.strict {
                    self.errors.push(SvgParseError::new_curse(
                        self.get_cursor(),
                        SvgParseErrorMessage::MalformedComment,
                    ))
                }
                self.set_state(State::Comment);
                self.sax.comment.push_str("--");
                self.sax.comment.push(char.clone());
            }
            State::CData if char == &']' => self.set_state(State::CDataEnding),
            State::CData => self.sax.cdata.push(char.clone()),
            State::CDataEnding if char == &']' => self.set_state(State::CDataEnding2),
            State::CDataEnding => {
                self.set_state(State::CData);
                self.sax.cdata.push(']');
                self.sax.cdata.push(char.clone());
            }
            State::CDataEnding2 if char == &'>' => {
                self.set_state(State::Text);
                self.sax.cdata = String::default();
            }
            State::CDataEnding2 if char == &']' => self.sax.cdata.push(char.clone()),
            State::CDataEnding2 => {
                self.set_state(State::CData);
                self.sax.cdata.push_str("]]");
                self.sax.cdata.push(char.clone());
            }
            State::ProcessingInstruction if char == &'?' => {
                self.set_state(State::ProcessingInstructionEnding)
            }
            State::ProcessingInstruction if is_whitespace(char) => {
                self.set_state(State::ProcessingInstructionBody)
            }
            State::ProcessingInstruction => self.sax.processing_instruction_name.push(char.clone()),
            State::ProcessingInstructionBody if char == &'?' => {
                self.set_state(State::ProcessingInstructionEnding)
            }
            State::ProcessingInstructionBody
                if self.sax.processing_instruction_body.is_empty() && is_whitespace(char) => {}
            State::ProcessingInstructionBody => {
                self.sax.processing_instruction_body.push(char.clone())
            }
            State::ProcessingInstructionEnding if char == &'>' => {
                self.set_state(State::Text);
                self.sax.processing_instruction_body = String::default();
                self.sax.processing_instruction_name = String::default();
            }
            State::ProcessingInstructionEnding => {
                self.set_state(State::ProcessingInstructionBody);
                self.sax.processing_instruction_body.push('?');
                self.sax.processing_instruction_body.push(char.clone());
            }
            State::OpenTag if Name::is_name_char(char) => self.sax.tag_name.push(char.clone()),
            State::OpenTag if char == &'>' => self.open_tag(false),
            State::OpenTag if char == &'/' => self.set_state(State::OpenTagSlash),
            State::OpenTag => {
                if !is_whitespace(char) {
                    self.errors.push(SvgParseError::new_curse(
                        self.get_cursor(),
                        SvgParseErrorMessage::UnexpectedChar(
                            char.clone(),
                            "Valid character in name tag".into(),
                        ),
                    ));
                }
                self.set_state(State::Attribute);
            }
            State::OpenTagSlash if char == &'>' => {
                self.open_tag(true);
                self.close_tag()
            }
            State::OpenTagSlash => {
                self.errors.push(SvgParseError::new_curse(
                    self.get_cursor(),
                    SvgParseErrorMessage::UnexpectedChar(
                        char.clone(),
                        "`>` to end self-closing tag".into(),
                    ),
                ));
                self.set_state(State::Attribute);
            }
            State::Attribute if is_whitespace(char) => {}
            State::Attribute if char == &'>' => self.open_tag(false),
            State::Attribute if char == &'/' => self.set_state(State::OpenTagSlash),
            State::Attribute if Name::is_name_start_char(char) => {
                self.sax.attribute_name = String::from(char.clone());
                self.sax.attribute_value = String::default();
                self.set_state(State::AttributeName);
            }
            State::Attribute => self.errors.push(SvgParseError::new_curse(
                self.get_cursor(),
                SvgParseErrorMessage::UnexpectedChar(
                    char.clone(),
                    "Valid attribute starting character".into(),
                ),
            )),
            State::AttributeName | State::AttributeNameSawWhite if char == &'=' => {
                self.set_state(State::AttributeValue)
            }
            State::AttributeName | State::AttributeNameSawWhite if char == &'>' => {
                if self.options.strict {
                    self.errors.push(SvgParseError::new_curse(
                        self.get_cursor(),
                        SvgParseErrorMessage::UnexpectedChar(
                            char.clone(),
                            "attribute value".into(),
                        ),
                    ))
                }
                self.sax.attribute_value = self.sax.attribute_name.clone();
                self.attribute();
                self.open_tag(false);
            }
            State::AttributeName | State::AttributeNameSawWhite if is_whitespace(char) => {
                self.set_state(State::AttributeNameSawWhite)
            }
            State::AttributeName if Name::is_name_char(char) => {
                self.sax.attribute_name.push(char.clone())
            }
            State::AttributeName => self.errors.push(SvgParseError::new_curse(
                self.get_cursor(),
                SvgParseErrorMessage::UnexpectedChar(char.clone(), "Valid attribute name".into()),
            )),
            State::AttributeNameSawWhite if Name::is_name_start_char(char) => {
                self.set_state(State::AttributeName);
                self.sax.attribute_name = char.clone().into();
            }
            State::AttributeNameSawWhite => {
                self.errors.push(SvgParseError::new_curse(
                    self.get_cursor(),
                    SvgParseErrorMessage::UnexpectedChar(
                        char.clone(),
                        "valid attribute name".into(),
                    ),
                ));
                self.set_state(State::Attribute)
            }
            State::AttributeValue if is_whitespace(char) => {}
            State::AttributeValue if char == &'"' || char == &'\'' => {
                self.set_state(State::AttributeValueQuoted);
                self.sax.quote = Some(char.clone());
            }
            State::AttributeValue => {
                if self.options.strict {
                    self.errors.push(SvgParseError::new_curse(
                        self.get_cursor(),
                        SvgParseErrorMessage::UnexpectedChar(char.clone(), "opening quote".into()),
                    ))
                }
                self.set_state(State::AttributeValueUnquoted);
                self.sax.attribute_value = char.clone().into();
            }
            State::AttributeValueQuoted if char == &'&' => {
                self.set_state(State::AttributeValueEntityQ)
            }
            State::AttributeValueQuoted if Some(char.clone()) != self.sax.quote => {
                self.sax.attribute_value.push(*char)
            }
            State::AttributeValueQuoted => {
                self.attribute();
                self.sax.quote = None;
                self.set_state(State::AttributeValueClosed);
            }
            State::AttributeValueClosed if is_whitespace(char) => self.set_state(State::Attribute),
            State::AttributeValueClosed if char == &'>' => self.open_tag(false),
            State::AttributeValueClosed if char == &'/' => self.set_state(State::OpenTagSlash),
            State::AttributeValueClosed if Name::is_name_start_char(char) => {
                if self.options.strict {
                    self.errors.push(SvgParseError::new_curse(
                        self.get_cursor(),
                        SvgParseErrorMessage::Generic("No whitespace between attributes".into()),
                    ));
                }
                self.set_state(State::AttributeName);
                self.sax.attribute_name = (*char).into();
                self.sax.attribute_value = String::new();
            }
            State::AttributeValueClosed => self.errors.push(SvgParseError::new_curse(
                self.get_cursor(),
                SvgParseErrorMessage::UnexpectedChar(*char, "attribute name".into()),
            )),
            State::AttributeValueUnquoted if char == &'&' => {
                self.set_state(State::AttributeValueEntityU)
            }
            State::AttributeValueUnquoted if char == &'>' => {
                self.attribute();
                self.open_tag(false);
            }
            State::AttributeValueUnquoted if is_whitespace(char) => {
                self.attribute();
                self.set_state(State::Attribute);
            }
            State::AttributeValueUnquoted => self.sax.attribute_value.push(*char),
            State::CloseTag if self.sax.tag_name.is_empty() && is_whitespace(char) => {}
            State::CloseTag if self.sax.tag_name.is_empty() && !Name::is_name_start_char(char) => {
                if !self.sax.script.is_empty() {
                    self.set_state(State::Script);
                    self.sax.script.push_str(&format!("</{}", char));
                    return;
                }
                self.errors.push(SvgParseError::new_curse(
                    self.get_cursor(),
                    SvgParseErrorMessage::UnexpectedChar(*char, "valid tag name".into()),
                ));
            }
            State::CloseTag if Name::is_name_char(char) => self.sax.tag_name.push(*char),
            State::CloseTag if char == &'>' => self.close_tag(),
            State::CloseTag if !self.sax.script.is_empty() => {
                self.set_state(State::Script);
                self.sax.script.push_str(&format!("</{}", char));
                self.sax.tag_name = String::new();
            }
            State::CloseTag if is_whitespace(char) => self.set_state(State::CloseTagSawWhite),
            State::CloseTag => self.errors.push(SvgParseError::new_curse(
                self.get_cursor(),
                SvgParseErrorMessage::UnexpectedChar(*char, "valid tag name".into()),
            )),
            State::CloseTagSawWhite if is_whitespace(char) => {}
            State::CloseTagSawWhite if char == &'>' => self.close_tag(),
            State::CloseTagSawWhite => self.errors.push(SvgParseError::new_curse(
                self.get_cursor(),
                SvgParseErrorMessage::UnexpectedChar(*char, "end of closing tag".into()),
            )),
            State::TextEntity | State::AttributeValueEntityU | State::AttributeValueEntityQ => {
                let return_state = match self.sax.state {
                    State::TextEntity => State::Text,
                    State::AttributeValueEntityU => State::AttributeValueUnquoted,
                    State::AttributeValueEntityQ => State::AttributeValueQuoted,
                    _ => State::Text,
                };
                match char {
                    ';' => {
                        let (entity, is_tag) = self.parse_entity();
                        if !is_tag {
                            self.apply_entity(&entity);
                        } else {
                            todo!();
                        }
                        self.sax.entity = String::new();
                        self.set_state(return_state);
                    }
                    c if self.sax.entity.is_empty()
                        && (Name::is_name_start_char(c) || c == &'#') =>
                    {
                        self.sax.entity.push(*c);
                    }
                    c if !self.sax.entity.is_empty() && (Name::is_name_char(c) || c == &'#') => {
                        self.sax.entity.push(*c);
                    }
                    _ => {
                        self.apply_entity(&format!("&{};", self.sax.entity));
                        self.sax.entity = String::new();
                        self.set_state(return_state);
                    }
                }
            }
        }
    }

    fn begin_whitespace(&mut self, char: &char) {
        if char == &'<' {
            self.set_state(State::OpenWaka);
            self.sax.start_tag_position = self.get_cursor();
            return;
        }
        if self.options.strict {
            self.errors.push(SvgParseError::new_curse(
                self.get_cursor(),
                SvgParseErrorMessage::TextBeforeFirstTag,
            ));
        }
        self.sax.text_node = String::from(char.clone());
        self.set_state(State::Text);
    }

    fn text(&mut self, char: &char) {
        if !is_whitespace(char)
            && self.options.strict
            && (!self.sax.saw_root || self.sax.closed_root)
        {
            self.errors.push(SvgParseError::new_curse(
                self.get_cursor(),
                SvgParseErrorMessage::TextOutsideRoot,
            ));
            return;
        }
        match char {
            '&' => self.set_state(State::TextEntity),
            '<' => {
                self.set_state(State::OpenWaka);
                self.sax.start_tag_position = self.get_cursor();
            }
            _ => self.sax.text_node.push(char.clone()),
        };
    }

    fn open_tag(&mut self, self_closing: bool) {
        if self_closing {
            self.set_state(match self.sax.tag_name.to_lowercase() == "script" {
                true => State::Script,
                false => State::Text,
            });
        } else {
            self.set_state(State::Text);
        }
        let start_tag = STag::new(self.sax.tag_name.clone(), self.get_cursor());
        let span = self.get_cursor().as_span(0);
        let (parent, ns) = match self.sax.tags.last() {
            Some(p) => {
                let ns = match self.options.xmlns {
                    true => todo!(),
                    false => None,
                };
                (Some(p.clone()), ns)
            }
            None => (None, None),
        };
        if self_closing {
            self.sax.tag = Some(Node::EmptyNode(EmptyElemTag {
                parent,
                tag_name: self.sax.tag_name.clone(),
                attributes: HashMap::new(),
                span,
                ns,
            }));
        } else {
            self.sax.tag = Some(Node::ContentNode((
                Rc::new(RefCell::new(STag {
                    parent,
                    tag_name: self.sax.tag_name.clone(),
                    attributes: HashMap::new(),
                    span,
                    ns,
                })),
                Vec::new(),
                ETag::default(),
            )));
            self.sax.tag = None;
            self.sax.tag_name = "".into();
        }
        self.sax.tag = Some(Node::ContentNode((
            Rc::new(RefCell::new(start_tag)),
            Vec::new(),
            ETag::default(),
        )));
        self.sax.attribute_map = HashMap::new();
        self.sax.attribute_name = String::default();

        if self.options.xmlns {
            todo!();
        }
    }

    fn close_tag(&mut self) {
        if self.sax.tag_name.is_empty() {
            if self.options.strict {
                self.errors.push(SvgParseError::new_curse(
                    self.get_cursor(),
                    SvgParseErrorMessage::UnexpectedChar('>', "start of tag name".into()),
                ));
            }
            self.sax.text_node = "</>".into();
            self.set_state(State::Text);
            return;
        }

        if !self.sax.script.is_empty() {
            if self.sax.tag_name.to_lowercase() != "script" {
                self.sax
                    .script
                    .push_str(&format!("</{}>", self.sax.tag_name));
                self.sax.tag_name = String::default();
                self.set_state(State::Script);
                return;
            }
            self.sax.script = String::default();
        }

        self.set_state(State::Text);
        let normalised_tag_name = self.sax.tag_name.to_lowercase();
        let mut opening_tag_index = None;
        for (i, close) in self.sax.tags.iter().enumerate().rev() {
            let close: &RefCell<Node> = &*close.borrow();
            let close = &mut *close.borrow_mut();
            match close {
                Node::ContentNode((s, ..)) => {
                    let s = &mut *s.borrow_mut();
                    if s.tag_name.to_lowercase() == normalised_tag_name {
                        opening_tag_index = Some(i);
                        break;
                    }
                }
                Node::EmptyNode(n) if n.tag_name.to_lowercase() == normalised_tag_name => {
                    opening_tag_index = Some(i);
                    break;
                }
                _ => {}
            }
        }

        if opening_tag_index.is_none() {
            self.errors.push(SvgParseError::new_curse(
                self.get_cursor(),
                SvgParseErrorMessage::UnmatchedTag(self.sax.tag_name.clone(), "unknown".into()),
            ));
            self.sax
                .text_node
                .push_str(&format!("</{}>", self.sax.tag_name));
            return;
        }

        for (i, close) in self.sax.tags.iter().enumerate().rev() {
            let close: &RefCell<Node> = &*close.borrow();
            let close = &*close.borrow();
            self.sax.tag_name = match close {
                Node::ContentNode((start_tag, ..)) => {
                    let start_tag: &RefCell<STag> = &*start_tag.borrow();
                    let start_tag: &STag = &*start_tag.borrow();
                    start_tag.tag_name.clone()
                }
                Node::EmptyNode(EmptyElemTag { tag_name, .. }) => tag_name.clone(),
            };
            if opening_tag_index.is_none() || opening_tag_index == Some(i) {
                break;
            }
        }
        if let Some(i) = opening_tag_index {
            for _ in 0..self.sax.tags.len() - i {
                self.sax.tags.pop();
            }
            if i == 0 {
                self.sax.closed_root = true;
            }
        }

        self.sax.tag_name = String::default();
        self.sax.attribute_map = HashMap::new();
        self.sax.attribute_name = String::default();
    }

    fn attribute(&mut self) {
        let cursor = self.get_cursor();
        if self
            .sax
            .attribute_map
            .contains_key(&self.sax.attribute_name)
        {
            self.errors.push(SvgParseError::new_curse(
                self.get_cursor(),
                SvgParseErrorMessage::DuplicateAttribute(self.sax.attribute_name.clone()),
            ));
            self.sax.attribute_name = String::default();
            self.sax.attribute_value = String::default();
            return;
        }

        if self.options.xmlns {
            todo!();
        }

        self.sax.attribute_map.insert(
            self.sax.attribute_name.clone(),
            self.sax.attribute_value.clone(),
        );
        self.sax
            .ordered_attribute_names
            .push(self.sax.attribute_name.clone());
        self.sax.attribute_name = String::new();
        self.sax.attribute_value = String::new();

        if let Some(a) = &mut self.sax.tag {
            match a {
                Node::EmptyNode(ref mut a) => a.attributes = self.sax.attribute_map.clone(),
                Node::ContentNode((r, ..)) => {
                    let r = r.as_ref();
                    r.borrow_mut().attributes = self.sax.attribute_map.clone();
                }
            };
        } else {
            self.errors.push(SvgParseError::new_curse(
                cursor,
                SvgParseErrorMessage::Internal("Attempted to add attribute to nothing".into()),
            ));
        };
    }

    fn parse_entity(&mut self) -> (String, bool) {
        // Lazily build the entity map
        if self.sax.entity_map.is_empty() {
            for &(key, value) in XML_ENTITIES {
                self.sax.entity_map.insert(key.into(), value);
            }
            if self.options.strict {
                for &(key, value) in ENTITIES {
                    self.sax.entity_map.insert(key.into(), value);
                }
            }
        }

        if let Some(value) = self.sax.entity_map.get(&self.sax.entity) {
            return ((*value).into(), false);
        }
        self.sax.entity = self.sax.entity.to_lowercase();
        if let Some(value) = self.sax.entity_map.get(&self.sax.entity) {
            return ((*value).into(), false);
        }
        let num = match &self.sax.entity {
            e if e.starts_with("#x") => u8::from_str_radix(&e[2..e.len() - 1], 16).map_err(|_| ()),
            e if e.starts_with("#") => u8::from_str_radix(&e[1..e.len() - 1], 10).map_err(|_| ()),
            _ => Err(()),
        };
        if let Ok(num) = num {
            let char = from_u32(num as u32);
            if let Some(char) = char {
                return (char.into(), false);
            }
        }
        self.errors.push(SvgParseError::new_curse(
            self.get_cursor(),
            SvgParseErrorMessage::Generic("Invalid character entity".into()),
        ));
        (format!("&{};", self.sax.entity), true)
    }

    fn apply_entity(&mut self, parsed_entity: &str) {
        match self.sax.state {
            State::TextEntity => self.sax.text_node.push_str(parsed_entity),
            State::AttributeValueEntityU | State::AttributeValueEntityQ => {
                self.sax.attribute_value.push_str(parsed_entity)
            }
            _ => {}
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum State {
    /// Leading byte order mark or whitespace
    #[default]
    Begin,
    /// Leading whitespace
    BeginWhitespace,
    /// General stuff
    Text,
    /// &amp; and such
    TextEntity,
    /// `<`
    OpenWaka,
    /// <!BLARG
    SGMLDeclaration,
    /// <!BLARG foo "bar"
    SGMLDeclarationQuoted,
    /// <!DOCTYPE
    Doctype,
    /// <!DOCTYPE "foo
    DoctypeQuoted,
    /// <!DOCTYPE "foo" [ ...
    DoctypeDTD,
    /// <!DOCTYPE "foo" [ "bar
    DoctypeDTDQuoted,
    /// <!--
    Comment,
    /// <!-- foo -
    CommentEnding,
    /// <!-- foo --
    CommentEnded,
    /// <![CDATA[ foo
    CData,
    /// ]
    CDataEnding,
    /// ]]
    CDataEnding2,
    /// <?foo
    ProcessingInstruction,
    /// <?foo bar
    ProcessingInstructionBody,
    /// <?foo bar ?
    ProcessingInstructionEnding,
    /// <foo
    OpenTag,
    /// <foo /
    OpenTagSlash,
    /// <foo \s
    Attribute,
    /// <foo bar
    AttributeName,
    /// <foo bar\s
    AttributeNameSawWhite,
    /// <foo bar=
    AttributeValue,
    /// <foo bar="baz
    AttributeValueQuoted,
    /// <foo bar="baz"
    AttributeValueClosed,
    /// <foo bar=baz
    AttributeValueUnquoted,
    /// <foo bar="&quot;"
    AttributeValueEntityQ,
    /// <foo bar=&quot
    AttributeValueEntityU,
    /// </foo
    CloseTag,
    /// </foo >
    CloseTagSawWhite,
    /// <script>/* ... */
    Script,
    /// <script>/* ... */<
    ScriptEnding,
}
