use crate::{
    diagnostics::{SvgParseError, SvgParseErrorMessage},
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
    fn next(
        self: Box<Self>,
        file_reader: &mut crate::file_reader::SAXState,
        char: &char,
    ) -> Box<dyn FileReaderState>
    where
        Self: std::marker::Sized,
    {
        match char {
            c if is_whitespace(c) => self,
            '>' => CloseTag::handle_end(file_reader),
            '/' => Box::new(OpenTagSlash),
            c if Name::is_name_start_char(c) => {
                file_reader.attribute_name = String::from(*c);
                file_reader.attribute_value = String::default();
                Box::new(AttributeName)
            }
            c => {
                file_reader.add_error(SvgParseError::new_curse(
                    file_reader.get_position().end,
                    SvgParseErrorMessage::UnexpectedChar(
                        *c,
                        "Valid attribute starting character".into(),
                    ),
                ));
                self
            }
        }
    }

    fn id(&self) -> State {
        State::Attribute
    }
}

impl Attribute {
    pub fn handle_end(file_reader: &mut SAXState) -> Box<dyn FileReaderState> {
        let cursor = file_reader.get_position().end;
        if file_reader
            .attribute_map
            .contains_key(&file_reader.attribute_name)
        {
            file_reader.add_error(SvgParseError::new_curse(
                file_reader.get_position().end,
                SvgParseErrorMessage::DuplicateAttribute(file_reader.attribute_name.clone()),
            ));
            file_reader.attribute_name = String::default();
            file_reader.attribute_value = String::default();
            return Box::new(Attribute);
        }

        if file_reader.get_options().xmlns {
            todo!();
        }

        file_reader.attribute_map.insert(
            file_reader.attribute_name.clone(),
            file_reader.attribute_value.clone(),
        );
        file_reader
            .ordered_attribute_names
            .push(file_reader.attribute_name.clone());
        file_reader.attribute_name = String::new();
        file_reader.attribute_value = String::new();

        if let Some(a) = &file_reader.tag {
            let tag = &mut *a.borrow_mut();
            tag.attributes = file_reader.attribute_map.clone();
        } else {
            file_reader.add_error(SvgParseError::new_curse(
                cursor,
                SvgParseErrorMessage::Internal("Attempted to add attribute to nothing".into()),
            ));
        };
        Box::new(Attribute)
    }
}

impl FileReaderState for AttributeValue {
    fn next(self: Box<Self>, file_reader: &mut SAXState, char: &char) -> Box<dyn FileReaderState>
    where
        Self: std::marker::Sized,
    {
        match char {
            c if is_whitespace(c) => self,
            '"' | '\'' => {
                file_reader.quote = Some(*char);
                Box::new(AttributeValueQuoted)
            }
            c => {
                if file_reader.get_options().strict {
                    file_reader.add_error(SvgParseError::new_curse(
                        file_reader.get_position().end,
                        SvgParseErrorMessage::UnexpectedChar(*c, "opening quote".into()),
                    ))
                }
                file_reader.attribute_value = c.to_string();
                Box::new(AttributeValueUnquoted)
            }
        }
    }

    fn id(&self) -> State {
        State::AttributeValue
    }
}

impl FileReaderState for AttributeValueUnquoted {
    fn next(
        self: Box<Self>,
        file_reader: &mut crate::file_reader::SAXState,
        char: &char,
    ) -> Box<dyn FileReaderState>
    where
        Self: std::marker::Sized,
    {
        match char {
            '&' => Box::new(AttributeValueEntityUnquoted),
            '>' => {
                Attribute::handle_end(file_reader);
                OpenTag::handle_end(file_reader, false)
            }
            c if is_whitespace(c) => Attribute::handle_end(file_reader),
            c => {
                file_reader.attribute_value.push(*c);
                self
            }
        }
    }

    fn id(&self) -> State {
        State::AttributeValueUnquoted
    }
}

