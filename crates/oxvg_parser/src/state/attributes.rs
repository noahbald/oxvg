//! [3.1 Start-Tags, End-Tags, and Empty-Element Tags](https://www.w3.org/TR/2006/REC-xml11-20060816/#sec-starttags)

use oxvg_ast::Parent;

use crate::{
    file_reader::SAXState,
    syntactic_constructs::{name, whitespace},
};

use super::{
    entities::{AttributeValueEntityQuoted, AttributeValueEntityUnquoted},
    tags::{CloseTag, OpenTag, OpenTagSlash},
    State, ID,
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

impl State for Attribute {
    fn next(self: Box<Self>, sax: &mut SAXState, char: char) -> Box<dyn State> {
        match char {
            c if whitespace::is(c) => self,
            '>' => CloseTag::handle_end(sax),
            '/' => Box::new(OpenTagSlash),
            c if name::is_start(c) => {
                sax.attribute_name = String::from(c);
                sax.attribute_value = String::default();
                Box::new(AttributeName)
            }
            _ => {
                sax.error_char("Not a valid starting character");
                self
            }
        }
    }

    fn id(&self) -> ID {
        ID::Attribute
    }
}

impl Attribute {
    /// This function takes the collected data for the attribute and applies it to the current tag.
    ///
    /// # Side effects
    ///
    /// Calling always applies the following to the given `SAXState`
    /// * Resets `attribute_name` and `attribute_value`
    ///
    /// Calling may apply the following to the given `SAXState`
    /// * Insert `attribute_value` with the key `attribute_name` to `attribute_map`
    /// * Add to the error list
    pub fn handle_end(sax: &mut SAXState) -> Box<dyn State> {
        if sax.attribute_map.contains_key(&sax.attribute_name) {
            sax.error_token("Found duplicate attribute");
            sax.attribute_name = String::default();
            sax.attribute_value = String::default();
            return Box::new(Attribute);
        }

        if sax.get_options().xmlns {
            todo!();
        }

        if let Parent::Element(e) = &sax.tag {
            e.borrow_mut()
                .attributes_order
                .push(sax.attribute_name.clone());
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

impl State for AttributeValue {
    fn next(self: Box<Self>, sax: &mut SAXState, char: char) -> Box<dyn State> {
        match char {
            c if whitespace::is(c) => self,
            '"' | '\'' => {
                sax.quote = Some(char);
                Box::new(AttributeValueQuoted)
            }
            c => {
                if sax.get_options().strict {
                    sax.error_char("Expected opening quote");
                }
                sax.attribute_value = c.to_string();
                Box::new(AttributeValueUnquoted)
            }
        }
    }

    fn id(&self) -> ID {
        ID::AttributeValue
    }
}

impl State for AttributeValueUnquoted {
    fn next(self: Box<Self>, sax: &mut SAXState, char: char) -> Box<dyn State> {
        match char {
            '&' => Box::new(AttributeValueEntityUnquoted),
            '>' => {
                Attribute::handle_end(sax);
                OpenTag::handle_end(sax, false)
            }
            c if whitespace::is(c) => Attribute::handle_end(sax),
            c => {
                sax.attribute_value.push(c);
                self
            }
        }
    }

    fn id(&self) -> ID {
        ID::AttributeValueUnquoted
    }
}

impl State for AttributeValueQuoted {
    fn next(self: Box<Self>, sax: &mut SAXState, char: char) -> Box<dyn State> {
        match char {
            '&' => Box::new(AttributeValueEntityQuoted),
            c if Some(c) != sax.quote => {
                sax.attribute_value.push(c);
                self
            }
            _ => {
                Attribute::handle_end(sax);
                sax.quote = None;
                Box::new(AttributeValueClosed)
            }
        }
    }

    fn id(&self) -> ID {
        ID::AttributeValueQuoted
    }
}

impl State for AttributeValueClosed {
    fn next(self: Box<Self>, sax: &mut SAXState, char: char) -> Box<dyn State> {
        match char {
            c if whitespace::is(c) => Box::new(Attribute),
            '>' => OpenTag::handle_end(sax, false),
            '/' => Box::new(OpenTagSlash),
            c if name::is_start(c) => {
                if sax.get_options().strict {
                    sax.error_char("Expected whitespace between attributes");
                }
                sax.attribute_name = (c).into();
                sax.attribute_value = String::new();
                Box::new(AttributeName)
            }
            _ => {
                sax.error_char("Expected valid starting character for attribute");
                self
            }
        }
    }

    fn id(&self) -> ID {
        ID::AttributeValueClosed
    }
}

impl State for AttributeName {
    fn next(self: Box<Self>, sax: &mut SAXState, char: char) -> Box<dyn State> {
        if let Some(new_state) = Self::despite_whitespace(sax, char) {
            return new_state;
        }
        match char {
            c if name::is(c) => {
                sax.attribute_name.push(c);
                self
            }
            _ => {
                sax.error_char("Expected valid name character");
                self
            }
        }
    }

    fn id(&self) -> ID {
        ID::AttributeName
    }
}

impl AttributeName {
    /// Handles the common transitions for `AttributeName` and `AttributeNameSawWhite`.
    ///
    /// Returns `None` if the given character needs to be processed as per the requirements of
    /// `AttributeName` or `AttributeNameSawWhite`
    fn despite_whitespace(sax: &mut SAXState, char: char) -> Option<Box<dyn State>> {
        match char {
            '=' => Some(Box::new(AttributeValue)),
            '>' => {
                if sax.get_options().strict {
                    sax.error_char("Expected attribute to have value");
                }
                sax.attribute_value = sax.attribute_name.clone();
                Attribute::handle_end(sax);
                Some(OpenTag::handle_end(sax, false))
            }
            c if whitespace::is(c) => Some(Box::new(AttributeNameSawWhite)),
            _ => None,
        }
    }
}

impl State for AttributeNameSawWhite {
    fn next(self: Box<Self>, sax: &mut SAXState, char: char) -> Box<dyn State> {
        if let Some(next_state) = AttributeName::despite_whitespace(sax, char) {
            return next_state;
        }
        match char {
            c if name::is_start(c) => {
                sax.attribute_value = sax.attribute_name.clone();
                Attribute::handle_end(sax);
                sax.attribute_name.push(c);
                Box::new(AttributeName)
            }
            _ => {
                sax.error_char("Expected valid attribute name character");
                Box::new(Attribute)
            }
        }
    }

    fn id(&self) -> ID {
        ID::AttributeNameSawWhite
    }
}
