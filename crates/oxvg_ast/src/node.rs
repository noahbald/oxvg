use std::{borrow::BorrowMut, cell::RefCell, collections::BTreeMap, rc::Rc, str};

use quick_xml::events::{
    attributes::{AttrError, Attribute},
    BytesCData, BytesDecl, BytesEnd, BytesStart, BytesText,
};
use serde::{Deserialize, Serialize};

#[derive(Default, Debug)]
pub struct Root {
    pub children: Vec<Rc<RefCell<Child>>>,
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct QName(String);

impl PartialOrd for QName {
    fn lt(&self, other: &Self) -> bool {
        self.0.lt(&other.0)
    }

    fn le(&self, other: &Self) -> bool {
        self.0.le(&other.0)
    }

    fn gt(&self, other: &Self) -> bool {
        self.0.gt(&other.0)
    }

    fn ge(&self, other: &Self) -> bool {
        self.0.ge(&other.0)
    }

    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for QName {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }

    fn max(self, other: Self) -> Self
    where
        Self: Sized,
    {
        if self > other {
            self
        } else {
            other
        }
    }

    fn min(self, other: Self) -> Self
    where
        Self: Sized,
    {
        if self < other {
            self
        } else {
            other
        }
    }

    fn clamp(self, min: Self, max: Self) -> Self
    where
        Self: Sized,
        Self: PartialOrd,
    {
        Self(self.0.clamp(min.0, max.0))
    }
}

impl<'a> From<&'a QName> for &'a [u8] {
    fn from(value: &'a QName) -> Self {
        value.0.as_bytes()
    }
}

impl<'a> From<&'a QName> for quick_xml::name::QName<'a> {
    fn from(val: &'a QName) -> Self {
        quick_xml::name::QName(val.0.as_bytes())
    }
}

impl From<quick_xml::name::QName<'_>> for QName {
    fn from(value: quick_xml::name::QName<'_>) -> Self {
        QName(String::from_utf8_lossy(value.0).into())
    }
}

impl From<String> for QName {
    fn from(value: String) -> Self {
        Self(value)
    }
}

#[derive(Debug, Serialize, Deserialize, Default, PartialEq)]
pub struct Attributes {
    map: BTreeMap<QName, Vec<u8>>,
    pub order: Vec<QName>,
}

impl Attributes {
    pub fn contains_key(&self, key: &QName) -> bool {
        self.map.contains_key(key)
    }

    pub fn insert(&mut self, key: QName, value: Vec<u8>) -> Option<Vec<u8>> {
        self.order.push(key.clone());
        self.map.insert(key, value)
    }

    pub fn into_b_tree(&self) -> &BTreeMap<QName, Vec<u8>> {
        &self.map
    }

    pub fn iter(&self) -> std::collections::btree_map::Iter<'_, QName, Vec<u8>> {
        self.map.iter()
    }
}

impl<'a> IntoIterator for &'a Attributes {
    type Item = (&'a QName, &'a Vec<u8>);
    type IntoIter = std::collections::btree_map::Iter<'a, QName, Vec<u8>>;

    fn into_iter(self) -> Self::IntoIter {
        self.map.iter()
    }
}

impl TryFrom<quick_xml::events::attributes::Attributes<'_>> for Attributes {
    type Error = AttrError;

    fn try_from(value: quick_xml::events::attributes::Attributes<'_>) -> Result<Self, Self::Error> {
        let mut map = BTreeMap::new();
        let mut order = Vec::new();
        value
            .into_iter()
            .try_for_each(|attribute| -> Result<(), Self::Error> {
                let attribute = attribute?;
                let Attribute { key, value } = attribute;
                let key: QName = key.into();
                let value = value.clone().into_owned();
                map.insert(key.clone(), value);
                order.push(key);
                Ok(())
            })?;
        Ok(Self { map, order })
    }
}

#[derive(derivative::Derivative)]
#[derivative(Debug)]
pub struct Element {
    pub start: BytesStart<'static>,
    pub start_pos: usize,
    pub attributes: Attributes,
    pub end: Option<BytesEnd<'static>>,
    pub end_pos: Option<usize>,
    pub children: Vec<Rc<RefCell<Child>>>,
    #[derivative(Debug = "ignore")]
    pub parent: Parent,
    pub is_self_closing: bool,
}

impl Element {
    /// Creates a oxvg element from a quick-xml element
    ///
    /// # Errors
    /// If the element's attributes happen to have some errors
    pub fn new(
        src: &BytesStart<'_>,
        parent: &Parent,
        is_self_closing: bool,
        start_pos: usize,
    ) -> Result<Rc<RefCell<Self>>, AttrError> {
        let element = Self {
            start: src.to_owned(),
            start_pos,
            parent: parent.to_owned(),
            attributes: src.attributes().try_into()?,
            end: None,
            end_pos: None,
            children: Vec::new(),
            is_self_closing,
        };
        let element = Rc::new(RefCell::new(element));
        Ok(element)
    }

    /// Applies the end-tag information to the element from a quick-xml element
    ///
    /// # Panics
    /// This panics if the end-tag provided is non-sensical and likely a bug
    /// * If the element being ended is self-closing
    /// * If the end tag is before the start tag
    /// * If the end tag has a different tag name
    pub fn end(&mut self, src: BytesEnd<'_>, end_pos: usize) {
        assert!(!self.is_self_closing);
        assert!(self.start_pos < end_pos);

        let name = src.name();
        assert_eq!(self.start.name(), name);

        self.end = Some(src.into_owned());
        self.end_pos = Some(end_pos);
    }

    pub fn name(&self) -> quick_xml::name::QName<'_> {
        self.start.name()
    }

    pub fn span(&self) -> std::ops::Range<usize> {
        self.start_pos..self.end_pos.unwrap_or(self.start_pos + self.start.len())
    }
}

#[derive(Debug)]
pub enum Child {
    XMLDeclaration(BytesDecl<'static>),
    SGMLDeclaration { value: String },
    Doctype(BytesText<'static>),
    Instruction(BytesText<'static>),
    Comment(BytesText<'static>),
    CData(BytesCData<'static>),
    Text(BytesText<'static>),
    Element(Rc<RefCell<Element>>),
}

#[derive(Debug, Clone)]
pub enum Parent {
    Root(Rc<RefCell<Root>>),
    Element(Rc<RefCell<Element>>),
}

impl Parent {
    pub fn push_child(&mut self, child: Child) {
        let child = Rc::new(RefCell::new(child));
        self.push_rc(&child);
    }

    pub fn push_rc(&mut self, child: &Rc<RefCell<Child>>) {
        match self {
            Self::Root(r) => {
                let r: &RefCell<Root> = &*r.borrow_mut();
                let r: &mut Root = &mut r.borrow_mut();
                r.children.push(child.clone());
            }
            Self::Element(e) => {
                let e: &RefCell<Element> = &*e.borrow_mut();
                let e: &mut Element = &mut e.borrow_mut();
                e.children.push(child.clone());
            }
        }
    }

    pub fn is_root(&self) -> bool {
        matches!(self, Self::Root(_))
    }
}

impl Default for Parent {
    fn default() -> Self {
        Parent::Root(Rc::new(RefCell::new(Root::default())))
    }
}
