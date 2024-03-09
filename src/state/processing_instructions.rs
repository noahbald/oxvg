use crate::{file_reader::Child, syntactic_constructs::is_whitespace};

use super::{text::Text, FileReaderState, State};

/// <?foo
pub struct ProcessingInstruction;
/// <?foo bar
pub struct ProcessingInstructionBody;
/// <?foo bar ?
pub struct ProcessingInstructionEnding;

impl FileReaderState for ProcessingInstruction {
    fn next(
        self: Box<Self>,
        file_reader: &mut crate::file_reader::SAXState,
        char: &char,
    ) -> Box<dyn FileReaderState>
    where
        Self: std::marker::Sized,
    {
        match char {
            '?' => Box::new(ProcessingInstructionEnding),
            c if is_whitespace(c) => Box::new(ProcessingInstructionBody),
            c => {
                file_reader.processing_instruction_name.push(*c);
                self
            }
        }
    }

    fn id(&self) -> State {
        State::ProcessingInstruction
    }
}

impl FileReaderState for ProcessingInstructionBody {
    fn next(
        self: Box<Self>,
        file_reader: &mut crate::file_reader::SAXState,
        char: &char,
    ) -> Box<dyn FileReaderState>
    where
        Self: std::marker::Sized,
    {
        match char {
            '?' => Box::new(ProcessingInstructionEnding),
            c if file_reader.processing_instruction_body.is_empty() && is_whitespace(c) => self,
            c => {
                file_reader.processing_instruction_body.push(*c);
                self
            }
        }
    }

    fn id(&self) -> State {
        State::ProcessingInstructionBody
    }
}

impl FileReaderState for ProcessingInstructionEnding {
    fn next(
        self: Box<Self>,
        file_reader: &mut crate::file_reader::SAXState,
        char: &char,
    ) -> Box<dyn FileReaderState>
    where
        Self: std::marker::Sized,
    {
        match char {
            '>' => {
                let child = Child::Instruction {
                    name: std::mem::take(&mut file_reader.processing_instruction_name),
                    value: std::mem::take(&mut file_reader.processing_instruction_body),
                };
                file_reader.add_child(child);
                Box::new(Text)
            }
            c => {
                file_reader.processing_instruction_body.push('?');
                file_reader.processing_instruction_body.push(*c);
                Box::new(ProcessingInstructionBody)
            }
        }
    }

    fn id(&self) -> State {
        State::ProcessingInstructionEnding
    }
}
