use crate::file_reader::SAXState;

use super::{node_start::NodeStart, text::Text, FileReaderState, State};

/// Leading byte mark or whitespace
pub struct Begin;
/// Leading whitespace
pub struct BeginWhitespace;
/// Reached end of file
pub struct Ended;

impl FileReaderState for Begin {
    fn next(self: Box<Self>, file_reader: &mut SAXState, char: &char) -> Box<dyn FileReaderState> {
        let next_state = Box::new(BeginWhitespace);
        if char != &'\u{FEFF}' {
            next_state.next(file_reader, char)
        } else {
            next_state
        }
    }

    fn id(&self) -> State {
        State::Begin
    }
}

impl FileReaderState for BeginWhitespace {
    fn next(self: Box<Self>, file_reader: &mut SAXState, char: &char) -> Box<dyn FileReaderState> {
        if char == &'<' {
            file_reader.start_tag_position = file_reader.get_position().end;
            return Box::new(NodeStart);
        }
        if file_reader.get_options().strict {
            file_reader.error_char("Expected opening of tag or declaration with `<`");
            file_reader.text_node = String::from(*char);
            return Box::new(Text);
        }
        self
    }

    fn id(&self) -> State {
        State::BeginWhitespace
    }
}

impl FileReaderState for Ended {
    fn next(self: Box<Self>, file_reader: &mut SAXState, _char: &char) -> Box<dyn FileReaderState> {
        if file_reader.root_tag.is_none() {
            file_reader.error_char("Couldn't find root element in document")
        }
        self
    }

    fn id(&self) -> State {
        State::Ended
    }
}
