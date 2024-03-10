use crate::{
    file_reader::{Child, SAXState},
    syntactic_constructs::whitespace,
};

use super::{entities::TextEntity, node_start::NodeStart, tags::CloseTag, State, ID};

/// General content
pub struct Text;
/// <script>/* ... */
pub struct Script;
/// <script>/* ... */<
pub struct ScriptEnding;

impl State for Text {
    fn next(self: Box<Self>, sax: &mut SAXState, char: char) -> Box<dyn State> {
        if !whitespace::is(char) && sax.get_options().strict && (!sax.saw_root || sax.closed_root) {
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
            c => {
                sax.text_node.push(c);
                self
            }
        }
    }

    fn id(&self) -> ID {
        ID::Text
    }
}

impl State for Script {
    fn next(self: Box<Self>, sax: &mut SAXState, char: char) -> Box<dyn State> {
        if char == '<' {
            Box::new(ScriptEnding)
        } else {
            sax.script.push(char);
            self
        }
    }

    fn id(&self) -> ID {
        ID::Script
    }
}

impl State for ScriptEnding {
    fn next(self: Box<Self>, sax: &mut SAXState, char: char) -> Box<dyn State> {
        if char == '/' {
            Box::new(CloseTag)
        } else {
            sax.script.push('<');
            sax.script.push(char);
            Box::new(Script)
        }
    }

    fn id(&self) -> ID {
        ID::ScriptEnding
    }
}
