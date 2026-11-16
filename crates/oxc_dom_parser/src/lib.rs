//! oxc-dom-parser is a [DOMParser](https://developer.mozilla.org/en-US/docs/Web/API/DOMParser/parseFromString) parser that produces a comment-preserving,
//! span-faithful typed AST, designed for building formatters.
//!
//! ## Basic Usage
//!
//! ```rust
//! use oxc_dom_parser::{Allocator, Parser, MimeType};
//!
//! let allocator = Allocator::default();
//! let parser = Parser::new(&allocator, "<svg></svg>", MimeType::ImageSVGPlusXML);
//! match parser.parse() {
//!     Ok(root) => {
//!         assert_eq!(root.children.len(), 1);
//!     }
//!     Err(error) => {
//!         // Syntax error with span; no partial AST is produced.
//!         println!("{error}");
//!     }
//! }
//! ```

#![cfg_attr(feature = "nightly", feature(portable_simd))]

pub mod ast;
mod dtd_scanner;
mod entity;
mod error;
mod namespace;
pub mod options;
mod parser;
mod parser_doctype;
#[cfg(feature = "html")]
mod parser_html;
mod parser_xml;
mod scanner;
mod simd;
mod span;

pub use ast::MimeType;
pub use error::{Error, ErrorKind};
pub use oxc_allocator::Allocator;
pub use parser::Parser;
pub use span::Span;

/// enum regression guards, in the spirit of oxc_ast's generated assertions:
#[test]
fn size_asserts() {
    // Hot tokens stay small.
    // 1. Global tokens should be tiny <12 bytes.
    assert_eq!(size_of::<scanner::Token>(), 12);
    assert_eq!(size_of::<scanner::TokenKind>(), 1);
    assert_eq!(size_of::<span::Span>(), 8);
    // 2. Literals should stay small, <24 bytes.
    assert_eq!(size_of::<ast::Name>(), 24);
    assert_eq!(size_of::<ast::TextSpan>(), 24);
    assert_eq!(size_of::<ast::EscapedStr>(), 32);
    // 3. Element primitives should stay moderate, <80 bytes.
    assert_eq!(size_of::<ast::Element>(), 80);
    assert_eq!(size_of::<ast::Attribute>(), 72);
    assert_eq!(size_of::<ast::AttributeValue>(), 40);
    assert_eq!(size_of::<ast::Text>(), 40);

    // structured enums stay pointer-sized-plus-tag.
    assert_eq!(size_of::<ast::Node>(), 16);
    assert_eq!(size_of::<ast::DTDItem>(), 16);
    assert_eq!(size_of::<ast::EntityDef>(), 16);
}
