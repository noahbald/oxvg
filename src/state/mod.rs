mod attributes;
mod begin;
mod declarations;
mod entities;
mod node_start;
mod processing_instructions;
mod tags;
mod text;
use crate::file_reader::SAXState;

pub use self::begin::{Begin, Ended};
use self::{
    attributes::{
        Attribute, AttributeName, AttributeNameSawWhite, AttributeValue, AttributeValueClosed,
        AttributeValueQuoted, AttributeValueUnquoted,
    },
    begin::BeginWhitespace,
    declarations::{
        CData, CDataEnded, CDataEnding, Comment, CommentEnded, CommentEnding, Doctype, DoctypeDTD,
        DoctypeDTDQuoted, DoctypeQuoted, SGMLDeclaration, SGMLDeclarationQuoted,
    },
    entities::{AttributeValueEntityQuoted, AttributeValueEntityUnquoted, TextEntity},
    node_start::NodeStart,
    processing_instructions::{
        ProcessingInstruction, ProcessingInstructionBody, ProcessingInstructionEnding,
    },
    tags::{CloseTag, CloseTagSawWhite, OpenTag, OpenTagSlash},
    text::{Script, ScriptEnding, Text},
};

pub trait FileReaderState {
    fn next(self: Box<Self>, sax: &mut SAXState, char: &char) -> Box<dyn FileReaderState>;

    fn id(&self) -> State;

    fn token_id(&self) -> StateToken {
        match self.id() {
            State::Begin => StateToken::Begin,
            State::BeginWhitespace => StateToken::Begin,
            State::Text => StateToken::Text,
            State::TextEntity => StateToken::TextEntity,
            State::NodeStart => StateToken::NodeStart,
            State::SGMLDeclaration => StateToken::SGMLDeclaration,
            State::SGMLDeclarationQuoted => StateToken::SGMLDeclaration,
            State::Doctype => StateToken::SGMLDeclaration,
            State::DoctypeQuoted => StateToken::SGMLDeclaration,
            State::DoctypeDTD => StateToken::SGMLDeclaration,
            State::DoctypeDTDQuoted => StateToken::SGMLDeclaration,
            State::Comment => StateToken::SGMLDeclaration,
            State::CommentEnding => StateToken::SGMLDeclaration,
            State::CommentEnded => StateToken::SGMLDeclaration,
            State::CData => StateToken::CData,
            State::CDataEnding => StateToken::CData,
            State::CDataEnded => StateToken::CData,
            State::ProcessingInstruction => StateToken::ProcessingInstruction,
            State::ProcessingInstructionBody => StateToken::ProcessingInstruction,
            State::ProcessingInstructionEnding => StateToken::ProcessingInstruction,
            State::OpenTag => StateToken::OpenTag,
            State::OpenTagSlash => StateToken::OpenTag,
            State::Attribute => StateToken::Attribute,
            State::AttributeName => StateToken::Attribute,
            State::AttributeNameSawWhite => StateToken::Attribute,
            State::AttributeValue => StateToken::Attribute,
            State::AttributeValueQuoted => StateToken::AttributeValue,
            State::AttributeValueClosed => StateToken::AttributeValue,
            State::AttributeValueUnquoted => StateToken::AttributeValue,
            State::AttributeValueEntityQuoted => StateToken::AttributeValue,
            State::AttributeValueEntityUnquoted => StateToken::AttributeValue,
            State::CloseTag => StateToken::CloseTag,
            State::CloseTagSawWhite => StateToken::CloseTag,
            State::Script => StateToken::Script,
            State::ScriptEnding => StateToken::Script,
            State::Ended => StateToken::Begin,
        }
    }
}

impl Default for Box<dyn FileReaderState> {
    fn default() -> Self {
        Box::new(Begin)
    }
}

impl PartialEq for Box<dyn FileReaderState> {
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id()
    }
}

