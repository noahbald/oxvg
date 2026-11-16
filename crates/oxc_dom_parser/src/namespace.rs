use oxc_str::Str;

use crate::{Error, ErrorKind, ast::Element, error::ParseResult};

pub const XML_NAMESPACE: &str = "http://www.w3.org/XML/1998/namespace";
pub const XMLNS_NAMESPACE: &str = "http://www.w3.org/2000/xmlns/";

/// In-scope namespace bindings.
///
/// `bindings` holds `(prefix, uri)` pairs in declaration order; the default
/// namespace is stored with prefix `""`. `default` is `index + 1` of the
/// innermost default binding (0 = none), so unprefixed lookups are O(1).
#[derive(Default)]
pub(crate) struct Scope<'a> {
    bindings: std::vec::Vec<(&'a str, &'a str)>,
    default: u32,
}

/// Snapshot taken when an element opens, restored when it closes.
#[derive(Clone, Copy)]
pub(crate) struct ScopeMark {
    len: u32,
    default: u32,
}

impl<'a> Scope<'a> {
    #[inline]
    pub fn mark(&self) -> ScopeMark {
        ScopeMark {
            len: self.bindings.len() as u32,
            default: self.default,
        }
    }

    #[inline]
    pub fn restore(&mut self, mark: ScopeMark) {
        self.bindings.truncate(mark.len as usize);
        self.default = mark.default;
    }

    #[inline]
    fn default_uri(&self) -> &'a str {
        match self.default {
            0 => "",
            i => self.bindings[i as usize - 1].1,
        }
    }

    #[inline]
    fn lookup(&self, prefix: &str) -> Option<&'a str> {
        if prefix == "xml" {
            return Some(XML_NAMESPACE);
        }
        self.bindings
            .iter()
            .rev()
            .find(|(p, _)| *p == prefix)
            .map(|&(_, uri)| uri)
    }
}

/// Registers `element`'s declarations, then resolves its name and attribute names.
///
/// Call once the open tag is complete (`>` or `/>`), and `Scope::restore` when
/// the element closes.
pub(crate) fn resolve<'a>(element: &mut Element<'a>, scope: &mut Scope<'a>) -> ParseResult<()> {
    // Pass 1: declarations. They may follow the attributes that use them.
    for attr in element.attributes.iter_mut() {
        let name = attr.name.value.as_str();
        if !name.as_bytes().starts_with(b"xmlns") {
            continue;
        }
        let prefix = match attr.name.prefix() {
            Some("xmlns") => attr.name.local(),
            None if name == "xmlns" => "",
            _ => continue, // `xmlns-foo`, `xmlnsfoo:bar`: ordinary attributes
        };
        let span = attr.name.span;
        let uri = attr.value.value.as_str();

        if prefix == "xmlns" {
            return Err(Error::new(ErrorKind::ReservedNamespace, span));
        }
        if prefix == "xml" {
            if uri != XML_NAMESPACE {
                return Err(Error::new(ErrorKind::ReservedNamespace, span));
            }
        } else if uri == XML_NAMESPACE || uri == XMLNS_NAMESPACE {
            return Err(Error::new(ErrorKind::ReservedNamespace, span));
        }
        // Namespaces 1.0: only the default namespace may be undeclared.
        if !prefix.is_empty() && uri.is_empty() {
            return Err(Error::new(ErrorKind::EmptyNamespaceDeclaration, span));
        }

        if prefix != "xml" {
            scope.bindings.push((prefix, uri));
            if prefix.is_empty() {
                scope.default = scope.bindings.len() as u32;
            }
        }
        attr.name.uri = Str::from(XMLNS_NAMESPACE);
    }

    // Pass 2: names.
    let uri = match element.name.prefix() {
        Some(prefix) => scope
            .lookup(prefix)
            .ok_or_else(|| Error::new(ErrorKind::UnboundNamespacePrefix, element.name.span))?,
        None => scope.default_uri(),
    };
    element.name.uri = Str::from(uri);

    let mut has_namespaced_attr = false;
    for attr in element.attributes.iter_mut() {
        if !attr.name.uri.as_str().is_empty() {
            continue; // declaration, handled above
        }
        // Unprefixed attributes never take the default namespace.
        if let Some(prefix) = attr.name.prefix() {
            let uri = scope
                .lookup(prefix)
                .ok_or_else(|| Error::new(ErrorKind::UnboundNamespacePrefix, attr.name.span))?;
            attr.name.uri = Str::from(uri);
            has_namespaced_attr = true;
        }
    }

    // Namespaces 1.0 "Attributes Unique": `p:x` and `q:x` clash if p and q share a URI.
    // The raw-name duplicate check in the parser already caught identical prefixes.
    if has_namespaced_attr {
        let attrs = &element.attributes[..];
        for (i, a) in attrs.iter().enumerate() {
            if a.name.uri.as_str().is_empty() {
                continue;
            }
            if attrs[..i]
                .iter()
                .any(|b| b.name.uri == a.name.uri && b.name.local() == a.name.local())
            {
                return Err(Error::new(
                    ErrorKind::DisallowedDuplicateAttributes,
                    a.name.span,
                ));
            }
        }
    }
    Ok(())
}
