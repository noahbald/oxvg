use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::{
    diagnostics::{SvgParseError, SvgParseErrorMessage},
    file_reader::{Element, SAXState},
    syntactic_constructs::{is_whitespace, Name},
};

use super::{
    attributes::Attribute,
    text::{Script, Text},
    FileReaderState, State,
};

/// <foo
pub struct OpenTag;
/// <foo /
pub struct OpenTagSlash;
/// </foo
pub struct CloseTag;
/// <foo \s
pub struct CloseTagSawWhite;

impl FileReaderState for OpenTag {
    fn next(
        self: Box<Self>,
        file_reader: &mut crate::file_reader::SAXState,
        char: &char,
    ) -> Box<dyn FileReaderState>
    where
        Self: std::marker::Sized,
    {
        match char {
            c if Name::is_name_char(c) => {
                file_reader.tag_name.push(*c);
                self
            }
            '>' => Self::handle_end(file_reader, false),
            '/' => Box::new(OpenTagSlash),
            c => {
                if !is_whitespace(c) {
                    file_reader.add_error(SvgParseError::new_curse(
                        file_reader.get_position().end,
                        SvgParseErrorMessage::UnexpectedChar(
                            *c,
                            "Valid character in name tag".into(),
                        ),
                    ));
                }
                Box::new(Attribute)
            }
        }
    }

    fn id(&self) -> State {
        State::OpenTag
    }
}

impl OpenTag {
    pub fn handle_end(
        file_reader: &mut SAXState,
        is_self_closing: bool,
    ) -> Box<dyn FileReaderState> {
        let state: Box<dyn FileReaderState> =
            match !is_self_closing && file_reader.tag_name.to_lowercase() == "script" {
                true => Box::new(Script),
                false => Box::new(Text),
            };
        match &file_reader.tag {
            Some(t) => {
                t.borrow_mut().is_self_closing = is_self_closing;
                file_reader.tags.push(Rc::clone(t))
            }
            None => file_reader.tags.push(Rc::new(RefCell::new(Element {
                name: file_reader.tag_name.clone(),
                attributes: HashMap::new(),
                children: Vec::new(),
                is_self_closing,
            }))),
        };
        if !is_self_closing {
            file_reader.tag = None;
            file_reader.tag_name = String::new();
        }
        file_reader.attribute_map = HashMap::new();
        file_reader.attribute_name = String::default();

        if file_reader.get_options().xmlns {
            todo!();
        }
        state
    }
}

impl FileReaderState for OpenTagSlash {
    fn next(self: Box<Self>, file_reader: &mut SAXState, char: &char) -> Box<dyn FileReaderState>
    where
        Self: std::marker::Sized,
    {
        match char {
            '>' => {
                OpenTag::handle_end(file_reader, true);
                CloseTag::handle_end(file_reader)
            }
            _ => {
                file_reader.add_error(SvgParseError::new_curse(
                    file_reader.get_position().end,
                    SvgParseErrorMessage::UnexpectedChar(
                        *char,
                        "`>` to end self-closing tag".into(),
                    ),
                ));
                Box::new(Attribute)
            }
        }
    }

    fn id(&self) -> State {
        State::OpenTagSlash
    }
}

impl FileReaderState for CloseTag {
    fn next(self: Box<Self>, file_reader: &mut SAXState, char: &char) -> Box<dyn FileReaderState>
    where
        Self: std::marker::Sized,
    {
        match char {
            c if file_reader.tag_name.is_empty() && is_whitespace(c) => self,
            c if file_reader.tag_name.is_empty() && !Name::is_name_start_char(c) => {
                if !file_reader.script.is_empty() {
                    file_reader.script.push_str(&format!("</{}", c));
                    return Box::new(Script);
                }
                file_reader.add_error(SvgParseError::new_curse(
                    file_reader.get_position().end,
                    SvgParseErrorMessage::UnexpectedChar(*c, "valid tag name".into()),
                ));
                self
            }
            c if Name::is_name_char(c) => {
                file_reader.tag_name.push(*c);
                self
            }
            '>' => Self::handle_end(file_reader),
            c if !file_reader.script.is_empty() => {
                file_reader.script.push_str(&format!("</{}", c));
                file_reader.tag_name = String::new();
                Box::new(Script)
            }
            c if is_whitespace(c) => Box::new(CloseTagSawWhite),
            c => {
                file_reader.add_error(SvgParseError::new_curse(
                    file_reader.get_position().end,
                    SvgParseErrorMessage::UnexpectedChar(*c, "valid tag name".into()),
                ));
                self
            }
        }
    }

    fn id(&self) -> State {
        State::CloseTag
    }
}

impl CloseTag {
    pub fn handle_end(file_reader: &mut SAXState) -> Box<dyn FileReaderState> {
        if file_reader.tag_name.is_empty() {
            if file_reader.get_options().strict {
                file_reader.add_error(SvgParseError::new_curse(
                    file_reader.get_position().end,
                    SvgParseErrorMessage::UnexpectedChar('>', "start of tag name".into()),
                ));
            }
            file_reader.text_node = "</>".into();
            return Box::new(Text);
        }

        if !file_reader.script.is_empty() {
            if file_reader.tag_name.to_lowercase() != "script" {
                file_reader
                    .script
                    .push_str(&format!("</{}>", file_reader.tag_name));
                file_reader.tag_name = String::default();
                return Box::new(Script);
            }
            file_reader.script = String::default();
        }

        let new_state = Box::new(Text);
        let normalised_tag_name = file_reader.tag_name.to_lowercase();
        // Find the matching opening tag, it should be at the end of `sax.tags`, unless...
        // <a><b></c></b></a>
        let mut opening_tag_index = None;
        for (i, matching_open) in file_reader.tags.iter().enumerate().rev() {
            let matching_open = &mut *matching_open.borrow_mut();
            if matching_open.is_self_closing {
                continue;
            }
            if matching_open.name.to_lowercase() == normalised_tag_name {
                opening_tag_index = Some(i);
                break;
            }
        }

        // No matching tag, abort!
        if opening_tag_index.is_none() {
            file_reader.add_error(SvgParseError::new_curse(
                file_reader.get_position().end,
                SvgParseErrorMessage::UnmatchedTag(file_reader.tag_name.clone(), "unknown".into()),
            ));
            file_reader
                .text_node
                .push_str(&format!("</{}>", file_reader.tag_name));
            return new_state;
        }

        // Say goodbye to our opening tag, and any baddies between us
        if let Some(i) = opening_tag_index {
            for _ in 0..file_reader.tags.len() - i {
                file_reader.tags.pop();
            }
            if i == 0 {
                file_reader.closed_root = true;
            }
        }

        file_reader.tag_name = String::default();
        file_reader.attribute_map = HashMap::new();
        file_reader.attribute_name = String::default();
        new_state
    }
}

impl FileReaderState for CloseTagSawWhite {
    fn next(self: Box<Self>, file_reader: &mut SAXState, char: &char) -> Box<dyn FileReaderState>
    where
        Self: std::marker::Sized,
    {
        match char {
            c if is_whitespace(c) => self,
            '>' => CloseTag::handle_end(file_reader),
            c => {
                file_reader.add_error(SvgParseError::new_curse(
                    file_reader.get_position().end,
                    SvgParseErrorMessage::UnexpectedChar(*c, "end of closing tag".into()),
                ));
                self
            }
        }
    }

    fn id(&self) -> State {
        State::CloseTagSawWhite
    }
}
