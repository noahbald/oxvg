use std::{
    borrow::BorrowMut, cell::RefCell, collections::HashMap, iter::Peekable, rc::Rc, str::Chars,
};

use crate::{
    cursor::{Cursor, Span},
    diagnostics::SVGError,
    document::Document,
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
    peekable: Peekable<Chars<'a>>,
    cursor: Cursor,
    offset: usize,
    options: SAXOptions,
    state: Box<dyn FileReaderState>,
    sax: SAXState,
    current_state: SAXCollectedState,
}

#[derive(Default)]
pub struct SAXMeta {
    previous: Box<dyn FileReaderState>,
    change_count: u32,
    pub start: usize,
    pub token_start: usize,
    pub end: usize,
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
    pub start_tag_position: usize,
    pub tags: Vec<Rc<RefCell<Element>>>,
    pub tag: Parent,
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
    pub doctype: String,
    pub doctype_data: String,
    pub quote: Option<char>,
    pub entity: String,
    pub entity_map: HashMap<String, char>,
    pub root: Root,
    pub root_tag: Option<Rc<RefCell<Element>>>,
    options: SAXOptions,
    errors: Vec<SVGError>,
}

impl SAXState {
    pub fn get_options(&self) -> &SAXOptions {
        &self.options
    }

    pub fn get_position(&self) -> &SAXMeta {
        &self.state_meta
    }

    pub fn error_char(&mut self, label: &str) {
        self.errors.push(SVGError::new(
            label.into(),
            (self.state_meta.end..self.state_meta.end).into(),
        ))
    }

    pub fn error_state(&mut self, label: &str) {
        self.errors.push(SVGError::new(
            label.into(),
            (self.state_meta.start..self.state_meta.end).into(),
        ))
    }

    pub fn error_token(&mut self, label: &str) {
        self.errors.push(SVGError::new(
            label.into(),
            (self.state_meta.token_start..self.state_meta.end).into(),
        ))
    }

    pub fn error_tag(&mut self, label: &str) {
        self.errors.push(SVGError::new(
            label.into(),
            (self.start_tag_position..self.state_meta.end).into(),
        ))
    }

    pub fn error_internal(&mut self, label: &str) {
        self.errors.push(
            SVGError::new(
                label.into(),
                (self.state_meta.start..self.state_meta.end).into(),
            )
            .with_advice("This is likely a bug with OXVG. Please consider raising a report."),
        )
    }

    pub fn add_error(&mut self, error: SVGError) {
        self.errors.push(error);
    }

    pub fn add_child(&mut self, child: Child) {
        if self.saw_root && !self.closed_root {
            self.tag.push_child(child);
        } else {
            self.root.children.push(Rc::new(RefCell::new(child)));
        }
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
            peekable: "".chars().peekable(),
            cursor: Cursor::default(),
            offset: 0,
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
            peekable: file.chars().peekable(),
            ..FileReader::default()
        }
    }

    pub fn peek(&mut self) -> Option<&char> {
        self.peekable.peek()
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

        self.sax.state_meta.end += 1;
        char
    }
}

impl Into<Document> for FileReader<'_> {
    fn into(self) -> Document {
        Document {
            root: self.sax.root,
            root_element: self.sax.root_tag,
            errors: self.sax.errors,
        }
    }
}

impl<'a> FileReader<'a> {
    /// Collects the entire file, returning the generated `Root`
    ///
    /// # Examples
    /// ```
    /// let file_reader = FileReader::new("<svg></svg>");
    ///
    /// let root = file_reader.collect_root();
    /// assert!(matches!(
    ///     &*root.children.get(0).unwrap().borrow(),
    ///     Child::Element(Element { name, .. }) if name == "svg"
    /// ));
    /// ```
    pub fn collect_root(&mut self) -> &Root {
        let _: String = self.collect();
        &self.sax.root
    }

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
        let new_state = self.state.clone().next(&mut self.sax, char);
        if self.state.id() != new_state.id() {
            self.sax.state_meta.start = self.sax.state_meta.end;
        }
        if self.state.token_id() != new_state.token_id() {
            self.sax.state_meta.token_start = self.sax.state_meta.end;
        }
        self.state = new_state;
    }

    pub fn ended(&self) -> bool {
        let ended: Box<dyn FileReaderState> = Box::new(Ended);
        self.state == ended
    }

    pub fn get_cursor(&self) -> Cursor {
        todo!("Delete me")
    }
}

#[derive(Default, Debug)]
pub struct Root {
    pub children: Vec<Rc<RefCell<Child>>>,
}

#[derive(Default, Debug)]
pub struct Element {
    pub name: String,
    pub attributes: HashMap<String, String>,
    pub children: Vec<Rc<RefCell<Child>>>,
    pub is_self_closing: bool,
}

#[derive(Debug)]
pub enum Child {
    SGMLDeclaration { value: String },
    Doctype { data: String },
    Instruction { name: String, value: String },
    Comment { value: String },
    CData { value: String },
    Text { value: String },
    Element(Element),
}

#[derive(Debug)]
pub enum Parent {
    Root(Root),
    Element(Rc<RefCell<Element>>),
}

impl Parent {
    pub fn push_child(&mut self, child: Child) {
        let child = Rc::new(RefCell::new(child));
        self.push_rc(&child);
    }

    pub fn push_rc(&mut self, child: &Rc<RefCell<Child>>) {
        match self {
            Self::Root(r) => r.children.push(Rc::clone(child)),
            Self::Element(e) => {
                let e: &RefCell<Element> = e.borrow_mut();
                e.borrow_mut().children.push(Rc::clone(child))
            }
        }
    }

    pub fn is_root(&self) -> bool {
        matches!(self, Self::Root(_))
    }
}

impl Default for Parent {
    fn default() -> Self {
        Parent::Root(Root::default())
    }
}
