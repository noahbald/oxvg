use std::{collections::BTreeMap, marker::PhantomData};

use serde::{de::Visitor, Deserialize};
use tendril::StrTendril;

#[derive(Default)]
pub struct Attributes(Vec<QualName>, BTreeMap<QualName, Tendril>);

struct AttributesVisitor(PhantomData<fn() -> Attributes>);

#[derive(PartialEq, PartialOrd, Eq, Ord, Clone)]
pub struct QualName(pub markup5ever::QualName);

struct QualNameVisitor(PhantomData<fn() -> QualName>);

pub struct Tendril(pub tendril::StrTendril);

pub struct TendrilVisitor(PhantomData<fn() -> Tendril>);

impl Attributes {
    pub fn contains_key(&self, key: &QualName) -> bool {
        self.1.contains_key(key)
    }

    pub fn insert(&mut self, key: QualName, value: Tendril) -> Option<Tendril> {
        self.0.push(key.clone());
        self.1.insert(key, value)
    }

    pub fn into_b_tree(&self) -> &BTreeMap<QualName, Tendril> {
        &self.1
    }

    pub fn iter(&self) -> std::collections::btree_map::Iter<'_, QualName, Tendril> {
        self.1.iter()
    }
}

impl<'a> IntoIterator for &'a Attributes {
    type Item = (&'a QualName, &'a Tendril);
    type IntoIter = std::collections::btree_map::Iter<'a, QualName, Tendril>;

    fn into_iter(self) -> Self::IntoIter {
        self.1.iter()
    }
}

impl<'de> Deserialize<'de> for Attributes {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(AttributesVisitor::new())
    }
}

impl<'a> From<&'a Vec<markup5ever::Attribute>> for Attributes {
    fn from(value: &'a Vec<markup5ever::Attribute>) -> Self {
        let mut output = Self::default();
        for attr in value {
            output.0.push(QualName(attr.name.clone()));
            output
                .1
                .insert(QualName(attr.name.clone()), Tendril(attr.value.clone()));
        }
        output
    }
}

impl From<&Attributes> for Vec<markup5ever::Attribute> {
    fn from(val: &Attributes) -> Self {
        let mut output: Vec<markup5ever::Attribute> = Vec::new();
        for attr in val {
            output.push(markup5ever::Attribute {
                name: attr.0 .0.clone(),
                value: attr.1 .0.clone(),
            });
        }
        output
    }
}

impl AttributesVisitor {
    fn new() -> Self {
        Self(PhantomData)
    }
}

impl<'de> Visitor<'de> for AttributesVisitor {
    type Value = Attributes;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("A map of attributes")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        let mut attributes = Attributes::default();

        while let Some((key, value)) = map.next_entry()? {
            attributes.insert(key, value);
        }

        Ok(attributes)
    }
}

impl<'de> Deserialize<'de> for QualName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(QualNameVisitor::new())
    }
}

impl From<markup5ever::QualName> for QualName {
    fn from(value: markup5ever::QualName) -> Self {
        Self(value)
    }
}

impl QualNameVisitor {
    fn new() -> Self {
        Self(PhantomData)
    }
}

impl<'de> Visitor<'de> for QualNameVisitor {
    type Value = QualName;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("A qualified name for an attribute as a string")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        let mut parts = v.split(':');
        let Some(prefix) = parts.next() else {
            Err(serde::de::Error::custom("attribute should have contents"))?
        };

        let (prefix, local) = match parts.next() {
            Some(local) => (
                markup5ever::Prefix::try_static(prefix),
                markup5ever::LocalName::try_static(local),
            ),
            None => (None, markup5ever::LocalName::try_static(prefix)),
        };
        let Some(local) = local else {
            Err(serde::de::Error::custom(
                "could not create local name from attribute",
            ))?
        };
        Ok(QualName(markup5ever::QualName {
            prefix,
            ns: ns!(svg),
            local,
        }))
    }
}

impl<'de> Deserialize<'de> for Tendril {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(TendrilVisitor::new())
    }
}

impl From<StrTendril> for Tendril {
    fn from(value: StrTendril) -> Self {
        Self(value)
    }
}

impl TendrilVisitor {
    fn new() -> Self {
        Self(PhantomData)
    }
}

impl<'de> Visitor<'de> for TendrilVisitor {
    type Value = Tendril;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a string")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Tendril(v.into()))
    }
}
