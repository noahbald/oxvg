use std::{cell::RefCell, rc::Rc};

use crate::{
    file_reader::{Element, Parent, SAXState},
    syntactic_constructs::{is_whitespace, Name},
};

use super::{
    declarations::SGMLDeclaration,
    processing_instructions::ProcessingInstruction,
    tags::{CloseTag, OpenTag},
    text::Text,
    FileReaderState, State,
};

/// `<`
pub struct NodeStart;

impl FileReaderState for NodeStart {
    fn next(self: Box<Self>, file_reader: &mut SAXState, char: &char) -> Box<dyn FileReaderState> {
        match char {
            &'!' => {
                file_reader.sgml_declaration = String::new();
                Box::new(SGMLDeclaration)
            }
            char if is_whitespace(char) => self,
            char if Name::is_name_start_char(char) => {
                let new_element = Rc::new(RefCell::new(Element::default()));
                file_reader.tag = Parent::Element(new_element);
                file_reader.tag_name = String::from(*char);
                Box::new(OpenTag)
            }
            &'/' => {
                file_reader.tag_name = String::new();
                Box::new(CloseTag)
            }
            &'?' => {
                file_reader.processing_instruction_body = String::new();
                Box::new(ProcessingInstruction)
            }
            c => {
                if file_reader.get_options().strict {
                    file_reader.error_char("Unencoded `<` should be avoided")
                }
                file_reader.text_node.push('<');
                file_reader.text_node.push(*c);
                Box::new(Text)
            }
        }
    }

    fn id(&self) -> State {
        State::NodeStart
    }
}
