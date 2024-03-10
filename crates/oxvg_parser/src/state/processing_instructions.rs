use crate::{
    file_reader::{Child, SAXState},
    syntactic_constructs::whitespace,
};

use super::{text::Text, State, ID};

/// <?foo
pub struct ProcessingInstruction;
/// <?foo bar
pub struct ProcessingInstructionBody;
/// <?foo bar ?
pub struct ProcessingInstructionEnding;

impl State for ProcessingInstruction {
    fn next(self: Box<Self>, sax: &mut SAXState, char: char) -> Box<dyn State> {
        match char {
            '?' => Box::new(ProcessingInstructionEnding),
            c if whitespace::is(c) => Box::new(ProcessingInstructionBody),
            c => {
                sax.processing_instruction_name.push(c);
                self
            }
        }
    }

    fn id(&self) -> ID {
        ID::ProcessingInstruction
    }
}

impl State for ProcessingInstructionBody {
    fn next(self: Box<Self>, sax: &mut SAXState, char: char) -> Box<dyn State> {
        match char {
            '?' => Box::new(ProcessingInstructionEnding),
            c if sax.processing_instruction_body.is_empty() && whitespace::is(c) => self,
            c => {
                sax.processing_instruction_body.push(c);
                self
            }
        }
    }

    fn id(&self) -> ID {
        ID::ProcessingInstructionBody
    }
}

impl State for ProcessingInstructionEnding {
    fn next(self: Box<Self>, sax: &mut SAXState, char: char) -> Box<dyn State> {
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
                sax.processing_instruction_body.push(c);
                Box::new(ProcessingInstructionBody)
            }
        }
    }

    fn id(&self) -> ID {
        ID::ProcessingInstructionEnding
    }
}
