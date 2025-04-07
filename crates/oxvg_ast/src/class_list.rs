//! XML DOM token list traits.
use crate::attribute::Attr;

/// A list observing and manipulating a set of whitespace separated tokens.
///
/// [MDN DOMTokenList](https://developer.mozilla.org/en-US/docs/Web/API/DOMTokenList)
pub trait ClassList {
    /// The type of an attribute which the class-list is manipulating.
    type Attribute: Attr;

    /// The number of objects stored in the object.
    ///
    /// [MDN | length](https://developer.mozilla.org/en-US/docs/Web/API/DOMTokenList/length)
    fn length(&self) -> usize;

    /// The value of the list serialized as a string
    ///
    /// [MDN | value](https://developer.mozilla.org/en-US/docs/Web/API/DOMTokenList/value)
    fn value(&self) -> <Self::Attribute as Attr>::Atom;

    /// Adds the given token to the list, skipping if already present.
    ///
    /// [MDN | add](https://developer.mozilla.org/en-US/docs/Web/API/DOMTokenList/add)
    fn add(&mut self, token: <Self::Attribute as Attr>::Atom);

    /// Returns whether the list contains the given token.
    ///
    /// [MDN | contains](https://developer.mozilla.org/en-US/docs/Web/API/DOMTokenList/contains)
    fn contains(&self, token: &<Self::Attribute as Attr>::Atom) -> bool;

    /// Returns an iterator to go through the key/value pairs in the object.
    ///
    /// [MDN | entries](https://developer.mozilla.org/en-US/docs/Web/API/DOMTokenList/entries)
    fn entries(&self) -> impl Iterator<Item = (usize, &<Self::Attribute as Attr>::Atom)> {
        self.values().enumerate()
    }

    /// Calls back the parameter once for each value in the list, in insertion order
    ///
    /// [MDN | forEach](https://developer.mozilla.org/en-US/docs/Web/API/DOMTokenList/forEach)
    fn for_each<F>(&self, f: F)
    where
        F: FnMut(&<Self::Attribute as Attr>::Atom),
    {
        self.values().for_each(f);
    }

    /// Returns an item in the list based on it's index.
    ///
    /// [MDN | item](https://developer.mozilla.org/en-US/docs/Web/API/DOMTokenList/item)
    fn item(&self, index: usize) -> Option<&<Self::Attribute as Attr>::Atom>;

    /// Returns an iterator to go through all the keys in this object.
    ///
    /// [MDN | keys](https://developer.mozilla.org/en-US/docs/Web/API/DOMTokenList/item)
    fn keys(&self) -> impl DoubleEndedIterator<Item = usize> {
        0..self.length()
    }

    /// Removes the specified token from the list.
    ///
    /// [MDN | remove](https://developer.mozilla.org/en-US/docs/Web/API/DOMTokenList/remove)
    fn remove(&mut self, token: &<Self::Attribute as Attr>::Atom);

    /// Replaces an existing token with a new token.
    /// If the token doesn't exist, `false` is returned without changing the list.
    ///
    /// [MDN | replace](https://developer.mozilla.org/en-US/docs/Web/API/DOMTokenList/replace)
    fn replace(
        &mut self,
        old_token: <Self::Attribute as Attr>::Atom,
        new_token: <Self::Attribute as Attr>::Atom,
    ) -> bool;

    /// Either removes the token if it exists; returning `false`, or adding the token if it doesn't;
    /// returning `true`.
    ///
    /// [MDN | toggle](https://developer.mozilla.org/en-US/docs/Web/API/DOMTokenList/toggle)
    fn toggle(&mut self, token: <Self::Attribute as Attr>::Atom) -> bool {
        if self.contains(&token) {
            self.remove(&token);
            false
        } else {
            self.add(token);
            true
        }
    }

    /// Returns an iterator to go through the values in the object.
    ///
    /// [MDN | values](https://developer.mozilla.org/en-US/docs/Web/API/DOMTokenList/values)
    fn values(&self) -> impl DoubleEndedIterator<Item = &<Self::Attribute as Attr>::Atom> {
        self.iter()
    }

    /// Returns an iterator to go through the tokens in the object.
    fn iter(&self) -> impl DoubleEndedIterator<Item = &<Self::Attribute as Attr>::Atom>;
}
