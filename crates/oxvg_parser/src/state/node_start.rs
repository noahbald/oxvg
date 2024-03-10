use std::{cell::RefCell, rc::Rc};

use crate::{
    file_reader::{Element, Parent, SAXState},
    syntactic_constructs::{names, whitespace},
};

use super::{
    declarations::SGMLDeclaration,
    processing_instructions::ProcessingInstruction,
    tags::{CloseTag, OpenTag},
    text::Text,
    State, ID,
};

/// `<`
pub struct NodeStart;

impl State for NodeStart {
    fn next(self: Box<Self>, sax: &mut SAXState, char: char) -> Box<dyn State> {
        match char {
            '!' => {
                sax.sgml_declaration = String::new();
                Box::new(SGMLDeclaration)
            }
            char if whitespace::is(char) => self,
            char if names::is_start(char) => {
                let new_element = Rc::new(RefCell::new(Element::default()));
                sax.tag = Parent::Element(new_element);
                sax.tag_name = String::from(char);
                Box::new(OpenTag)
            }
            '/' => {
                sax.tag_name = String::new();
                Box::new(CloseTag)
            }
            '?' => {
                sax.processing_instruction_body = String::new();
                Box::new(ProcessingInstruction)
            }
            c => {
                if sax.get_options().strict {
                    sax.error_char("Unencoded `<` should be avoided");
                }
                sax.text_node.push('<');
                sax.text_node.push(c);
                Box::new(Text)
            }
        }
    }

    fn id(&self) -> ID {
        ID::NodeStart
    }
}
