/*!

A copy of the [xmlwriter](https://docs.rs/xmlwriter/latest/xmlwriter/index.html) create, except with some slight modifications.

- Errors instead of panicking
- Uses our `QualName` instead of storing a reference to `&str`
- Allows writing custom xml declarations

---

A simple, streaming, partially-validating XML writer that writes XML data to a
`std::io::Write` implementation.

### Features

- A simple, bare-minimum API that panics when writing invalid XML.
- Non-allocating API. All methods are accepting either `fmt::Display` or `fmt::Arguments`.
- Nodes auto-closing.

### Example

```rust
use oxvg_ast::xmlwriter::*;
use oxvg_collections::{
    element::ElementId,
    attribute::{
        Attr,
        uncategorised::ViewBox,
        presentation::LengthPercentage,
        list_of::{ListOf, SpaceOrComma},
    },
};
use std::io;

fn main() -> Result {
    let opt = Options {
        use_single_quote: true,
        ..Options::pretty()
    };

    let mut w = XmlWriter::new(Vec::<u8>::new(), opt);
    w.start_element(&ElementId::Svg)?;
    w.write_attribute(&Attr::XMLNS("http://www.w3.org/2000/svg".into()))?;
    // Write arbitrary attributes
    w.write_attribute_fmt(format_args!("viewBox"), format_args!("0 0 128 128"))?;
    w.start_element(&ElementId::Text)?;
    // Write concrete attributes
    w.write_attribute(&Attr::XText(ListOf {
        list: vec![LengthPercentage::px(10.0)],
        separator: SpaceOrComma,
    }))?;
    w.write_attribute(&Attr::YText(ListOf {
        list: vec![LengthPercentage::px(20.0)],
        separator: SpaceOrComma,
    }))?;
    w.write_text_fmt(format_args!("length is {}", 5))?;

    assert_eq!(std::str::from_utf8(w.end_document()?.as_slice())
        .expect("xmlwriter always writes valid UTF-8"),
"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 128 128'>
    <text x='10' y='20'>length is 5</text>
</svg>
"
    );
    Ok(())
}
```
*/

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(missing_copy_implementations)]

use std::fmt::{self, Write as FmtWrite};
use std::io::{self, Write};
use std::result;

use lightningcss::rules::CssRuleList;
use oxvg_collections::atom::Atom;
use oxvg_collections::attribute::xml::XmlSpace;
use oxvg_collections::attribute::Attr;
use oxvg_collections::element::ElementId;
use oxvg_serialize::{Printer, PrinterOptions, ToValue as _};

use crate::{error::XmlWriterError, is_element};

/// A result from serializing a document.
pub type Result = result::Result<(), XmlWriterError>;

/// Post-processing of whitespace characters inside elements.
#[derive(Clone, Copy, PartialEq, Debug)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
pub enum Space {
    /// Never modify whitespace characters from the original
    Never,
    /// Determine post-processing based on `xml:space` attributes
    Auto,
    /// Handle whitespace equivalent to `xml:space="default"`
    ///
    /// 1. Remove newline characters
    /// 2. Convert tab characters to spaces
    /// 3. Trim spaces
    /// 4. Collapse continuous spaces
    Default,
    /// Handle whitespace equivalent to `xml:space="preserve"`
    ///
    /// 1. Convert newline and tab characters to spaces
    Preserve,
}

/// An XML node indention.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Indent {
    /// Disable indention and new lines.
    None,
    /// Indent with spaces. Preferred range is 0..4.
    Spaces(u8),
    /// Indent with tabs.
    Tabs,
}
#[cfg(feature = "clap")]
impl clap::builder::ValueParserFactory for Indent {
    type Parser = IndentParser;

    fn value_parser() -> Self::Parser {
        IndentParser
    }
}
#[cfg(feature = "clap")]
#[derive(Clone, Copy)]
#[doc(hidden)]
pub struct IndentParser;
#[cfg(feature = "clap")]
impl clap::builder::TypedValueParser for IndentParser {
    type Value = Indent;

