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
pub mod options;
mod parser;
mod scanner;
mod simd;
mod span;

pub use ast::MimeType;
pub use error::{Error, ErrorKind};
pub use oxc_allocator::Allocator;
pub use parser::Parser;
pub use span::Span;

/// Size regression guards, in the spirit of oxc_ast's generated assertions:
/// enums stay pointer-sized-plus-tag, and hot token types stay small.
#[cfg(all(test, target_pointer_width = "64"))]
mod size_asserts {
    use crate::{ast, scanner, span};

    #[test]
    fn sizes() {
        // Scanner-side: every buffered token pays these.
        assert_eq!(size_of::<scanner::Token>(), 12);
        assert_eq!(size_of::<scanner::TokenKind>(), 1);
        assert_eq!(size_of::<span::Span>(), 8);
        // `Node` adds the span and props on top of the content.
        // Node-position fields box it so container children stay small (guarded below).
        assert_eq!(size_of::<ast::Node>(), 16);
        assert_eq!(size_of::<ast::Element>(), 80);
        assert_eq!(size_of::<ast::TextSpan>(), 24);
        assert_eq!(size_of::<ast::Doctype>(), 24);
        assert_eq!(size_of::<ast::Comment>(), 32);
        assert_eq!(size_of::<ast::CData>(), 32);
        assert_eq!(size_of::<ast::ProcessingInstruction>(), 56);
    }
}
