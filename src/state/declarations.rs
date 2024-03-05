use crate::diagnostics::{SvgParseError, SvgParseErrorMessage};

use super::{text::Text, FileReaderState, State};

/// <!BLARG
pub struct SGMLDeclaration;
/// <!BLARG foo "bar"
pub struct SGMLDeclarationQuoted;
/// <![CDATA[ foo
pub struct CData;
/// <![CDATA[ foo ]
pub struct CDataEnding;
/// <![CDATA[ foo ]]
pub struct CDataEnded;
/// <!--
pub struct Comment;
/// <!-- foo -
pub struct CommentEnding;
/// <!-- foo --
pub struct CommentEnded;
/// <!DOCTYPE
pub struct Doctype;
/// <!DOCTYPE "foo
pub struct DoctypeQuoted;
/// <!DOCTYPE "foo" [ ...
pub struct DoctypeDTD;
/// <!DOCTYPE "foo" [ "bar
pub struct DoctypeDTDQuoted;

impl FileReaderState for SGMLDeclaration {
    fn next(
        self: Box<Self>,
        file_reader: &mut crate::file_reader::SAXState,
        char: &char,
    ) -> Box<dyn FileReaderState>
    where
        Self: std::marker::Sized,
    {
        match &file_reader.sgml_declaration {
            d if d.to_uppercase() == "[CDATA[" => {
                file_reader.sgml_declaration = String::default();
                file_reader.cdata = String::default();
                return Box::new(CData);
            }
            d if d == "-" && char == &'-' => {
                file_reader.comment = String::default();
                file_reader.sgml_declaration = String::default();
                return Box::new(Comment);
            }
            d if d.to_uppercase() == "DOCTYPE" => {
                if !file_reader.doctype || file_reader.saw_root {
                    file_reader.add_error(SvgParseError::new_curse(
                        file_reader.get_position().end,
                        SvgParseErrorMessage::InappropriateDoctype,
                    ));
                }
                file_reader.sgml_declaration = String::default();
                return Box::new(Doctype);
            }
            _ => {}
        }
        match char {
            '>' => {
                file_reader.sgml_declaration = String::default();
                Box::new(Text)
            }
            '"' | '\'' => {
                file_reader.sgml_declaration.push(*char);
                file_reader.quote = Some(*char);
                Box::new(SGMLDeclarationQuoted)
            }
            c => {
                file_reader.sgml_declaration.push(*c);
                self
            }
        }
    }

    fn id(&self) -> State {
        State::SGMLDeclaration
    }
}

impl FileReaderState for SGMLDeclarationQuoted {
    fn next(
        self: Box<Self>,
        file_reader: &mut crate::file_reader::SAXState,
        char: &char,
    ) -> Box<dyn FileReaderState>
    where
        Self: std::marker::Sized,
    {
        if Some(*char) == file_reader.quote {
            file_reader.quote = None;
            return Box::new(SGMLDeclaration);
        }
        file_reader.sgml_declaration.push(*char);
        self
    }

    fn id(&self) -> State {
        State::SGMLDeclarationQuoted
    }
}

impl FileReaderState for CData {
    fn next(
        self: Box<Self>,
        file_reader: &mut crate::file_reader::SAXState,
        char: &char,
    ) -> Box<dyn FileReaderState>
    where
        Self: std::marker::Sized,
    {
        match char {
            ']' => Box::new(CDataEnding),
            c => {
                file_reader.cdata.push(*c);
                self
            }
        }
    }

    fn id(&self) -> State {
        State::CData
    }
}

impl FileReaderState for CDataEnding {
    fn next(
        self: Box<Self>,
        file_reader: &mut crate::file_reader::SAXState,
        char: &char,
    ) -> Box<dyn FileReaderState>
    where
        Self: std::marker::Sized,
    {
        match char {
            ']' => Box::new(CDataEnded),
            c => {
                file_reader.cdata.push(']');
                file_reader.cdata.push(*c);
                Box::new(CData)
            }
        }
    }

    fn id(&self) -> State {
        State::CDataEnding
    }
}

