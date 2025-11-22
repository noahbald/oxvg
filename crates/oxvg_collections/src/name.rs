//! Types for names of elements and attributes
use std::fmt::Display;

use crate::atom::Atom;

#[macro_export]
/// Returns whether the prefix matches the given names
macro_rules! is_prefix {
    ($element:expr, $($name:ident)|+$(,)?) => {
        matches!($element.prefix().unaliased(), $($crate::name::Prefix::$name)|+)
    };
}

macro_rules! define_prefix {
    ($($prefix:ident {
        $(name: $name:literal,)?
        uri: $uri:tt,
    },)+) => {
        macro_rules! name_else {
            ($_name:expr) => { Some($_name) };
            () => { None };
        }
        #[cfg(not(feature = "markup5ever"))]
        #[allow(non_upper_case_globals)]
        mod _uri {
            use crate::atom::Atom;
            $(pub const $prefix: &'static Atom<'static> = &Atom::Static($uri);)+
        }
        #[cfg(feature = "markup5ever")]
        #[allow(non_upper_case_globals)]
        pub(crate) mod _uri {
            use crate::atom::Atom;
            $(pub const $prefix: &'static Atom<'static> = &Atom::NS(xml5ever::namespace_url!($uri));)+
        }
        #[allow(non_upper_case_globals)]
        mod _ns {
            use super::NS;
            $(pub const $prefix: &'static NS<'static> = &NS::$prefix;)+
        }
        #[allow(non_upper_case_globals)]
        mod _name {
            use crate::atom::Atom;
            $(pub const $prefix: Option<Atom<'static>> = name_else!($(Atom::Static($name))?);)+
        }

        #[derive(Debug, Clone)]
        /// A prefix for a qualified name, e.g. `xlink` of `xlink:href`
        pub enum Prefix<'input> {
            $(
                #[doc=concat!("The standard prefix"$(, " for `", $name, "`")?, " in SVG")]
                $prefix,
            )+
            /// A not well-known prefix, e.g. `sodipodi:nodetypes`
            Unknown {
                /// The prefix name, e.g. `prefix` of `xmlns:prefix="<url>"`
                prefix: Option<Atom<'input>>,
                /// The unique resource identifier, e.g. `<url>` of `xmlns:prefix="<url>"`
                ns: NS<'input>,
            },
            /// A prefix with a non-usual, e.g. `<alias:svg></alias:svg>` when `xmlns:alias="http://www.w3.org/2000/svg"`
            Aliased {
                /// The prefix that's being aliased
                prefix: Box<Prefix<'input>>,
                /// The name assigned to the prefix
                alias: Option<Atom<'input>>,
            },
        }

        #[derive(Debug, Clone, Hash)]
        /// A namespace for a qualified name's prefix
        pub enum NS<'input> {
            $(
                #[doc=concat!("The standard uri"$(, " for `", $name, "`")?, " in SVG")]
                $prefix,
            )+
            /// A not well-known namespace
            Unknown(Atom<'input>),
        }

        impl<'input> Prefix<'input> {
            /// Takes a namespace and prefix and returns the associated variant
            pub fn new(ns: Atom<'input>, prefix: Option<Atom<'input>>) -> Self {
                match (&*ns, prefix.as_deref()) {
                    $(($uri, name_else!($($name)?)) => Self::$prefix,)+
                    $(($uri, _) => Self::Aliased {
                        prefix: Box::new(Self::$prefix),
                        alias: prefix,
                    },)+
                    ("", None) => Self::SVG,
                    (..) => Self::Unknown {
                        prefix,
                        ns: NS::new(ns),
                    },
                }
            }

            // For use in macros interoperable with `Prefix`
            #[doc(hidden)]
            pub fn prefix<'a>(&'a self) -> &'a Prefix<'input> {
                self
            }

            /// Returns the alias of the prefix
            pub fn value(&self) -> Option<Atom<'input>> {
                match self {
                    $(Self::$prefix => _name::$prefix,)+
                    Self::Unknown { prefix, .. } => prefix.clone(),
                    Self::Aliased { alias, .. } => alias.clone(),
                }
            }

            /// Returns the URI of the prefix
            pub fn ns(&self) -> &NS<'input> {
                match self {
                    $(Self::$prefix => _ns::$prefix,)+
                    Self::Unknown { ns, .. } => ns,
                    Self::Aliased { prefix, .. } => prefix.ns(),
                }
            }

            /// Compares whether two prefix values belong to the same namespace
            pub fn is_ns(&self, ns: &NS<'input>) -> bool {
                match (self, ns) {
                    $((Self::$prefix, NS::$prefix) => true,)+
                    (Self::Unknown { ns, .. }, NS::Unknown( uri )) => ns.uri() == uri,
                    (Self::Aliased { prefix, .. }, ..) => prefix.is_ns(ns),
                    _ => false,
                }
            }

            /// Returns whether the prefix has a name.
            /// E.g. using `xmlns="<url>"` would produce an empty prefix like `foo="bar"`,
            /// whereas using `xmlns:alias="<url>"` would produce a non-empty prefix
            /// like `alias:foo="bar"`.
            pub fn is_empty(&self) -> bool {
                self.value().is_none()
            }

            /// Returns whether the prefix deviates from the standard prefix for a given namespace.
            pub fn is_aliased(&self) -> bool {
                match self {
                    Self::Aliased { .. } => true,
                    _ => false
                }
            }

            /// Returns a `Prefix` that may be aliased as `Prefix::Aliased` as the inner prefix
            /// that's aliased.
            pub fn unaliased(&self) -> &Self {
                match self {
                    Self::Aliased { prefix, .. } => {
                        let result = prefix.as_ref();
                        debug_assert!(!matches!(result, Self::Aliased { .. }));
                        result
                    },
                    _ => self,
                }
            }
        }

        impl<'input> NS<'input> {
            /// Returns the associated namespace variant based on the uri
            pub fn new(uri: Atom<'input>) -> Self {
                match &*uri {
                    $($uri => Self::$prefix,)+
                    _ => Self::Unknown(uri),
                }
            }

            /// Returns the uri of this namespace
            pub fn uri<'a>(&'a self) -> &'a Atom<'input> {
                match self {
                    $(Self::$prefix => _uri::$prefix,)+
                    Self::Unknown(uri) => uri,
                }
            }
        }

        impl PartialEq for NS<'_> {
            fn eq(&self, other: &Self) -> bool {
                match (self, other) {
                    $((Self::$prefix, Self::$prefix) => true,)+
                    (Self::Unknown(uri), Self::Unknown(other_uri)) => uri == other_uri,
                    _ => false,
                }
            }
        }
    };
}

