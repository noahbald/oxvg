use std::fmt::Display;

use crate::atom::Atom;

macro_rules! define_prefix {
    ($($prefix:ident {
        $(name: $name:literal,)?
        uri: $uri:literal,
    },)+) => {
        macro_rules! name_else {
            ($_name:literal) => { Some(Atom::Static($_name)) };
            () => { None };
        }
        #[allow(non_upper_case_globals)]
        mod _uri {
            use crate::atom::Atom;
            $(pub const $prefix: &'static Atom<'static> = &Atom::Static($uri);)+
        }
        #[allow(non_upper_case_globals)]
        mod _ns {
            use super::NS;
            $(pub const $prefix: &'static NS<'static> = &NS::$prefix;)+
        }
        #[allow(non_upper_case_globals)]
        mod _name {
            use crate::atom::Atom;
            $(pub const $prefix: Option<Atom<'static>> = name_else!($($name)?);)+
        }

        #[derive(PartialOrd, Debug, Clone)]
        pub enum Prefix<'input> {
            $($prefix,)+
            /// A
            Unknown {
                prefix: Option<Atom<'input>>,
                ns: NS<'input>,
            },
            Aliased {
                prefix: Box<Prefix<'input>>,
                alias: Option<Atom<'input>>,
            },
        }

        #[derive(Debug, Clone, Hash)]
        pub enum NS<'input> {
            $($prefix,)+
            Unknown(Atom<'input>),
        }

        impl<'input> Prefix<'input> {
            pub fn new(ns: Atom<'input>, prefix: Option<Atom<'input>>) -> Self {
                match (&*ns, prefix.as_ref()) {
                    $(($uri, name_else!($($name)?)) => Self::$prefix,)+
                    $(($uri, _) => Self::Aliased {
                        prefix: Box::new(Self::$prefix),
                        alias: prefix,
                    },)+
                    (..) => Self::Unknown {
                        prefix,
                        ns: NS::new(ns),
                    },
                }
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

            pub fn is_ns(&self, ns: &NS<'input>) -> bool {
                match (self, ns) {
                    $((Self::$prefix, NS::$prefix) => true,)+
                    (Self::Unknown { ns, .. }, NS::Unknown( uri )) => ns.uri() == uri,
                    (Self::Aliased { prefix, .. }, ..) => prefix.is_ns(ns),
                    _ => false,
                }
            }

            pub fn is_empty(&self) -> bool {
                self.value().is_none()
            }
        }

        impl<'input> NS<'input> {
            pub fn new(uri: Atom<'input>) -> Self {
                match &*uri {
                    $($uri => Self::$prefix,)+
                    _ => Self::Unknown(uri),
                }
            }

            pub fn uri<'a>(&'a self) -> &'a Atom<'input> {
                match self {
                    $(Self::$prefix => _uri::$prefix,)+
                    Self::Unknown(uri) => uri,
                }
            }
        }
    };
}

impl Display for QualName<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if !self.prefix.is_empty() {
            f.write_fmt(format_args!("{}:", self.prefix))?;
        }
        f.write_str(&*self.local)
    }
}

impl PartialOrd for NS<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.uri().partial_cmp(other.uri())
    }
}
impl PartialEq for NS<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.uri().eq(other.uri())
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
        self.ns().hash(state)
    }
}

impl<'input> Eq for Prefix<'input> {}

impl<'input> PartialEq for Prefix<'input> {
    fn eq(&self, other: &Self) -> bool {
        self.is_ns(&other.ns())
    }
}

impl<'input> Ord for Prefix<'input> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.value().cmp(&other.value())
    }
}

/// A qualified name used for the names of tags and attributes.
#[derive(Eq, PartialEq, Debug, Clone, Hash)]
pub struct QualName<'arena> {
    /// The prefix (e.g. `xlink` of `xlink:href`) of a qualified name.
    pub prefix: Prefix<'arena>,
    /// The local name (e.g. the `href` of `xlink:href`) of a qualified name.
    pub local: Atom<'arena>,
}

impl Display for Prefix<'_> {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!("depends on parsing")
    }
}
