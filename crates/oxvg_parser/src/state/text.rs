use crate::{
    file_reader::{Child, SAXState},
    syntactic_constructs::is_whitespace,
};

use super::{entities::TextEntity, node_start::NodeStart, tags::CloseTag, FileReaderState, State};

/// General content
pub struct Text;
/// <script>/* ... */
pub struct Script;
/// <script>/* ... */<
pub struct ScriptEnding;

impl FileReaderState for Text {
    fn next(self: Box<Self>, sax: &mut SAXState, char: &char) -> Box<dyn FileReaderState> {
        if !is_whitespace(char) && sax.get_options().strict && (!sax.saw_root || sax.closed_root) {
            sax.error_char("Text outside the root element should be avoided");
            return self;
        }
        match char {
            '&' => Box::new(TextEntity),
            '<' => {
                sax.start_tag_position = sax.get_position().end;
                let child = Child::Text {
                    value: std::mem::take(&mut sax.text_node),
                };
                sax.add_child(child);
                Box::new(NodeStart)
            }
            _ => {
                sax.text_node.push(*char);
                self
            }
        }
    }

    fn id(&self) -> State {
        State::Text
    }
}

impl FileReaderState for Script {
    fn next(self: Box<Self>, sax: &mut SAXState, char: &char) -> Box<dyn FileReaderState> {
        match char {
            '<' => Box::new(ScriptEnding),
            _ => {
                sax.script.push(*char);
                self
            }
        }
    }

    fn id(&self) -> State {
        State::Script
    }
}

impl FileReaderState for ScriptEnding {
    fn next(self: Box<Self>, sax: &mut SAXState, char: &char) -> Box<dyn FileReaderState> {
        match char {
            '/' => Box::new(CloseTag),
            _ => {
                sax.script.push('<');
                sax.script.push(*char);
                Box::new(Script)
            }
        }
    }

    fn id(&self) -> State {
        State::ScriptEnding
    }
}
