//! [3.1 Start-Tags, End-Tags, and Empty-Element Tags](https://www.w3.org/TR/2006/REC-xml11-20060816/#sec-starttags)

use crate::{
    file_reader::SAXState,
    syntactic_constructs::{is_whitespace, Name},
};

use super::{
    entities::{AttributeValueEntityQuoted, AttributeValueEntityUnquoted},
    tags::{CloseTag, OpenTag, OpenTagSlash},
    FileReaderState, State,
};

/// <foo b
pub struct Attribute;
/// <foo bar
pub struct AttributeName;
/// <foo bar \s
pub struct AttributeNameSawWhite;
/// <foo bar=
pub struct AttributeValue;
/// <foo bar="
pub struct AttributeValueQuoted;
/// <foo bar=baz
pub struct AttributeValueUnquoted;
/// <foo bar="baz"
pub struct AttributeValueClosed;

impl FileReaderState for Attribute {
    fn next(self: Box<Self>, sax: &mut SAXState, char: &char) -> Box<dyn FileReaderState> {
        match char {
            c if is_whitespace(c) => self,
            '>' => CloseTag::handle_end(sax),
            '/' => Box::new(OpenTagSlash),
            c if Name::is_name_start_char(c) => {
                sax.attribute_name = String::from(*c);
                sax.attribute_value = String::default();
                Box::new(AttributeName)
            }
            _ => {
                sax.error_char("Not a valid starting character");
                self
            }
        }
    }

    fn id(&self) -> State {
        State::Attribute
    }
}

impl Attribute {
    pub fn handle_end(sax: &mut SAXState) -> Box<dyn FileReaderState> {
        if sax.attribute_map.contains_key(&sax.attribute_name) {
            sax.error_token("Found duplicate attribute");
            sax.attribute_name = String::default();
            sax.attribute_value = String::default();
            return Box::new(Attribute);
        }

        if sax.get_options().xmlns {
            todo!();
        }

        sax.attribute_map.insert(
            std::mem::take(&mut sax.attribute_name),
            std::mem::take(&mut sax.attribute_value),
        );

        if sax.tag.is_root() {
            sax.error_internal("Attempted to add attribute to nothing");
        }
        Box::new(Attribute)
    }
}

impl FileReaderState for AttributeValue {
    fn next(self: Box<Self>, sax: &mut SAXState, char: &char) -> Box<dyn FileReaderState> {
        match char {
            c if is_whitespace(c) => self,
            '"' | '\'' => {
                sax.quote = Some(*char);
                Box::new(AttributeValueQuoted)
            }
            c => {
                if sax.get_options().strict {
                    sax.error_char("Expected opening quote")
                }
                sax.attribute_value = c.to_string();
                Box::new(AttributeValueUnquoted)
            }
        }
    }

    fn id(&self) -> State {
        State::AttributeValue
    }
}

impl FileReaderState for AttributeValueUnquoted {
    fn next(self: Box<Self>, sax: &mut SAXState, char: &char) -> Box<dyn FileReaderState> {
        match char {
            '&' => Box::new(AttributeValueEntityUnquoted),
            '>' => {
                Attribute::handle_end(sax);
                OpenTag::handle_end(sax, false)
            }
            c if is_whitespace(c) => Attribute::handle_end(sax),
            c => {
                sax.attribute_value.push(*c);
                self
            }
        }
    }

    fn id(&self) -> State {
        State::AttributeValueUnquoted
    }
}

impl FileReaderState for AttributeValueQuoted {
    fn next(self: Box<Self>, sax: &mut SAXState, char: &char) -> Box<dyn FileReaderState> {
        match char {
            '&' => Box::new(AttributeValueEntityQuoted),
            c if Some(*c) != sax.quote => {
                sax.attribute_value.push(*c);
                self
            }
            _ => {
                Attribute::handle_end(sax);
                sax.quote = None;
                Box::new(AttributeValueClosed)
            }
        }
    }

    fn id(&self) -> State {
        State::AttributeValueQuoted
    }
}

impl FileReaderState for AttributeValueClosed {
    fn next(self: Box<Self>, sax: &mut SAXState, char: &char) -> Box<dyn FileReaderState> {
        match char {
            c if is_whitespace(c) => Box::new(Attribute),
            '>' => OpenTag::handle_end(sax, false),
            '/' => Box::new(OpenTagSlash),
            c if Name::is_name_start_char(c) => {
                if sax.get_options().strict {
                    sax.error_char("Expected whitespace between attributes");
                }
                sax.attribute_name = (*c).into();
                sax.attribute_value = String::new();
                Box::new(AttributeName)
            }
            _ => {
                sax.error_char("Expected valid starting character for attribute");
                self
            }
        }
    }

    fn id(&self) -> State {
        State::AttributeValueClosed
    }
}

impl FileReaderState for AttributeName {
    fn next(self: Box<Self>, sax: &mut SAXState, char: &char) -> Box<dyn FileReaderState> {
        if let Some(new_state) = Self::despite_whitespace(sax, char) {
            return new_state;
        }
        match char {
            c if Name::is_name_char(c) => {
                sax.attribute_name.push(*c);
                self
            }
            _ => {
                sax.error_char("Expected valid name character");
                self
            }
        }
    }

    fn id(&self) -> State {
        State::AttributeName
    }
}

impl AttributeName {
    fn despite_whitespace(sax: &mut SAXState, char: &char) -> Option<Box<dyn FileReaderState>> {
        match char {
            '=' => Some(Box::new(AttributeValue)),
            '>' => {
                if sax.get_options().strict {
                    sax.error_char("Expected attribute to have value")
                }
                sax.attribute_value = sax.attribute_name.clone();
                Attribute::handle_end(sax);
                Some(OpenTag::handle_end(sax, false))
            }
            c if is_whitespace(c) => Some(Box::new(AttributeNameSawWhite)),
            _ => None,
        }
    }
}

impl FileReaderState for AttributeNameSawWhite {
    fn next(self: Box<Self>, sax: &mut SAXState, char: &char) -> Box<dyn FileReaderState> {
        if let Some(next_state) = AttributeName::despite_whitespace(sax, char) {
            return next_state;
        }
        match char {
            c if Name::is_name_start_char(c) => {
                sax.attribute_name = c.to_string();
                Box::new(AttributeName)
            }
            _ => {
                sax.error_char("Expected valid attribute name character");
                Box::new(Attribute)
            }
        }
    }

    fn id(&self) -> State {
        State::AttributeNameSawWhite
    }
}