impl Clone for Box<dyn FileReaderState> {
    fn clone(&self) -> Self {
        match self.id() {
            State::Begin => Box::new(Begin),
            State::BeginWhitespace => Box::new(BeginWhitespace),
            State::Text => Box::new(Text),
            State::TextEntity => Box::new(TextEntity),
            State::NodeStart => Box::new(NodeStart),
            State::SGMLDeclaration => Box::new(SGMLDeclaration),
            State::SGMLDeclarationQuoted => Box::new(SGMLDeclarationQuoted),
            State::Doctype => Box::new(Doctype),
            State::DoctypeQuoted => Box::new(DoctypeQuoted),
            State::DoctypeDTD => Box::new(DoctypeDTD),
            State::DoctypeDTDQuoted => Box::new(DoctypeDTDQuoted),
            State::Comment => Box::new(Comment),
            State::CommentEnding => Box::new(CommentEnding),
            State::CommentEnded => Box::new(CommentEnded),
            State::CData => Box::new(CData),
            State::CDataEnding => Box::new(CDataEnding),
            State::CDataEnded => Box::new(CDataEnded),
            State::ProcessingInstruction => Box::new(ProcessingInstruction),
            State::ProcessingInstructionBody => Box::new(ProcessingInstructionBody),
            State::ProcessingInstructionEnding => Box::new(ProcessingInstructionEnding),
            State::OpenTag => Box::new(OpenTag),
            State::OpenTagSlash => Box::new(OpenTagSlash),
            State::Attribute => Box::new(Attribute),
            State::AttributeName => Box::new(AttributeName),
            State::AttributeNameSawWhite => Box::new(AttributeNameSawWhite),
            State::AttributeValue => Box::new(AttributeValue),
            State::AttributeValueQuoted => Box::new(AttributeValueQuoted),
            State::AttributeValueClosed => Box::new(AttributeValueClosed),
            State::AttributeValueUnquoted => Box::new(AttributeValueUnquoted),
            State::AttributeValueEntityQuoted => Box::new(AttributeValueEntityQuoted),
            State::AttributeValueEntityUnquoted => Box::new(AttributeValueEntityUnquoted),
            State::CloseTag => Box::new(CloseTag),
            State::CloseTagSawWhite => Box::new(CloseTagSawWhite),
            State::Script => Box::new(Script),
            State::ScriptEnding => Box::new(ScriptEnding),
            State::Ended => Box::new(Ended),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum State {
    /// Leading byte order mark or whitespace
    #[default]
    Begin,
    /// Leading whitespace
    BeginWhitespace,
    /// General stuff
    Text,
    /// &amp; and such
    TextEntity,
    /// `<`
    NodeStart,
    /// <!BLARG
    SGMLDeclaration,
    /// <!BLARG foo "bar"
    SGMLDeclarationQuoted,
    /// <!DOCTYPE
    Doctype,
    /// <!DOCTYPE "foo
    DoctypeQuoted,
    /// <!DOCTYPE "foo" [ ...
    DoctypeDTD,
    /// <!DOCTYPE "foo" [ "bar
    DoctypeDTDQuoted,
    /// <!--
    Comment,
    /// <!-- foo -
    CommentEnding,
    /// <!-- foo --
    CommentEnded,
    /// <![CDATA[ foo
    CData,
    /// ]
    CDataEnding,
    /// ]]
    CDataEnded,
    /// <?foo
    ProcessingInstruction,
    /// <?foo bar
    ProcessingInstructionBody,
    /// <?foo bar ?
    ProcessingInstructionEnding,
    /// <foo
    OpenTag,
    /// <foo /
    OpenTagSlash,
    /// <foo \s
    Attribute,
    /// <foo bar
    AttributeName,
    /// <foo bar\s
    AttributeNameSawWhite,
    /// <foo bar=
    AttributeValue,
    /// <foo bar="baz
    AttributeValueQuoted,
    /// <foo bar="baz"
    AttributeValueClosed,
    /// <foo bar=baz
    AttributeValueUnquoted,
    /// <foo bar="&quot;"
    AttributeValueEntityQuoted,
    /// <foo bar=&quot
    AttributeValueEntityUnquoted,
    /// </foo
    CloseTag,
    /// </foo >
    CloseTagSawWhite,
    /// <script>/* ... */
    Script,
    /// <script>/* ... */<
    ScriptEnding,
    /// EOF
    Ended,
}

#[derive(Clone, Debug, PartialEq)]
pub enum StateToken {
    /// Leading byte order mark or whitespace
    Begin,
    /// General stuff
    Text,
    /// &amp; and such
    TextEntity,
    /// `<`
    NodeStart,
    /// <!BLARG foo "bar" >
    SGMLDeclaration,
    /// <![CDATA[ foo ]]>
    CData,
    /// <?foo bar ?>
    ProcessingInstruction,
    /// <foo>
    OpenTag,
    /// foo
    Attribute,
    /// "bar"
    AttributeValue,
    /// </foo>
    CloseTag,
    /// <script>/* ... */
    Script,
}