    fn parse_ref(
        &self,
        cmd: &clap::Command,
        arg: Option<&clap::Arg>,
        value: &std::ffi::OsStr,
    ) -> result::Result<Self::Value, clap::Error> {
        let spaces = clap::value_parser!(u8);
        if let Ok(spaces) = spaces.parse_ref(cmd, arg, value) {
            return Ok(Indent::Spaces(spaces));
        }
        let error_raw = "Valid pretty index values are `none`, `tabs`, or a number";
        match value.to_str() {
            Some("none") => Ok(Indent::None),
            Some("tabs") => Ok(Indent::Tabs),
            Some(_) => Err(clap::Error::raw(
                clap::error::ErrorKind::InvalidValue,
                error_raw,
            )),
            None => Err(clap::Error::raw(
                clap::error::ErrorKind::MissingRequiredArgument,
                error_raw,
            )),
        }
    }
}

/// An XML writing options.
#[derive(Clone, Copy, Debug)]
pub struct Options {
    /// Use single quote marks instead of double quote.
    ///
    /// # Examples
    ///
    /// Before:
    ///
    /// ```text
    /// <rect fill="red"/>
    /// ```
    ///
    /// After:
    ///
    /// ```text
    /// <rect fill='red'/>
    /// ```
    ///
    /// Default: disabled
    pub use_single_quote: bool,

    /// Set XML nodes indention.
    ///
    /// Setting an indentation other than [`Indent::None`] will cause the writer
    /// to ignore `xml:space` values.
    ///
    /// # Examples
    ///
    /// `Indent::None`
    /// Before:
    ///
    /// ```text
    /// <svg>
    ///     <rect fill="red"/>
    /// </svg>
    /// ```
    ///
    /// After:
    ///
    /// ```text
    /// <svg><rect fill="red"/></svg>
    /// ```
    ///
    /// Default: None
    pub indent: Indent,

    /// Set whether to trim whitespace around text.
    ///
    /// # Examples
    ///
    /// `TrimWhitespace::Auto`
    ///
    /// Before:
    ///
    /// ```text
    /// <svg>
    ///     <p> text </p>
    /// </svg>
    /// ```
    ///
    /// After:
    ///
    /// ```text
    /// <svg>
    ///     <p>text</p>
    /// </svg>
    ///
    /// Default: `Auto`
    /// ```
    pub trim_whitespace: Space,

    /// Set XML attributes indention.
    ///
    /// # Examples
    ///
    /// `Indent::Spaces(2)`
    ///
    /// Before:
    ///
    /// ```text
    /// <svg>
    ///     <rect fill="red" stroke="black"/>
    /// </svg>
    /// ```
    ///
    /// After:
    ///
    /// ```text
    /// <svg>
    ///     <rect
    ///       fill="red"
    ///       stroke="black"/>
    /// </svg>
    /// ```
    ///
    /// Default: `None`
    pub attributes_indent: Indent,

    /// Write self-closing tags when element is empty.
    ///
    /// # Examples
    ///
    /// Before:
    ///
    /// ```text
    /// <tag/>
    /// ```
    ///
    /// After:
    ///
    /// ```text
    /// <tag>
    /// </tag>
    /// ```
    ///
    /// Default: enabled
    pub enable_self_closing: bool,

    /// Whether to minify while printing attributes and style content
    ///
    /// # Examples
    ///
    /// Before:
    ///
    /// ```text
    /// <tag width="10px" />
    /// ```
    ///
    /// After:
    ///
    /// ```text
    /// <tag width="10" />
    /// ```
    ///
    /// Default: true
    pub minify: bool,
}

impl Default for Options {
    #[inline]
    fn default() -> Self {
        Options {
            use_single_quote: false,
            trim_whitespace: Space::Auto,
            indent: Indent::None,
            attributes_indent: Indent::None,
            enable_self_closing: true,
            minify: true,
        }
    }
}

