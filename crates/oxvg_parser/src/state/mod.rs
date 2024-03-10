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
    begin::LeadingWhitespace,
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

/// Represents the transitioned-to state for a processed character
pub trait State {
    /// Transitions from the current state to the next state based on the given character
    fn next(self: Box<Self>, sax: &mut SAXState, char: char) -> Box<dyn State>;

    /// Returns an enumerable ID of the current state
    fn id(&self) -> ID;

    /// Returns an enumerable ID of the token the state is a part of
    fn token_id(&self) -> Token {
        match self.id() {
            ID::Begin | ID::BeginWhitespace | ID::Ended => Token::Begin,
            ID::Text => Token::Text,
            ID::TextEntity => Token::TextEntity,
            ID::NodeStart => Token::NodeStart,
            ID::SGMLDeclaration
            | ID::SGMLDeclarationQuoted
            | ID::Doctype
            | ID::DoctypeQuoted
            | ID::DoctypeDTD
            | ID::DoctypeDTDQuoted
            | ID::Comment
            | ID::CommentEnding
            | ID::CommentEnded => Token::SGMLDeclaration,
            ID::CData | ID::CDataEnding | ID::CDataEnded => Token::CData,
            ID::ProcessingInstruction
            | ID::ProcessingInstructionBody
            | ID::ProcessingInstructionEnding => Token::ProcessingInstruction,
            ID::OpenTag | ID::OpenTagSlash => Token::OpenTag,
            ID::Attribute | ID::AttributeName | ID::AttributeNameSawWhite | ID::AttributeValue => {
                Token::Attribute
            }
            ID::AttributeValueQuoted
            | ID::AttributeValueClosed
            | ID::AttributeValueUnquoted
            | ID::AttributeValueEntityQuoted
            | ID::AttributeValueEntityUnquoted => Token::AttributeValue,
            ID::CloseTag | ID::CloseTagSawWhite => Token::CloseTag,
            ID::Script | ID::ScriptEnding => Token::Script,
        }
    }
}

impl Default for Box<dyn State> {
    fn default() -> Self {
        Box::new(Begin)
    }
}

impl PartialEq for Box<dyn State> {
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id()
    }
}

impl Clone for Box<dyn State> {
    fn clone(&self) -> Self {
        match self.id() {
            ID::Begin => Box::new(Begin),
            ID::BeginWhitespace => Box::new(LeadingWhitespace),
            ID::Text => Box::new(Text),
            ID::TextEntity => Box::new(TextEntity),
            ID::NodeStart => Box::new(NodeStart),
            ID::SGMLDeclaration => Box::new(SGMLDeclaration),
            ID::SGMLDeclarationQuoted => Box::new(SGMLDeclarationQuoted),
            ID::Doctype => Box::new(Doctype),
            ID::DoctypeQuoted => Box::new(DoctypeQuoted),
            ID::DoctypeDTD => Box::new(DoctypeDTD),
            ID::DoctypeDTDQuoted => Box::new(DoctypeDTDQuoted),
            ID::Comment => Box::new(Comment),
            ID::CommentEnding => Box::new(CommentEnding),
            ID::CommentEnded => Box::new(CommentEnded),
            ID::CData => Box::new(CData),
            ID::CDataEnding => Box::new(CDataEnding),
            ID::CDataEnded => Box::new(CDataEnded),
            ID::ProcessingInstruction => Box::new(ProcessingInstruction),
            ID::ProcessingInstructionBody => Box::new(ProcessingInstructionBody),
            ID::ProcessingInstructionEnding => Box::new(ProcessingInstructionEnding),
            ID::OpenTag => Box::new(OpenTag),
            ID::OpenTagSlash => Box::new(OpenTagSlash),
            ID::Attribute => Box::new(Attribute),
            ID::AttributeName => Box::new(AttributeName),
            ID::AttributeNameSawWhite => Box::new(AttributeNameSawWhite),
            ID::AttributeValue => Box::new(AttributeValue),
            ID::AttributeValueQuoted => Box::new(AttributeValueQuoted),
            ID::AttributeValueClosed => Box::new(AttributeValueClosed),
            ID::AttributeValueUnquoted => Box::new(AttributeValueUnquoted),
            ID::AttributeValueEntityQuoted => Box::new(AttributeValueEntityQuoted),
            ID::AttributeValueEntityUnquoted => Box::new(AttributeValueEntityUnquoted),
            ID::CloseTag => Box::new(CloseTag),
            ID::CloseTagSawWhite => Box::new(CloseTagSawWhite),
            ID::Script => Box::new(Script),
            ID::ScriptEnding => Box::new(ScriptEnding),
            ID::Ended => Box::new(Ended),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum ID {
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
pub enum Token {
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

#[test]
fn state() {
    let sax = &mut SAXState::default();
    let start: Box<dyn State> = Box::new(Begin);
    let next: Box<dyn State> = start.next(sax, 'a');

    assert_eq!(next.id(), LeadingWhitespace.id());
}
