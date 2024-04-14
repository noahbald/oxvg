use oxvg_ast::{Child, Document, Element, Parent, Root};
use oxvg_diagnostics::SVGError;
use quick_xml::{events::Event, Reader};
use std::{borrow::BorrowMut, cell::RefCell, rc::Rc};

/// A sax style parser for XML written for SVG.
/// This parser works as a state machine, changing from state-to-state as it arrives at different
/// parts of the syntax.
/// `FileReader` is designed so that when a state is left,
pub struct FileReader<'a>(Reader<&'a [u8]>);

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
        file_reader.read()
    }

    pub fn new(file: &'a str) -> Self {
        let reader = Reader::from_str(file);
        FileReader(reader)
    }

    pub fn strict(&mut self) {
        self.0.check_comments(true);
    }

    fn read(&mut self) -> Document {
        let reader = &mut self.0;
        let root = Rc::new(RefCell::new(Root::default()));
        let mut unclosed_elements: Vec<Rc<RefCell<Element>>> = Vec::new();
        let mut root_element = None;
        let mut errors = Vec::new();

        loop {
            use Event::{CData, Comment, Decl, DocType, Empty, End, Eof, Start, Text, PI};
            let mut parent = match unclosed_elements.last() {
                Some(element) => Parent::Element(element.clone()),
                None => Parent::Root(root.clone()),
            };

            match reader.read_event() {
                Err(error) => errors.push((error, reader.buffer_position()).into()),
                Ok(Eof) => {
                    if !unclosed_elements.is_empty() {
                        errors.push(SVGError::new("File ended with unclosed elements", None));
                    }
                    break;
                }
                Ok(Start(tag)) => {
                    let _attributes: Vec<_> = tag.attributes().collect();
                    let element = match Element::new(&tag, &parent, false, reader.buffer_position())
                    {
                        Ok(element) => element,
                        Err(error) => {
                            errors.push((error, reader.buffer_position()).into());
                            continue;
                        }
                    };
                    unclosed_elements.push(element.clone());
                    parent.push_child(Child::Element(element.clone()));
                    if root_element.is_none() {
                        root_element = Some(element.clone());
                    }
                }
                Ok(Empty(tag)) => {
                    let element = match Element::new(&tag, &parent, true, reader.buffer_position())
                    {
                        Ok(element) => element,
                        Err(error) => {
                            errors.push((error, reader.buffer_position()).into());
                            continue;
                        }
                    };
                    parent.push_child(Child::Element(element.clone()));
                    if root_element.is_none() {
                        root_element = Some(element.clone());
                    }
                }
                Ok(End(tag)) => {
                    let name = tag.name();
                    let Some((i, mut element)) = unclosed_elements
                        .iter()
                        .enumerate()
                        .rev()
                        .find(|(_, element)| element.borrow().name() == name)
                    else {
                        errors.push(SVGError::new(
                            "No matching opening tag found for element",
                            Some(reader.buffer_position().into()),
                        ));
                        continue;
                    };
                    let element: &RefCell<Element> = element.borrow_mut();

                    if i < unclosed_elements.len() - 1 {
                        let name = tag.name().0;
                        let name = String::from_utf8_lossy(name);
                        errors.push(SVGError::new(
                            &format!("Found unclosed element in tag {name}"),
                            Some(reader.buffer_position().into()),
                        ));
                    }

                    element.borrow_mut().end(tag, reader.buffer_position());
                    unclosed_elements.truncate(i);
                }
                Ok(Text(text)) => parent.push_child(Child::Text(text.into_owned())),
                Ok(CData(c_data)) => parent.push_child(Child::CData(c_data.into_owned())),
                Ok(Comment(comment)) => parent.push_child(Child::Comment(comment.into_owned())),
                Ok(Decl(decl)) => parent.push_child(Child::XMLDeclaration(decl.into_owned())),
                Ok(PI(processing_instruction)) => {
                    parent.push_child(Child::Instruction(processing_instruction.into_owned()));
                }
                Ok(DocType(doc_type)) => parent.push_child(Child::Doctype(doc_type.into_owned())),
            }
        }
        Document {
            root,
            root_element,
            errors,
        }
    }
}
