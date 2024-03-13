use oxvg_ast::{Child, Document, Element, Parent, Root};
use oxvg_diagnostics::SVGError;
use std::{
    borrow::BorrowMut, cell::RefCell, collections::HashMap, iter::Peekable, rc::Rc, str::Chars,
};

use crate::{
    state::{Begin, Ended, State},
    syntactic_constructs::character,
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
    state: Box<dyn State>,
    sax: SAXState,
}

#[derive(Default)]
/// Information related to the progress of the sax parser
pub struct SAXMeta {
    pub start: usize,
    pub token_start: usize,
    pub end: usize,
    pub size: usize,
}

#[derive(Default)]
/// User defined options for sax parsing
pub struct SAXOptions {
    /// Enables whether extra error checking as to whether the xml document is well-formed
    pub strict: bool,
    /// Enables whether xml namespaces will be processed
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
    pub root: Rc<RefCell<Root>>,
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
        ));
    }

    pub fn error_state(&mut self, label: &str) {
        self.errors.push(SVGError::new(
            label.into(),
            (self.state_meta.start..self.state_meta.end).into(),
        ));
    }

    pub fn error_token(&mut self, label: &str) {
        self.errors.push(SVGError::new(
            label.into(),
            (self.state_meta.token_start..self.state_meta.end).into(),
        ));
    }

    pub fn error_tag(&mut self, label: &str) {
        self.errors.push(SVGError::new(
            label.into(),
            (self.start_tag_position..self.state_meta.end).into(),
        ));
    }

    pub fn error_internal(&mut self, label: &str) {
        self.errors.push(
            SVGError::new(
                label.into(),
                (self.state_meta.start..self.state_meta.end).into(),
            )
            .with_advice("This is likely a bug with OXVG. Please consider raising a report."),
        );
    }

    pub fn add_error(&mut self, error: SVGError) {
        self.errors.push(error);
    }

    pub fn add_child(&mut self, child: Child) {
        if self.saw_root && !self.closed_root {
            self.tag.push_child(child);
        } else {
            let root: &RefCell<Root> = self.root.borrow_mut();
            root.borrow_mut()
                .children
                .push(Rc::new(RefCell::new(child)));
        }
    }
}

impl<'a> Default for FileReader<'a> {
    fn default() -> Self {
        Self {
            peekable: "".chars().peekable(),
            state: Box::new(Begin),
            sax: SAXState::default(),
        }
    }
}

impl<'a> FileReader<'a> {
    /// Parses the given string, returning a `Document` with the generated tree of elements
    ///
    /// # Example
    /// ```
    /// use oxvg_parser::FileReader;
    ///
    /// let document = FileReader::parse("<svg attr=\"hi\">\n</svg>");
    /// assert!(document.root_element.is_some());
    /// ```
    pub fn parse(svg: &str) -> Document {
        let mut file_reader = FileReader::new(svg);
        file_reader.collect_root();
        file_reader.into()
    }

    pub fn new(file: &'a str) -> Self {
        FileReader {
            peekable: file.chars().peekable(),
            ..FileReader::default()
        }
    }
}

impl<'a> Iterator for FileReader<'a> {
    type Item = char;

    /// Advances the file reader and returns the next value.
    ///
    /// The file reader is a state machine, and consuming next will transition it's state.
    /// Returns `None` when the iterator is finished.
    fn next(&mut self) -> Option<char> {
        let char = self.peekable.next();
        if let Some(char) = char {
            self.next_state(char);

            if self.sax.saw_root && !self.sax.closed_root && character::is_restricted(char) {
                self.sax
                    .error_char("Restricted characters are not allowed in the document");
            }
            if (!self.sax.saw_root || self.sax.closed_root) && !character::is(char) {
                self.sax.error_char(
                    "Disallowed surrogate unicode character now allowed in the document",
                );
            }
        } else {
            self.state = Box::new(Ended);
            return char;
        }

        self.sax.state_meta.end += 1;
        char
    }
}

impl From<FileReader<'_>> for Document {
    fn from(val: FileReader<'_>) -> Self {
        Document {
            root: val.sax.root,
            root_element: val.sax.root_tag,
            errors: val.sax.errors,
        }
    }
}

impl<'a> FileReader<'a> {
    /// Collects the entire file, returning the generated `Root`
    pub fn collect_root(&mut self) -> Rc<RefCell<Root>> {
        let _: String = self.collect();
        self.sax.root.clone()
    }

    /// Transitions the state of `FileReader` based on the given char.
    ///
    /// # Arguments
    ///
    /// * `char` - A character of the svg file
    fn next_state(&mut self, char: char) {
        let new_state = self.state.clone().next(&mut self.sax, char);
        if self.state.id() != new_state.id() {
            self.sax.state_meta.start = self.sax.state_meta.end;
        }
        if self.state.token_id() != new_state.token_id() {
            self.sax.state_meta.token_start = self.sax.state_meta.end;
        }
        self.state = new_state;
    }
}

#[test]
fn file_reader() {
    let file_reader = &mut FileReader::new("<svg></svg>");
    // The file_reader starts of in the Begin state
    assert_eq!(crate::state::ID::Begin, file_reader.state.id());

    // A call to next() returns the next value...
    assert_eq!(Some('<'), file_reader.next());
    // Providing `<` causes the state to transition into the `NodeStart` state
    assert_eq!(crate::state::ID::NodeStart, file_reader.state.id());

    assert_eq!(Some('s'), file_reader.next());
    assert_eq!(Some('v'), file_reader.next());

    // ... and then None once it's over.
    let _: String = file_reader.collect();
    assert_eq!(None, file_reader.next());

    // More calls may or may not return `None`. Here, they always will.
    assert_eq!(None, file_reader.next());

    assert_eq!(None, file_reader.next());

    let root = file_reader.collect_root();
    assert!(matches!(
        &*root.borrow().children.first().unwrap().borrow(),
        Child::Element(Element { name, .. }) if name == "svg"
    ));
}