impl Options {
    /// Returns a set of options that will add indentation for nested elements and
    /// add readable whitespace in attribute values.
    pub fn pretty() -> Self {
        Options {
            indent: Indent::Spaces(4),
            minify: false,
            ..Options::default()
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum State {
    Empty,
    Document,
    Attributes,
    CData,
}

#[derive(Clone, Debug)]
struct DepthData<'input> {
    element_name: Option<ElementId<'input>>,
    has_children: bool,
}

/// This wrapper writer is so that we can make sure formatted strings are properly escaped too,
/// as we don't have access to the formatting stuff without a `fmt::Write` implementation, so
/// we provide it by wrapping the writer given to us while escaping appropriately any string to
/// be written, depending on the type of node we're writing.
#[derive(Clone, Debug)]
pub struct FmtWriter<W: Write> {
    writer: W,
    error_kind: Option<io::ErrorKind>,
    // Set to None once the text is written, as a way to make sure the code
    // sets the proper escaping type before using the fmt_writer.write_str().
    escape: Option<Escape>,
    // Same as for Options, but kept available for write_escaped()
    use_single_quote: bool,
}

impl<W: Write> FmtWriter<W> {
    fn take_err(&mut self) -> XmlWriterError {
        let error_kind = self
            .error_kind
            .expect("there must have been an error before calling take_err()!");
        // This avoids forgetting to set it to the appropriate value when calling write_fmt().
        // We can't do it in FmtWriter's write_str(), since with a real format string the method
        // will be called several times so it'll fail in the expect() below as we'll have set
        // self.escape back to None.
        self.escape = None;
        // Make sure we can detect if take_err() is called without having an error that happened beforehand
        self.error_kind = None;

        // There's just no way of properly copying the io::Error (no Copy or Clone available), so
        // we have no choice to create a new one, which likely loses the backtrace up to this point.
        XmlWriterError::IO(io::Error::from(error_kind))
    }

    fn write_escaped(&mut self, s: &str, escape_quotes: bool) -> io::Result<()> {
        let mut part_start_pos = 0;
        for (byte_pos, byte) in s.bytes().enumerate() {
            let escaped_char: Option<&[u8]> = match byte {
                b'&' => Some(b"&amp;"),
                b'>' => Some(b"&gt;"),
                b'<' => Some(b"&lt;"),
                b'"' if escape_quotes && !self.use_single_quote => Some(b"&quot;"),
                b'\'' if escape_quotes && self.use_single_quote => Some(b"&apos;"),
                _ => None,
            };
            if let Some(escaped_char) = escaped_char {
                // We have a character to escape, so write the previous part and the escaped character
                self.writer
                    .write_all(&s.as_bytes()[part_start_pos..byte_pos])?;
                self.writer.write_all(escaped_char)?;
                // +1 skips the escaped character from part, for afterwards
                part_start_pos = byte_pos + 1;
            }
            // There's nothing to be done if the character doesn't need to be escaped, as we'll either
            // wait until we get an escapable character, or wait until the end of the string where we'll
            // just write out the rest of the string.
        }
        // Write the rest of the string which needs no escaping
        self.writer.write_all(&s.as_bytes()[part_start_pos..])
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Escape {
    Comment,
    AttributeValue,
    Text,
    CData,
    Style,
}

impl<W: Write> fmt::Write for FmtWriter<W> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let error = match self
            .escape
            .expect("You must have set self.escape to Some(…) before using the formatter!")
        {
            Escape::AttributeValue => self.write_escaped(s, true),
            Escape::Text => self.write_escaped(s, false),
            // We don't bother escaping double hyphen (--) in comment as it's
            // unlikely to ever happen, and even libxml2 does not do it.
            Escape::Comment | Escape::CData | Escape::Style => self.writer.write_all(s.as_bytes()),
        };
        if error.is_err() {
            self.error_kind = Some(error.as_ref().unwrap_err().kind());
            Err(fmt::Error)
        } else {
            Ok(())
        }
    }
}

/// An XML writer.
#[derive(Clone, Debug)]
pub struct XmlWriter<'input, W: Write> {
    // When you control what you're writing enough that you know the bytes are already escaped or
    // don't need escaping at all, then use fmt_writer.writer.write_all()?; directly. Otherwise,
    // set fmt_writer.escape to the appropriate escaping type and use fmt_writer.write_fmt()?; or
    // fmt_writer.write_str()?; if you are only printing a string directly without formatting, but
    // still want escaping to be done.
    fmt_writer: FmtWriter<W>,
    /// Which data type is being processed
    state: State,
    /// Whether `xml:space="preserve"` as been preserved
    preserve_whitespaces: bool,
    /// Tracks parent elements for writing closing tags
    depth_stack: Vec<DepthData<'input>>,
    /// Options provided by the user
    opt: Options,
}

impl<'input, W: Write> XmlWriter<'input, W> {
    /// Creates a new `XmlWriter`, writing data in the writer.
    #[inline]
    pub fn new(writer: W, opt: Options) -> Self {
        XmlWriter {
            fmt_writer: FmtWriter {
                writer,
                error_kind: None,
                escape: None,
                use_single_quote: opt.use_single_quote,
            },
            state: State::Empty,
            preserve_whitespaces: false,
            depth_stack: Vec::new(),
            opt,
        }
    }

