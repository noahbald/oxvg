use oxvg_ast::{element::Element, node::Node};
use oxvg_collections::{
    atom::Atom,
    attribute::core_attrs::Integer,
    element::ElementId,
    name::{NS, Prefix},
};

use crate::{Action, Actor, Error};

impl<'input, 'arena> Actor<'input, 'arena> {
    /// Creates a new SVG element and inserts it into the current selection.
    ///
    /// # Errors
    ///
    /// When root element is missing.
    ///
    /// # Spec
    ///
    #[doc = include_str!("../../spec/structure/insert.md")]
    pub fn insert(&mut self, qual_name: &Atom<'input>) -> Result<(), Error<'input>> {
        self.effect_history(&Action::Insert(qual_name.clone()));

        let Some(new_elements) = self.insert_internal(NS::SVG, qual_name)? else {
            return Ok(());
        };

        self.effect_selection(
            #[allow(clippy::cast_possible_wrap)]
            &new_elements
                .into_iter()
                .map(|n| n.id() as Integer)
                .collect(),
        )?;
        self.effect_tree()?;
        self.effect_document()
    }

    /// Creates a new element and inserts it into the current selection.
    ///
    /// # Errors
    ///
    /// When root element is missing.
    ///
    /// # Spec
    ///
    #[doc = include_str!("../../spec/structure/insert_ns.md")]
    pub fn insert_ns(
        &mut self,
        uri: &Atom<'input>,
        qual_name: &Atom<'input>,
    ) -> Result<(), Error<'input>> {
        self.effect_history(&Action::Insert(qual_name.clone()));

        let Some(new_elements) = self.insert_internal(NS::new(uri.clone()), qual_name)? else {
            return Ok(());
        };

        self.effect_selection(
            #[allow(clippy::cast_possible_wrap)]
            &new_elements
                .into_iter()
                .map(|n| n.id() as Integer)
                .collect(),
        )?;
        self.effect_tree()?;
        self.effect_document()
    }

    fn insert_internal(
        &mut self,
        ns: NS<'input>,
        qual_name: &Atom<'input>,
    ) -> Result<Option<Vec<Element<'input, 'arena>>>, Error<'input>> {
        let Some(selections) = self.get_selections()? else {
            return Ok(None);
        };
        let Some(root) = Element::from_parent(self.root) else {
            return Ok(None);
        };
        let document = root.as_document();
        let (prefix, local_name) = match qual_name.split_once(':') {
            Some((prefix, local_name)) => (
                Some((*self.allocator.alloc_str(prefix)).into()),
                (*self.allocator.alloc_str(local_name)).into(),
            ),
            None => (None, qual_name.clone()),
        };
        let prefix = Prefix::new(ns.uri().clone(), prefix);
        let element = ElementId::new(prefix, local_name);

        #[allow(clippy::cast_sign_loss)]
        let selections = selections
            .into_iter()
            .filter_map(|s| self.allocator.get(s as usize))
            .filter_map(Node::element)
            .collect::<Vec<_>>();
        let mut new_elements = Vec::with_capacity(selections.len());
        for selected in selections {
            let element = document.create_element(element.clone(), &self.allocator);
            new_elements.push(element);
            selected.append(*element);
        }
        Ok(Some(new_elements))
    }
}

#[cfg(test)]
mod test {
    use oxvg_ast::serialize::Node as _;

    use crate::Actor;

    #[test]
    fn insert() {
        oxvg_ast::parse::roxmltree::parse(
            r#"<svg xmlns="http://www.w3.org/2000/svg"/>"#,
            |root, allocator| {
                let mut actor = Actor::new(root, allocator).unwrap();

                actor.select("svg").unwrap();
                actor.insert(&"path".into()).unwrap();
                actor
                    .insert_ns(
                        &"http://www.w3.org/XML/1998/namespace".into(),
                        &"xml:path".into(),
                    )
                    .unwrap();
                insta::assert_snapshot!(actor.root.serialize().unwrap());
                insta::assert_debug_snapshot!(actor.derive_state().unwrap());
            },
        )
        .unwrap();
    }
}
