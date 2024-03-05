use crate::{
    diagnostics::{SvgParseError, SvgParseErrorMessage},
    file_reader::SAXState,
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
                    file_reader.add_error(SvgParseError::new_curse(
                        file_reader.get_position().end,
                        SvgParseErrorMessage::UnencodedLt,
                    ))
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
