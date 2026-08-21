use oxvg_ast::{document::Document, element::Element, node::Node};
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

    /// Creates a deep copy of each selected element and puts it after the selected element. Selection moved to copies.
    ///
    /// # Errors
    ///
    /// When the root element is missing
    ///
    /// # Spec
    ///
    #[doc = include_str!("../../spec/structure/duplicate.md")]
    pub fn duplicate(&mut self) -> Result<(), Error<'input>> {
        self.effect_history(&Action::Duplicate);

        let Some(selections) = self.get_selections()? else {
            return Ok(());
        };
        let mut new_selections = Vec::with_capacity(selections.len());

        for selection in selections {
            let Some(node) = self.allocator.get(selection as usize) else {
                continue;
            };
            let Some(parent) = node.parent_node() else {
                continue;
            };
            let clone = node.clone_node(&self.allocator, true);
            parent.insert_after(clone, node);
            new_selections.push(clone);
        }

        self.effect_selection(
            &new_selections
                .into_iter()
                .map(|n| n.id() as Integer)
                .collect(),
        )?;
        self.effect_tree()?;
        self.effect_document()
    }

    /// Wraps each selected element in the given element. Adjacent selections will be grouped within the
    /// same element. Selection moved to the created elements.
    ///
    /// # Errors
    ///
    /// When root element is missing.
    ///
    /// # Spec
    ///
    #[doc = include_str!("../../spec/structure/wrap.md")]
    pub fn wrap(&mut self, qual_name: &Atom<'input>) -> Result<(), Error<'input>> {
        self.effect_history(&Action::Wrap(qual_name.clone()));

        let Some(new_elements) = self.wrap_internal(NS::SVG, qual_name)? else {
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

    fn wrap_internal(
        &mut self,
        ns: NS<'input>,
        qual_name: &Atom<'input>,
    ) -> Result<Option<Vec<Element<'input, 'arena>>>, Error<'input>> {
        let Some((selections, document, element)) = self.creation_internal(ns, qual_name)? else {
            return Ok(None);
        };
        let mut new_elements = Vec::with_capacity(selections.len());
        for selected in selections {
            let element = document.create_element(element.clone(), &self.allocator);
            selected.replace_with(*element);
            element.append(*selected);
            new_elements.push(element);
        }
        Ok(Some(new_elements))
    }

    fn insert_internal(
        &mut self,
        ns: NS<'input>,
        qual_name: &Atom<'input>,
    ) -> Result<Option<Vec<Element<'input, 'arena>>>, Error<'input>> {
        let Some((selections, document, element)) = self.creation_internal(ns, qual_name)? else {
            return Ok(None);
        };
        let mut new_elements = Vec::with_capacity(selections.len());
        for selected in selections {
            let element = document.create_element(element.clone(), &self.allocator);
            new_elements.push(element);
            selected.append(*element);
        }
        Ok(Some(new_elements))
    }

    fn creation_internal(
        &mut self,
        ns: NS<'input>,
        qual_name: &Atom<'input>,
    ) -> Result<
        Option<(
            Vec<Element<'input, 'arena>>,
            Document<'input, 'arena>,
            ElementId<'input>,
        )>,
        Error<'input>,
    > {
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
        Ok(Some((selections, document, element)))
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
