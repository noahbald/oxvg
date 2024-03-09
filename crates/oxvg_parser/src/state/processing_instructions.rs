use crate::{
    file_reader::{Child, SAXState},
    syntactic_constructs::is_whitespace,
};

use super::{text::Text, FileReaderState, State};

/// <?foo
pub struct ProcessingInstruction;
/// <?foo bar
pub struct ProcessingInstructionBody;
/// <?foo bar ?
pub struct ProcessingInstructionEnding;

impl FileReaderState for ProcessingInstruction {
    fn next(self: Box<Self>, sax: &mut SAXState, char: &char) -> Box<dyn FileReaderState> {
        match char {
            '?' => Box::new(ProcessingInstructionEnding),
            c if is_whitespace(c) => Box::new(ProcessingInstructionBody),
            c => {
                sax.processing_instruction_name.push(*c);
                self
            }
        }
    }

    fn id(&self) -> State {
        State::ProcessingInstruction
    }
}

impl FileReaderState for ProcessingInstructionBody {
    fn next(self: Box<Self>, sax: &mut SAXState, char: &char) -> Box<dyn FileReaderState> {
        match char {
            '?' => Box::new(ProcessingInstructionEnding),
            c if sax.processing_instruction_body.is_empty() && is_whitespace(c) => self,
            c => {
                sax.processing_instruction_body.push(*c);
                self
            }
        }
    }

    fn id(&self) -> State {
        State::ProcessingInstructionBody
    }
}

impl FileReaderState for ProcessingInstructionEnding {
    fn next(self: Box<Self>, sax: &mut SAXState, char: &char) -> Box<dyn FileReaderState> {
        match char {
            '>' => {
                let child = Child::Instruction {
                    name: std::mem::take(&mut sax.processing_instruction_name),
                    value: std::mem::take(&mut sax.processing_instruction_body),
                };
                sax.add_child(child);
                Box::new(Text)
            }
            c => {
                sax.processing_instruction_body.push('?');
                sax.processing_instruction_body.push(*c);
                Box::new(ProcessingInstructionBody)
            }
        }
    }

    fn id(&self) -> State {
        State::ProcessingInstructionEnding
    }
}
