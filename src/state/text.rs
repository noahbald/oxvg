use crate::{
    diagnostics::{SvgParseError, SvgParseErrorMessage},
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
    fn next(
        self: Box<Self>,
        file_reader: &mut crate::file_reader::SAXState,
        char: &char,
    ) -> Box<dyn FileReaderState> {
        if !is_whitespace(char)
            && file_reader.get_options().strict
            && (!file_reader.saw_root || file_reader.closed_root)
        {
            file_reader.add_error(SvgParseError::new_curse(
                file_reader.get_position().end,
                SvgParseErrorMessage::TextOutsideRoot,
            ));
            return self;
        }
        match char {
            '&' => Box::new(TextEntity),
            '<' => {
                file_reader.start_tag_position = file_reader.get_position().end;
                Box::new(NodeStart)
            }
            _ => {
                file_reader.text_node.push(*char);
                self
            }
        }
    }

    fn id(&self) -> State {
        State::Text
    }
}

impl FileReaderState for Script {
    fn next(
        self: Box<Self>,
        file_reader: &mut crate::file_reader::SAXState,
        char: &char,
    ) -> Box<dyn FileReaderState>
    where
        Self: std::marker::Sized,
    {
        match char {
            '<' => Box::new(ScriptEnding),
            _ => {
                file_reader.script.push(*char);
                self
            }
        }
    }

    fn id(&self) -> State {
        State::Script
    }
}

impl FileReaderState for ScriptEnding {
    fn next(
        self: Box<Self>,
        file_reader: &mut crate::file_reader::SAXState,
        char: &char,
    ) -> Box<dyn FileReaderState>
    where
        Self: std::marker::Sized,
    {
        match char {
            '/' => Box::new(CloseTag),
            _ => {
                file_reader.script.push('<');
                file_reader.script.push(*char);
                Box::new(Script)
            }
        }
    }

    fn id(&self) -> State {
        State::ScriptEnding
    }
}