impl FileReaderState for AttributeValueQuoted {
    fn next(self: Box<Self>, file_reader: &mut SAXState, char: &char) -> Box<dyn FileReaderState>
    where
        Self: std::marker::Sized,
    {
        match char {
            '&' => Box::new(AttributeValueEntityQuoted),
            c if Some(*c) != file_reader.quote => {
                file_reader.attribute_value.push(*c);
                self
            }
            _ => {
                Attribute::handle_end(file_reader);
                file_reader.quote = None;
                Box::new(AttributeValueClosed)
            }
        }
    }

    fn id(&self) -> State {
        State::AttributeValueQuoted
    }
}

impl FileReaderState for AttributeValueClosed {
    fn next(self: Box<Self>, file_reader: &mut SAXState, char: &char) -> Box<dyn FileReaderState>
    where
        Self: std::marker::Sized,
    {
        match char {
            c if is_whitespace(c) => Box::new(Attribute),
            '>' => OpenTag::handle_end(file_reader, false),
            '/' => Box::new(OpenTagSlash),
            c if Name::is_name_start_char(c) => {
                if file_reader.get_options().strict {
                    file_reader.add_error(SvgParseError::new_curse(
                        file_reader.get_position().end,
                        SvgParseErrorMessage::Generic("No whitespace between attributes".into()),
                    ));
                }
                file_reader.attribute_name = (*c).into();
                file_reader.attribute_value = String::new();
                Box::new(AttributeName)
            }
            c => {
                file_reader.add_error(SvgParseError::new_curse(
                    file_reader.get_position().end,
                    SvgParseErrorMessage::UnexpectedChar(*c, "attribute name".into()),
                ));
                self
            }
        }
    }

    fn id(&self) -> State {
        State::AttributeValueClosed
    }
}

impl FileReaderState for AttributeName {
    fn next(self: Box<Self>, file_reader: &mut SAXState, char: &char) -> Box<dyn FileReaderState>
    where
        Self: std::marker::Sized,
    {
        if let Some(new_state) = Self::despite_whitespace(file_reader, char) {
            return new_state;
        }
        match char {
            c if Name::is_name_char(c) => {
                file_reader.attribute_name.push(*c);
                self
            }
            c => {
                file_reader.add_error(SvgParseError::new_curse(
                    file_reader.get_position().end,
                    SvgParseErrorMessage::UnexpectedChar(*c, "Valid attribute name".into()),
                ));
                self
            }
        }
    }

    fn id(&self) -> State {
        State::AttributeName
    }
}

impl AttributeName {
    fn despite_whitespace(
        file_reader: &mut SAXState,
        char: &char,
    ) -> Option<Box<dyn FileReaderState>> {
        match char {
            '=' => Some(Box::new(AttributeValue)),
            '>' => {
                if file_reader.get_options().strict {
                    file_reader.add_error(SvgParseError::new_curse(
                        file_reader.get_position().end,
                        SvgParseErrorMessage::UnexpectedChar(*char, "attribute value".into()),
                    ))
                }
                file_reader.attribute_value = file_reader.attribute_name.clone();
                Attribute::handle_end(file_reader);
                Some(OpenTag::handle_end(file_reader, false))
            }
            c if is_whitespace(c) => Some(Box::new(AttributeNameSawWhite)),
            _ => None,
        }
    }
}

impl FileReaderState for AttributeNameSawWhite {
    fn next(self: Box<Self>, file_reader: &mut SAXState, char: &char) -> Box<dyn FileReaderState>
    where
        Self: std::marker::Sized,
    {
        if let Some(next_state) = AttributeName::despite_whitespace(file_reader, char) {
            return next_state;
        }
        match char {
            c if Name::is_name_start_char(c) => {
                file_reader.attribute_name = c.to_string();
                Box::new(AttributeName)
            }
            c => {
                file_reader.add_error(SvgParseError::new_curse(
                    file_reader.get_position().end,
                    SvgParseErrorMessage::UnexpectedChar(*c, "valid attribute name".into()),
                ));
                Box::new(Attribute)
            }
        }
    }

    fn id(&self) -> State {
        State::AttributeNameSawWhite
    }
}
