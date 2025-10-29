//! XML DOM token list traits.

use std::cell::{Cell, RefMut};

use crate::{
    atom::Atom,
    attribute::{
        data::{
            list_of::{ListOf, Space},
            Attr, AttrId,
        },
        Attributes,
    },
    serialize::{PrinterOptions, ToAtom},
};

/// A list observing and manipulating a set of whitespace separated tokens.
///
/// [MDN DOMTokenList](https://developer.mozilla.org/en-US/docs/Web/API/DOMTokenList)
pub struct ClassList<'a, 'input> {
    pub(crate) attrs: Attributes<'a, 'input>,
    pub(crate) class_index_memo: Cell<usize>,
}

impl<'a, 'input: 'a> ClassList<'a, 'input> {
    fn attr(&self) -> Option<RefMut<'a, ListOf<Atom<'input>, Space>>> {
        self.attr_by_memo().or_else(|| self.attr_by_search())
    }

    fn attr_by_memo(&self) -> Option<RefMut<'a, ListOf<Atom<'input>, Space>>> {
        let attrs = self.attrs.0.borrow_mut();
        let index = self.class_index_memo.get();
        RefMut::filter_map(attrs, |a: &mut Vec<Attr<'input>>| match a.get_mut(index) {
            Some(Attr::Class(ref mut class)) => Some(class),
            _ => None,
        })
        .ok()
    }

    fn attr_by_search(&self) -> Option<RefMut<'a, ListOf<Atom<'input>, Space>>> {
        let attrs = self.attrs.0.borrow_mut();
        RefMut::filter_map(attrs, |a: &mut Vec<Attr<'input>>| {
            let (i, attr) = a.iter_mut().enumerate().find_map(|(i, attr)| match attr {
                Attr::Class(ref mut class) => Some((i, class)),
                _ => None,
            })?;
            self.class_index_memo.set(i);
            Some(attr)
        })
        .ok()
    }

    /// The number of objects stored in the object.
    ///
    /// [MDN | length](https://developer.mozilla.org/en-US/docs/Web/API/DOMTokenList/length)
    pub fn length(&self) -> usize {
        self.attr().map_or_else(|| 0, |attr| attr.len())
    }

    /// The value of the list serialized as a string
    ///
    /// [MDN | value](https://developer.mozilla.org/en-US/docs/Web/API/DOMTokenList/value)
    pub fn value(&self) -> String {
        self.attr()
            .and_then(|a| a.to_atom_string(PrinterOptions::default()).ok())
            .unwrap_or_default()
    }

    /// Adds the given token to the list, skipping if already present.
    ///
    /// [MDN | add](https://developer.mozilla.org/en-US/docs/Web/API/DOMTokenList/add)
    pub fn add(&mut self, token: Atom<'input>) {
        if self.contains(&token) {
            return;
        }
        let Some(mut attr) = self.attr() else {
            self.attrs.set_named_item(Attr::Class(ListOf {
                list: vec![token],
                seperator: Space,
            }));
            return;
        };

        attr.list.push(token);
    }

    /// Returns whether the list contains the given token.
    ///
    /// [MDN | contains](https://developer.mozilla.org/en-US/docs/Web/API/DOMTokenList/contains)
    pub fn contains(&self, token: &str) -> bool {
        match self.attr() {
            Some(attr) => attr.iter().any(|t| t.as_str() == token),
            None => false,
        }
    }

    /// Calls back the parameter once for each value in the list, in insertion order
    ///
    /// [MDN | forEach](https://developer.mozilla.org/en-US/docs/Web/API/DOMTokenList/forEach)
    pub fn for_each<F>(&self, f: F)
    where
        F: FnMut(&Atom<'input>),
    {
        self.with_iter(|iter| iter.for_each(f));
    }

    /// Returns an item in the list based on it's index.
    ///
    /// [MDN | item](https://developer.mozilla.org/en-US/docs/Web/API/DOMTokenList/item)
    pub fn item(&self, index: usize) -> Option<RefMut<'a, Atom<'input>>> {
        self.attr().and_then(|attr| {
            RefMut::filter_map(attr, |attr: &mut ListOf<Atom<'input>, Space>| {
                attr.list.get_mut(index)
            })
            .ok()
        })
    }

    /// Returns an iterator to go through all the keys in this object.
    ///
    /// [MDN | keys](https://developer.mozilla.org/en-US/docs/Web/API/DOMTokenList/item)
    pub fn keys(&self) -> impl DoubleEndedIterator<Item = usize> {
        0..self.length()
    }

    /// Removes the specified token from the list.
    ///
    /// [MDN | remove](https://developer.mozilla.org/en-US/docs/Web/API/DOMTokenList/remove)
    pub fn remove(&mut self, token: &str) {
        let Some(mut attr) = self.attr() else {
            return;
        };
        attr.list.retain(|t| t.as_str() != token);
        if attr.is_empty() {
            drop(attr);
            self.attrs.remove_named_item(&AttrId::Class);
        }
    }

    /// Replaces an existing token with a new token.
    /// If the token doesn't exist, `false` is returned without changing the list.
    ///
    /// [MDN | replace](https://developer.mozilla.org/en-US/docs/Web/API/DOMTokenList/replace)
    pub fn replace(&mut self, old_token: &str, new_token: Atom<'input>) -> bool {
        let Some(mut attr) = self.attr() else {
            return false;
        };
        let Some(token) = attr
            .list
            .iter_mut()
            .find(|token| token.as_str() == old_token)
        else {
            return false;
        };
        *token = new_token;
        true
    }

    /// Either removes the token if it exists; returning `false`, or adding the token if it doesn't;
    /// returning `true`.
    ///
    /// [MDN | toggle](https://developer.mozilla.org/en-US/docs/Web/API/DOMTokenList/toggle)
    pub fn toggle(&mut self, token: Atom<'input>) -> bool {
        if self.contains(&token) {
            self.remove(&token);
            false
        } else {
            self.add(token);
            true
        }
    }

    /// Calls the function and retains the classes where it returns `true`
    pub fn retain<F>(&self, f: F)
    where
        F: FnMut(&Atom<'input>) -> bool,
    {
        if let Some(mut attr) = self.attr() {
            attr.list.retain(f);
        }
    }

    /// Calls the function with an iterator of tokens, when it exists
    pub fn with_iter<F, O>(&self, f: F) -> Option<O>
    where
        F: FnOnce(std::slice::Iter<Atom<'input>>) -> O,
    {
        Some(f(self.attr()?.iter()))
    }
}