    /// Writes an XML declaration.
    ///
    /// `<?xml version="1.0" encoding="UTF-8" standalone="no"?>`
    ///
    /// # Errors
    ///
    /// - When called twice.
    #[inline(never)]
    pub fn write_declaration(&mut self, target: &Atom, value: &Option<Atom>) -> Result {
        if self.state != State::Empty {
            return Err(XmlWriterError::DeclarationAlreadyWritten);
        }
        self.fmt_writer
            .writer
            .write_fmt(format_args!("<?{target}"))
            .map_err(XmlWriterError::IO)?;
        if let Some(value) = value {
            self.fmt_writer
                .writer
                .write_fmt(format_args!(" {value}"))
                .map_err(XmlWriterError::IO)?;
        }
        self.fmt_writer
            .writer
            .write_fmt(format_args!("?>"))
            .map_err(XmlWriterError::IO)?;
        self.state = State::Document;

        Ok(())
    }

    /// Writes a comment string.
    ///
    /// # Errors
    ///
    /// When comment is in a bad state or when io fails.
    pub fn write_comment(&mut self, text: &str) -> Result {
        self.write_comment_fmt(format_args!("{text}"))
    }

    /// Writes a formatted comment. Forbidden double hyphens will be escaped.
    ///
    /// # Errors
    ///
    /// When comment is in a bad state or when io fails.
    #[inline(never)]
    pub fn write_comment_fmt(&mut self, fmt: fmt::Arguments) -> Result {
        if self.state == State::Attributes {
            self.write_open_element()?;
        }

        if self.state != State::Empty {
            self.write_new_line()?;
        }

        self.write_node_indent()?;

        // <!--text-->
        self.fmt_writer
            .writer
            .write_all(b"<!--")
            .map_err(XmlWriterError::IO)?;
        self.fmt_writer.escape = Some(Escape::Comment);
        self.fmt_writer
            .write_fmt(fmt)
            .map_err(|_| self.fmt_writer.take_err())?;
        self.fmt_writer
            .writer
            .write_all(b"-->")
            .map_err(XmlWriterError::IO)?;

        if self.state == State::Attributes {
            self.depth_stack.push(DepthData {
                element_name: None,
                has_children: false,
            });
        }

        self.state = State::Document;

        Ok(())
    }

    /// Starts writing a new element.
    ///
    /// This method writes only the `<tag-name` part.
    ///
    /// # Errors
    ///
    /// When in a bad state or when io fails.
    #[inline(never)]
    pub fn start_element(&mut self, name: &ElementId<'input>) -> Result {
        if self.state == State::Attributes {
            self.write_open_element()?;
        }

        if self.state != State::Empty {
            self.write_new_line()?;
        }

        if !self.preserve_whitespaces && !self.is_text_content_element() {
            self.write_node_indent()?;
        }

        self.fmt_writer
            .writer
            .write_all(b"<")
            .map_err(XmlWriterError::IO)?;
        self.fmt_writer
            .writer
            .write_fmt(format_args!("{name}"))
            .map_err(XmlWriterError::IO)?;

        self.depth_stack.push(DepthData {
            element_name: Some(name.clone()),
            has_children: false,
        });

        self.state = State::Attributes;

        Ok(())
    }

