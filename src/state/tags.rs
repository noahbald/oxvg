use std::{borrow::BorrowMut, cell::RefCell, collections::HashMap, rc::Rc};

use crate::{
    file_reader::{Child, Element, Parent, SAXState},
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
                    file_reader.error_char("Expected a valid tag name character");
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
        if let Parent::Element(e) = &mut file_reader.tag {
            let element: &RefCell<Element> = e.borrow_mut();
            element.borrow_mut().name = file_reader.tag_name.clone();
            file_reader.tags.push(e.clone());
            if file_reader.root_tag.is_none() {
                file_reader.root_tag = Some(e.clone());
            }
        }
        if !is_self_closing {
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
                file_reader.error_char("Expected a `>` to end self-closing tag");
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
                file_reader.error_char("Expected a valid starting tag name character");
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
            _ => {
                file_reader.error_char("Expected a valid tag name character");
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
                file_reader.error_tag("start of tag name");
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
        for (i, matching_open) in file_reader.tags.iter_mut().enumerate().rev() {
            let e: &RefCell<Element> = matching_open.borrow_mut();
            let e = e.borrow_mut();
            if e.is_self_closing {
                continue;
            }
            if e.name.to_lowercase() == normalised_tag_name {
                opening_tag_index = Some(i);
                break;
            }
        }

        // No matching tag, abort!
        if opening_tag_index.is_none() {
            file_reader.error_tag("Matching opening tag not found");
            file_reader
                .text_node
                .push_str(&format!("</{}>", file_reader.tag_name));
            return new_state;
        }

        // Say goodbye to our opening tag, and any baddies between us
        if let Some(i) = opening_tag_index {
            for _ in 0..file_reader.tags.len() - i - 1 {
                file_reader.tags.pop();
            }
            let opening_tag = file_reader.tags.pop();
            if i == 0 {
                file_reader.closed_root = true;
            }
            if let Some(o) = opening_tag {
                match file_reader.tags.last() {
                    Some(t) => Parent::Element(t.clone()).push_child(Child::Element(o.take())),
                    None => file_reader
                        .root
                        .children
                        .push(Rc::new(RefCell::new(Child::Element(o.take())))),
                };
            } else {
                unreachable!("The opening tag was accidentally lost");
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
            _ => {
                file_reader.error_char("Expected `>` to end closing tag");
                self
            }
        }
    }

    fn id(&self) -> State {
        State::CloseTagSawWhite
    }
}