/// A qualified name used for the names of tags and attributes.
#[derive(Eq, PartialEq, Debug, Clone, Hash)]
pub struct QualName<'input> {
    /// The prefix (e.g. `xlink` of `xlink:href`) of a qualified name.
    pub prefix: Prefix<'input>,
    /// The local name (e.g. the `href` of `xlink:href`) of a qualified name.
    pub local: Atom<'input>,
}

impl Display for QualName<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(prefix) = self.prefix.value() {
            f.write_fmt(format_args!("{prefix}:"))?;
        }
        f.write_str(&self.local)
    }
}

impl PartialOrd for NS<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.uri().partial_cmp(other.uri())
    }
}

define_prefix! {
    SVG {
        uri: "http://www.w3.org/2000/svg",
    },
    HTML {
        name: "html",
        uri: "http://www.w3.org/1999/xhtml",
    },
    XML {
        name: "xml",
        uri: "http://www.w3.org/XML/1998/namespace",
    },
    XMLNS {
        name: "xmlns",
        uri: "http://www.w3.org/2000/xmlns/",
    },
    XLink {
        name: "xlink",
        uri: "http://www.w3.org/1999/xlink",
    },
    MathML {
        name: "mathml",
        uri: "http://www.w3.org/1998/Math/MathML",
    },
}

impl std::hash::Hash for Prefix<'_> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.ns().hash(state);
    }
}

impl Eq for Prefix<'_> {}

impl PartialEq for Prefix<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.is_ns(other.ns())
    }
}

impl Ord for Prefix<'_> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.value().cmp(&other.value())
    }
}
impl PartialOrd for Prefix<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