    /// Writes an attribute.
    ///
    /// Any occurrence of `&<>"'` in the value will be escaped.
    ///
    /// # Errors
    ///
    /// - When called before `start_element()`.
    /// - When called after `close_element()`.
    ///
    /// # Example
    ///
    /// ```
    /// use oxvg_ast::{
    ///     xmlwriter::*,
    /// };
    /// use oxvg_collections::{
    ///     element::ElementId,
    ///     attribute::{Attr, presentation::LengthPercentage},
    /// };
    /// use std::io;
    ///
    /// fn main() -> Result {
    ///     let mut w = XmlWriter::new(Vec::<u8>::new(), Options::default());
    ///     w.start_element(&ElementId::Svg)?;
    ///     w.write_attribute(&Attr::XGeometry(LengthPercentage::px(5.0)))?;
    ///     w.write_attribute(&Attr::YGeometry(LengthPercentage::px(5.0)))?;
    ///     assert_eq!(std::str::from_utf8(w.end_document()?.as_slice())
    ///         .expect("xmlwriter should always produce valid UTF-8"),
    ///         "<svg x=\"5\" y=\"5\"/>",
    ///     );
    ///     Ok(())
    /// }
    /// ```
    pub fn write_attribute(&mut self, attr: &Attr<'_>) -> Result {
        let minify = self.opt.minify;
        if self.opt.trim_whitespace == Space::Auto {
            if let Attr::XmlSpace(space) = attr.unaliased() {
                self.preserve_whitespaces = matches!(space, XmlSpace::Preserve);
            }
        }
        match attr.prefix().value() {
            Some(prefix) => {
                self.write_attribute_raw(format_args!("{prefix}:{}", attr.local_name()), |w| {
                    attr.write_value(&mut Printer::new(
                        w,
                        PrinterOptions {
                            minify,
                            ..PrinterOptions::default()
                        },
                    ))
                    .map_err(XmlWriterError::PrinterError)
                })
            }
            None => self.write_attribute_raw(format_args!("{}", attr.local_name()), |w| {
                attr.write_value(&mut Printer::new(
                    w,
                    PrinterOptions {
                        minify,
                        ..PrinterOptions::default()
                    },
                ))
                .map_err(XmlWriterError::PrinterError)
            }),
        }
    }

    /// Writes a formatted attribute value.
    ///
    /// Any occurrence of `&<>"'` in the value will be escaped.
    ///
    /// # Errors
    ///
    /// - When called before `start_element()`.
    /// - When called after `close_element()`.
    ///
    /// # Example
    ///
    /// ```
    /// use oxvg_ast::{
    ///     xmlwriter::*,
    /// };
    /// use oxvg_collections::{
    ///     element::ElementId,
    ///     content_type::ContentType,
    /// };
    /// use std::io;
    ///
    /// fn main() -> Result {
    ///     let mut w = XmlWriter::new(Vec::<u8>::new(), Options::default());
    ///     w.start_element(&ElementId::Rect)?;
    ///     w.write_attribute_fmt(format_args!("fill"), format_args!("url(#gradient)"))?;
    ///     assert_eq!(std::str::from_utf8(w.end_document()?.as_slice())
    ///         .expect("xmlwriter should always produce valid UTF-8"),
    ///         "<rect fill=\"url(#gradient)\"/>"
    ///     );
    ///     Ok(())
    /// }
    /// ```
    #[inline(never)]
    pub fn write_attribute_fmt(&mut self, name: fmt::Arguments, value: fmt::Arguments) -> Result {
        if self.state != State::Attributes {
            return Err(XmlWriterError::AttributeWrittenBeforeElement);
        }

        self.write_attribute_prefix(name)
            .map_err(XmlWriterError::IO)?;
        self.fmt_writer.escape = Some(Escape::AttributeValue);
        self.fmt_writer
            .write_fmt(value)
            .map_err(XmlWriterError::FMT)?;
        self.write_quote().map_err(XmlWriterError::IO)
    }

