use std::{cell::RefCell, collections::HashMap, iter::Peekable, rc::Rc, str::Chars};

use crate::{
    cursor::{Cursor, Span},
    diagnostics::SvgParseError,
    state::{Begin, Ended, FileReaderState},
};

/// A sax style parser for XML written for SVG.
/// This parser works as a state machine, changing from state-to-state as it arrives at different
/// parts of the syntax.
/// `FileReader` is designed so that when a state is left,
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
    state: Box<dyn FileReaderState>,
    sax: SAXState,
    current_state: SAXCollectedState,
}

#[derive(Default)]
pub struct SAXMeta {
    previous: Box<dyn FileReaderState>,
    change_count: u32,
    pub start: Cursor,
    pub end: Cursor,
    pub size: usize,
}

#[derive(Default)]
pub struct SAXOptions {
    pub strict: bool,
    pub xmlns: bool,
}

#[derive(Default)]
pub struct SAXState {
    state_meta: SAXMeta,
    pub start_tag_position: Cursor,
    pub tags: Vec<Rc<RefCell<Element>>>,
    pub tag: Option<Rc<RefCell<Element>>>,
    pub attribute_map: HashMap<String, String>,
    pub attribute_name: String,
    pub attribute_value: String,
    pub ordered_attribute_names: Vec<String>,
    pub text_node: String,
    pub saw_root: bool,
    pub closed_root: bool,
    pub script: String,
    pub sgml_declaration: String,
    pub tag_name: String,
    pub processing_instruction_name: String,
    pub processing_instruction_body: String,
    pub cdata: String,
    pub comment: String,
    pub doctype: bool,
    pub quote: Option<char>,
    pub entity: String,
    pub entity_map: HashMap<String, char>,
    pub root: Root,
    options: SAXOptions,
    errors: Vec<SvgParseError>,
}

impl SAXState {
    pub fn get_options(&self) -> &SAXOptions {
        &self.options
    }

    pub fn get_position(&self) -> &SAXMeta {
        &self.state_meta
    }

    pub fn add_error(&mut self, error: SvgParseError) {
        self.errors.push(error);
    }
}

#[derive(Default)]
pub struct SAXCollectedState {
    pub contents: String,
    pub span: Span,
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
            state: Box::new(Begin),
            sax: SAXState::default(),
            current_state: SAXCollectedState::default(),
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
            .state_meta
            .start
            .as_span(self.offset - self.sax.state_meta.size)
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
        if let Some(char) = char {
            self.next_state(&char);
        } else {
            self.state = Box::new(Ended);
            return char;
        }

        self.offset += 1;
        if Some('\n') == char {
            self.cursor.mut_newline();
        } else {
            self.cursor.mut_advance();
        }
        char
    }
}

impl<'a> FileReader<'a> {
    /// Collects the file until a new token is started
    ///
    /// This isn't strictly necessary unless you want to avoid multiple passes, since a fully collected
    /// file can be use from `file_reader.root`
    pub fn collect_state(&mut self) -> String {
        let mut contents = String::new();
        let previous_state = &self.state.clone();
        while let Some(c) = self.next() {
            contents.push(c);
            if previous_state != &self.state {
                break;
            }
        }
        dbg!(previous_state.id(), &self.state.id());
        contents
    }

    /// Transitions the state of `FileReader` based on the given char.
    ///
    /// # Arguments
    ///
    /// * `char` - A character of the svg file
    ///
    /// # Examples
    /// ```
    /// let file_reader = FileReader::new("<svg></svg>");
    ///
    /// // The file_reader starts of in the Begin state
    /// assert_eq!(Box::new(Begin), file_reader.sax.state);
    ///
    /// // Providing `<` causes the state to transition into the `NodeStart` state
    /// file_reader.next_state(&'<');
    /// assert_eq!(Box::new(NodeStart), file_reader.sax.state);
    ///
    /// // Depending on the character, other parts of the sax state may change
    /// assert_eq!(Cursor::default(), file_reader.sax.start_tag_position);
    /// ```
    fn next_state(&mut self, char: &char) {
        self.state = self.state.clone().next(&mut self.sax, char);
    }

    pub fn ended(&self) -> bool {
        let ended: Box<dyn FileReaderState> = Box::new(Ended);
        self.state == ended
    }
}

#[derive(Default)]
pub struct Root {
    children: Vec<Rc<RefCell<Child>>>,
}

pub struct Element {
    pub name: String,
    pub attributes: HashMap<String, String>,
    pub children: Vec<Rc<RefCell<Element>>>,
    pub is_self_closing: bool,
}

pub enum Child {
    Doctype { name: String, data: String },
    Instruction { name: String, value: String },
    Comment { value: String },
    CData { value: String },
    Text { value: String },
    Element(Element),
}

pub enum Parent {
    Root(Root),
    Element(Element),
}

pub enum Node {
    Root(Root),
    Child(Child),
}