impl FileReaderState for CDataEnded {
    fn next(
        self: Box<Self>,
        file_reader: &mut crate::file_reader::SAXState,
        char: &char,
    ) -> Box<dyn FileReaderState>
    where
        Self: std::marker::Sized,
    {
        match char {
            '>' => {
                file_reader.cdata = String::default();
                Box::new(Text)
            }
            ']' => {
                file_reader.cdata.push(*char);
                self
            }
            c => {
                file_reader.cdata.push_str("]]");
                file_reader.cdata.push(*c);
                Box::new(CData)
            }
        }
    }

    fn id(&self) -> State {
        State::CDataEnded
    }
}

impl FileReaderState for Comment {
    fn next(
        self: Box<Self>,
        file_reader: &mut crate::file_reader::SAXState,
        char: &char,
    ) -> Box<dyn FileReaderState>
    where
        Self: std::marker::Sized,
    {
        match char {
            '-' => Box::new(CommentEnding),
            c => {
                file_reader.comment.push(*c);
                self
            }
        }
    }

    fn id(&self) -> State {
        State::Comment
    }
}

impl FileReaderState for CommentEnding {
    fn next(
        self: Box<Self>,
        file_reader: &mut crate::file_reader::SAXState,
        char: &char,
    ) -> Box<dyn FileReaderState>
    where
        Self: std::marker::Sized,
    {
        match char {
            '-' => {
                file_reader.comment = String::default();
                Box::new(CommentEnded)
            }
            c => {
                file_reader.comment.push('-');
                file_reader.comment.push(*c);
                Box::new(Comment)
            }
        }
    }

    fn id(&self) -> State {
        State::CommentEnding
    }
}

impl FileReaderState for CommentEnded {
    fn next(
        self: Box<Self>,
        file_reader: &mut crate::file_reader::SAXState,
        char: &char,
    ) -> Box<dyn FileReaderState>
    where
        Self: std::marker::Sized,
    {
        match char {
            '>' => Box::new(Text),
            c => {
                if file_reader.get_options().strict {
                    file_reader.add_error(SvgParseError::new_curse(
                        file_reader.get_position().end,
                        SvgParseErrorMessage::MalformedComment,
                    ))
                }
                file_reader.comment.push_str("--");
                file_reader.comment.push(*c);
                Box::new(Comment)
            }
        }
    }

    fn id(&self) -> State {
        State::CommentEnded
    }
}

impl FileReaderState for Doctype {
    fn next(
        self: Box<Self>,
        file_reader: &mut crate::file_reader::SAXState,
        char: &char,
    ) -> Box<dyn FileReaderState>
    where
        Self: std::marker::Sized,
    {
        match char {
            '>' => {
                file_reader.doctype = true;
                Box::new(Text)
            }
            '[' => Box::new(DoctypeDTD),
            '"' | '\'' => {
                file_reader.quote = Some(*char);
                Box::new(DoctypeQuoted)
            }
            _ => self,
        }
    }

    fn id(&self) -> State {
        State::Doctype
    }
}

impl FileReaderState for DoctypeDTD {
    fn next(
        self: Box<Self>,
        file_reader: &mut crate::file_reader::SAXState,
        char: &char,
    ) -> Box<dyn FileReaderState>
    where
        Self: std::marker::Sized,
    {
        match char {
            ']' => Box::new(Doctype),
            '"' | '\'' => {
                file_reader.quote = Some(*char);
                Box::new(DoctypeDTDQuoted)
            }
            _ => self,
        }
    }

    fn id(&self) -> State {
        State::DoctypeDTD
    }
}

impl FileReaderState for DoctypeQuoted {
    fn next(
        self: Box<Self>,
        file_reader: &mut crate::file_reader::SAXState,
        char: &char,
    ) -> Box<dyn FileReaderState>
    where
        Self: std::marker::Sized,
    {
        match char {
            c if Some(*c) == file_reader.quote => {
                file_reader.quote = None;
                Box::new(Doctype)
            }
            _ => self,
        }
    }

    fn id(&self) -> State {
        State::DoctypeQuoted
    }
}

impl FileReaderState for DoctypeDTDQuoted {
    fn next(
        self: Box<Self>,
        file_reader: &mut crate::file_reader::SAXState,
        char: &char,
    ) -> Box<dyn FileReaderState>
    where
        Self: std::marker::Sized,
    {
        match char {
            c if Some(*c) == file_reader.quote => {
                file_reader.quote = None;
                Box::new(DoctypeDTD)
            }
            _ => self,
        }
    }

    fn id(&self) -> State {
        State::DoctypeDTDQuoted
    }
}