    /// Writes a raw attribute value, without performing escaping.
    ///
    /// Closure provides a mutable reference to the writer.
    ///
    /// **Warning:** this method is an escape hatch for cases when you need to write
    /// a lot of data very fast, and as such does no validity checks whatsoever on the
    /// written value.
    ///
    /// # Errors
    ///
    /// - When called before `start_element()`.
    /// - When called after `close_element()`.
    ///
    /// # Example
    ///
    /// ```
    /// use oxvg_ast::{
    ///     xmlwriter::*,
    ///     error::XmlWriterError,
    /// };
    /// use oxvg_collections::element::ElementId;
    /// use std::fmt::Write;
    ///
    /// fn main() -> Result {
    ///     let mut w = XmlWriter::new(Vec::<u8>::new(), Options::default());
    ///     w.start_element(&ElementId::Path);
    ///     w.write_attribute_raw(format_args!("d"), |writer| writer.write_str("M 10 20 L 30 40").map_err(XmlWriterError::FMT));
    ///     assert_eq!(std::str::from_utf8(w.end_document()?.as_slice())
    ///         .expect("xmlwriter should always produce valid UTF-8"),
    ///         "<path d=\"M 10 20 L 30 40\"/>"
    ///     );
    ///     Ok(())
    /// }
    /// ```
    #[inline(never)]
    pub fn write_attribute_raw<F>(&mut self, name: fmt::Arguments, f: F) -> Result
    where
        F: for<'a> FnOnce(&mut FmtWriter<W>) -> Result,
    {
        if self.state != State::Attributes {
            return Err(XmlWriterError::AttributeWrittenBeforeElement);
        }

        self.write_attribute_prefix(name)
            .map_err(XmlWriterError::IO)?;
        self.fmt_writer.escape = Some(Escape::AttributeValue);
        f(&mut self.fmt_writer)?;
        self.write_quote().map_err(XmlWriterError::IO)
    }

    #[inline(never)]
    fn write_attribute_prefix(&mut self, name: fmt::Arguments) -> io::Result<()> {
        if self.opt.attributes_indent == Indent::None {
            self.fmt_writer.writer.write_all(b" ")?;
        } else {
            self.fmt_writer.writer.write_all(b"\n")?;

            let depth = self.depth_stack.len();
            if depth > 0 {
                self.write_indent(depth - 1, self.opt.indent)?;
            }

            self.write_indent(1, self.opt.attributes_indent)?;
        }

        self.fmt_writer.writer.write_fmt(name)?;
        self.fmt_writer.writer.write_all(b"=")?;
        self.write_quote()
    }

    /// Writes a text node.
    ///
    /// See [`write_text_fmt()`] for details.
    ///
    /// [`write_text_fmt()`]: struct.XmlWriter.html#method.write_text_fmt
    ///
    /// # Errors
    ///
    /// - When called not after `start_element()`.
    pub fn write_text(&mut self, text: &str, is_first: bool, is_last: bool) -> Result {
        let space = match self.opt.trim_whitespace {
            Space::Auto => {
                if self.preserve_whitespaces {
                    Space::Preserve
                } else {
                    Space::Default
                }
            }
            Space::Never => return self.write_text_fmt(format_args!("{text}")),
            space => space,
        };

        match space {
            Space::Default => {
                if text.trim_start().is_empty() {
                    return Ok(());
                }
                self.before_write_text(false)?;
                let mut parts = text.split_whitespace();
                if let Some(first) = parts.next() {
                    let mut chars = text.chars();
                    if !is_first
                        && self.opt.indent == Indent::None
                        && chars.next().is_some_and(char::is_whitespace)
                    {
                        self.fmt_writer
                            .write_char(' ')
                            .map_err(|_| self.fmt_writer.take_err())?;
                    }

                    self.fmt_writer
                        .write_str(first)
                        .map_err(|_| self.fmt_writer.take_err())?;
                    for part in parts {
                        self.fmt_writer
                            .write_char(' ')
                            .map_err(|_| self.fmt_writer.take_err())?;
                        self.fmt_writer
                            .write_str(part)
                            .map_err(|_| self.fmt_writer.take_err())?;
                    }

                    if !is_last
                        && self.opt.indent == Indent::None
                        && text.chars().next_back().is_some_and(char::is_whitespace)
                    {
                        self.fmt_writer
                            .write_char(' ')
                            .map_err(|_| self.fmt_writer.take_err())?;
                    }
                }
            }
            Space::Preserve => {
                let mut old_indent = Indent::None;
                std::mem::swap(&mut self.opt.indent, &mut old_indent);
                self.before_write_text(false)?;
                std::mem::swap(&mut self.opt.indent, &mut old_indent);

                for char in text.chars() {
                    if matches!(char, '\n' | '\t') {
                        self.fmt_writer
                            .write_char(' ')
                            .map_err(|_| self.fmt_writer.take_err())?;
                    } else {
                        self.fmt_writer
                            .write_char(char)
                            .map_err(|_| self.fmt_writer.take_err())?;
                    }
                }
            }
            _ => unreachable!("Space should have been resolved to another option"),
        }
        self.state = State::Document;
        Ok(())
    }

