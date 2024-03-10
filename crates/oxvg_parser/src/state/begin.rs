use crate::file_reader::SAXState;

use super::{node_start::NodeStart, text::Text, State, ID};

/// Leading byte mark or whitespace
pub struct Begin;
/// Leading whitespace
pub struct LeadingWhitespace;
/// Reached end of file
pub struct Ended;

impl State for Begin {
    fn next(self: Box<Self>, sax: &mut SAXState, char: char) -> Box<dyn State> {
        let next_state = Box::new(LeadingWhitespace);
        if char == '\u{FEFF}' {
            next_state
        } else {
            next_state.next(sax, char)
        }
    }

    fn id(&self) -> ID {
        ID::Begin
    }
}

impl State for LeadingWhitespace {
    fn next(self: Box<Self>, sax: &mut SAXState, char: char) -> Box<dyn State> {
        if char == '<' {
            sax.start_tag_position = sax.get_position().end;
            return Box::new(NodeStart);
        }
        if sax.get_options().strict {
            sax.error_char("Expected opening of tag or declaration with `<`");
            sax.text_node = String::from(char);
            return Box::new(Text);
        }
        self
    }

    fn id(&self) -> ID {
        ID::BeginWhitespace
    }
}

impl State for Ended {
    fn next(self: Box<Self>, sax: &mut SAXState, _char: char) -> Box<dyn State> {
        if sax.root_tag.is_none() {
            sax.error_char("Couldn't find root element in document");
        }
        self
    }

    fn id(&self) -> ID {
        ID::Ended
    }
}