    /// Writes a text node.
    ///
    /// # Errors
    ///
    /// - When called not after `start_element()`.
    pub fn write_style(&mut self, style: &CssRuleList) -> Result {
        if style.0.is_empty() {
            return Ok(());
        }
        self.before_write_text(false)?;
        self.fmt_writer.escape = Some(Escape::Style);
        style
            .write_value(&mut Printer::new(
                &mut self.fmt_writer,
                PrinterOptions {
                    minify: self.opt.minify,
                    ..PrinterOptions::default()
                },
            ))
            .map_err(XmlWriterError::PrinterError)
    }

    /// Writes a formatted text node.
    ///
    /// `><&` will be escaped.
    ///
    /// # Errors
    ///
    /// - When called not after `start_element()`.
    pub fn write_text_fmt(&mut self, fmt: fmt::Arguments) -> Result {
        self.write_text_fmt_impl(fmt, false)
    }

    /// Writes text inside a `<![CDATA[ ... ]]>` node.
    ///
    /// # Errors
    ///
    /// - When called not after `start_element()`.
    /// - When the text contains the literal `]]>`.
    pub fn write_cdata_text(&mut self, text: &str) -> Result {
        if text.contains("]]>") {
            return Err(XmlWriterError::BadCDATA);
        }
        self.write_text_fmt_impl(format_args!("{text}"), true)
    }

    fn before_write_text(&mut self, cdata: bool) -> Result {
        if self.state == State::Empty || self.depth_stack.is_empty() {
            return Err(XmlWriterError::TextBeforeElement);
        }

        if self.state == State::Attributes {
            self.write_open_element()?;
        }

        if cdata && self.state != State::CData {
            self.fmt_writer
                .writer
                .write_all(b"<![CDATA[")
                .map_err(XmlWriterError::IO)?;
        }

        if self.state != State::Empty {
            self.write_new_line()?;
        }

        self.write_node_indent()?;

        self.fmt_writer.escape = Some(if cdata { Escape::CData } else { Escape::Text });
        Ok(())
    }

    #[inline(never)]
    fn write_text_fmt_impl(&mut self, fmt: fmt::Arguments, cdata: bool) -> Result {
        self.before_write_text(cdata)?;
        self.fmt_writer
            .write_fmt(fmt)
            .map_err(|_| self.fmt_writer.take_err())?;

        if self.state == State::Attributes {
            self.depth_stack.push(DepthData {
                element_name: None,
                has_children: false,
            });
        }

        self.state = if cdata { State::CData } else { State::Document };

        Ok(())
    }

    /// Closes an open element.
    ///
    /// # Errors
    ///
    /// When in a bad state or when io fails.
    #[inline(never)]
    pub fn end_element(&mut self) -> Result {
        if let Some(depth) = self.depth_stack.pop() {
            if depth.has_children || !self.opt.enable_self_closing {
                // Close the empty node here as there were no children to close it.
                if !depth.has_children && !self.opt.enable_self_closing {
                    self.fmt_writer
                        .writer
                        .write_all(b">")
                        .map_err(XmlWriterError::IO)?;
                }

                if !self.preserve_whitespaces && !is_text_content_element(&depth) {
                    self.write_new_line()?;
                    self.write_node_indent()?;
                }

                if self.state == State::CData {
                    self.fmt_writer
                        .writer
                        .write_all(b"]]>")
                        .map_err(XmlWriterError::IO)?;
                }

                self.fmt_writer
                    .writer
                    .write_all(b"</")
                    .map_err(XmlWriterError::IO)?;

                // Write the previous opening element name as closing element now.
                let Some(element_name) = depth.element_name else {
                    return Err(XmlWriterError::ClosedUnopenedElement);
                };
                self.fmt_writer
                    .writer
                    .write_fmt(format_args!("{element_name}"))
                    .map_err(XmlWriterError::IO)?;

                self.fmt_writer
                    .writer
                    .write_all(b">")
                    .map_err(XmlWriterError::IO)?;
            } else {
                self.fmt_writer
                    .writer
                    .write_all(b"/>")
                    .map_err(XmlWriterError::IO)?;
            }
        }

        self.state = State::Document;

        Ok(())
    }

    /// Closes all open elements and returns back the writer.
    ///
    /// # Example
    ///
    /// ```
    /// use oxvg_ast::xmlwriter::*;
    /// use oxvg_collections::element::ElementId;
    /// use std::io;
    ///
    /// fn main() -> Result {
    ///     let mut w = XmlWriter::new(Vec::<u8>::new(), Options::pretty());
    ///     w.start_element(&ElementId::Svg)?;
    ///     w.start_element(&ElementId::G)?;
    ///     w.start_element(&ElementId::Rect)?;
    ///     assert_eq!(std::str::from_utf8(w.end_document()?.as_slice())
    ///         .expect("xmlwriter should always produce valid UTF-8"),
    /// "<svg>
    ///     <g>
    ///         <rect/>
    ///     </g>
    /// </svg>
    /// "
    ///     );
    ///     Ok(())
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// When in a bad state or when io fails.
    pub fn end_document(mut self) -> result::Result<W, XmlWriterError> {
        while !self.depth_stack.is_empty() {
            self.end_element()?;
        }

        self.write_new_line()?;

        Ok(self.fmt_writer.writer)
    }

    #[inline]
    fn get_quote_char(&self) -> u8 {
        if self.opt.use_single_quote {
            b'\''
        } else {
            b'"'
        }
    }

    // Writes quote unescaped, so only use when appropriate.
    #[inline]
    fn write_quote(&mut self) -> io::Result<()> {
        self.fmt_writer.writer.write_all(&[self.get_quote_char()])
    }

    // Writes the end of the current opening element, so `>`.
    fn write_open_element(&mut self) -> Result {
        if let Some(depth) = self.depth_stack.last_mut() {
            depth.has_children = true;
            self.fmt_writer
                .writer
                .write_all(b">")
                .map_err(XmlWriterError::IO)?;

            self.state = State::Document;
        }
        Ok(())
    }

    fn write_node_indent(&mut self) -> Result {
        self.write_indent(self.depth_stack.len(), self.opt.indent)
            .map_err(XmlWriterError::IO)
    }

    fn write_indent(&mut self, depth: usize, indent: Indent) -> io::Result<()> {
        if indent == Indent::None || self.preserve_whitespaces || self.is_text_content_element() {
            return Ok(());
        }

        for _ in 0..depth {
            match indent {
                Indent::None => {}
                Indent::Spaces(n) => {
                    for _ in 0..n {
                        self.fmt_writer.writer.write_all(b" ")?;
                    }
                }
                Indent::Tabs => self.fmt_writer.writer.write_all(b"\t")?,
            }
        }
        Ok(())
    }

    fn write_new_line(&mut self) -> Result {
        if self.opt.indent != Indent::None
            && !self.preserve_whitespaces
            && !self.is_text_content_element()
        {
            self.fmt_writer
                .writer
                .write_all(b"\n")
                .map_err(XmlWriterError::IO)?;
        }
        Ok(())
    }

    fn is_text_content_element(&self) -> bool {
        self.depth_stack.iter().rev().any(is_text_content_element)
    }
}

fn is_text_content_element(data: &DepthData) -> bool {
    data.element_name.as_ref().is_some_and(|name| {
        is_element!(
            name,
            A | Text | TextPath | TSpan | AltGlyph | AltGlyphDef | Glyph | GlyphRef | TRef
        )
    })
}
