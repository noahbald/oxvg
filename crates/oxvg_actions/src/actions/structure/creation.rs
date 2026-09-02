use oxvg_ast::{document::Document, element::Element, node::Ref, set_attribute};
use oxvg_collections::{
    atom::Atom,
    attribute::{Attr, core_attrs::NonWhitespace},
    element::ElementId,
    name::{NS, Prefix},
};
use oxvg_parse::Parse;

use crate::{Action, Actor, Error, utils::to_id};

#[cfg(test)]
#[allow(clippy::unnecessary_wraps)]
fn getrandom_u64() -> Result<u64, getrandom::Error> {
    Ok(0)
}
#[cfg(not(test))]
#[allow(non_upper_case_globals)]
const getrandom_u64: fn() -> Result<u64, getrandom::Error> = getrandom::u64;

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

        let Some(new_elements) = self.insert_internal(&NS::SVG, qual_name)? else {
            return Ok(());
        };

        self.effect_selection(&new_elements.into_iter().map(|n| to_id(*n)).collect())?;
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
        self.effect_history(&Action::InsertNS(uri.clone(), qual_name.clone()));

        let Some(new_elements) = self.insert_internal(&NS::new(uri.clone()), qual_name)? else {
            return Ok(());
        };

        self.effect_selection(&new_elements.into_iter().map(|n| to_id(*n)).collect())?;
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

        for node in self.get_selection_nodes(Some(selections)) {
            let Some(parent) = node.parent_node() else {
                continue;
            };
            let clone = node.clone_node(&self.allocator, true);
            parent.insert_after(clone, node);
            new_selections.push(to_id(clone));
        }

        self.effect_selection(&new_selections.into())?;
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

        let element = self.name_internal(&NS::SVG, qual_name);
        let Some(new_elements) = self.wrap_adjacent_internal(&element)? else {
            return Ok(());
        };

        self.effect_selection(&new_elements.into_iter().map(|n| to_id(*n)).collect())?;
        self.effect_tree()?;
        self.effect_document()
    }

    /// Wraps each element in `<symbol>` under the root `<svg>` and creates an adjacent `<use>` element, referencing `<symbol>` by a random id. Selects the new use elements.
    ///
    /// # Errors
    ///
    /// When root element is missing or if random id cannot be generated.
    ///
    /// # Spec
    ///
    #[doc = include_str!("../../spec/structure/clone.md")]
    pub fn clone(&mut self) -> Result<(), Error<'input>> {
        self.effect_history(&Action::Clone);

        let Some(root) = self.root.find_element() else {
            return Ok(());
        };
        let document = root.as_document();
        let Some(new_elements) = self.wrap_internal(&ElementId::Symbol)? else {
            return Ok(());
        };
        let mut use_elements = Vec::with_capacity(new_elements.len());
        for element in new_elements {
            let id = format!("#clone{}", getrandom_u64().map_err(|_| Error::GetRandom)?);
            let id_href = &*self.allocator.alloc_str(&id);
            let id = &id_href[1..];
            let r#use = document.create_element(ElementId::Use, &self.allocator);
            element.set_attribute(Attr::Id(
                NonWhitespace::parse_string(id).map_err(|e| Error::ParseError(e.to_string()))?,
            ));
            r#use.set_attribute(Attr::Href(id_href.into()));
            element.replace_with(*r#use);
            root.prepend(*element);
            use_elements.push(r#use);
        }

        self.effect_selection(&use_elements.into_iter().map(|n| to_id(*n)).collect())?;
        self.effect_tree()?;
        self.effect_document()
    }

    /// Wraps each selected element in an
    /// [anchor link element](https://developer.mozilla.org/en-US/docs/Web/SVG/Reference/Element/a).
    /// Adjacent selections will be grouped within the same link. Selection moved to links.
    ///
    /// # Errors
    ///
    /// When root element is missing.
    ///
    /// # Spec
    ///
    #[doc = include_str!("../../spec/structure/anchor_link.md")]
    pub fn anchor_link(&mut self, href: &Atom<'input>) -> Result<(), Error<'input>> {
        self.effect_history(&Action::AnchorLink(href.clone()));

        let Some(anchors) = self.wrap_adjacent_internal(&ElementId::A)? else {
            return Ok(());
        };
        for anchor in &anchors {
            set_attribute!(anchor, Href(href.clone()));
        }

        self.effect_selection(&anchors.into_iter().map(|a| to_id(*a)).collect())?;
        self.effect_tree()?;
        self.effect_document()
    }

    /// Wraps each selected element in a
    /// [group element](https://developer.mozilla.org/en-US/docs/Web/SVG/Reference/Element/g).
    /// Adjacent selections will be grouped within the same element. Selection moved to groups.
    ///
    /// # Errors
    ///
    /// When root element is missing.
    ///
    /// # Spec
    ///
    #[doc = include_str!("../../spec/structure/group.md")]
    pub fn group(&mut self) -> Result<(), Error<'input>> {
        self.effect_history(&Action::Group);

        let Some(groups) = self.wrap_adjacent_internal(&ElementId::G)? else {
            return Ok(());
        };

        self.effect_selection(&groups.into_iter().map(|a| to_id(*a)).collect())?;
        self.effect_tree()?;
        self.effect_document()
    }

    fn wrap_internal(
        &mut self,
        element: &ElementId<'input>,
    ) -> Result<Option<Vec<Element<'input, 'arena>>>, Error<'input>> {
        let Some(selections) = self.get_selections()? else {
            return Ok(None);
        };
        let selections = self.get_selection_nodes(Some(selections));
        let Some(root) = self.root.find_element() else {
            return Ok(None);
        };
        let document = root.as_document();

        let mut new_elements = Vec::new();
        for selected in selections.rev() {
            let Some(parent) = selected.parent_node() else {
                continue;
            };
            let element = document.create_element(element.clone(), &self.allocator);
            parent.replace_child(*element, selected);
            element.append(selected);
            new_elements.push(element);
        }
        Ok(Some(new_elements))
    }

    fn wrap_adjacent_internal(
        &mut self,
        element: &ElementId<'input>,
    ) -> Result<Option<Vec<Element<'input, 'arena>>>, Error<'input>> {
        let Some(selections) = self.get_selections()? else {
            return Ok(None);
        };
        let selections = self.get_selection_nodes(Some(selections));
        let Some(root) = Element::from_parent(self.root) else {
            return Ok(None);
        };
        let document = root.as_document();

        let mut groups: Vec<Vec<Ref<'input, 'arena>>> = Vec::new();
        for selected in selections {
            if let Some(last_group) = groups.last_mut() {
                let last = last_group.last().map(|e| &**e);
                if selected.previous_sibling() == last
                    || selected
                        .element()
                        .and_then(|e| e.previous_element_sibling())
                        .map(|e| &**e)
                        == last
                {
                    last_group.push(selected);
                    continue;
                }
            }
            groups.push(vec![selected]);
        }

        let mut new_elements = Vec::with_capacity(groups.len());
        for group in groups {
            let element = document.create_element(element.clone(), &self.allocator);
            let mut children = group.into_iter();
            match children
                .next()
                .and_then(|n| n.parent_node().map(|p| (p, n)))
            {
                Some((parent, child)) => {
                    parent.replace_child(*element, child);
                    element.append(child);
                    parent
                }
                None => continue,
            };
            for child in children {
                child.remove();
                element.append(child);
            }
            new_elements.push(element);
        }
        Ok(Some(new_elements))
    }

    fn insert_internal(
        &mut self,
        ns: &NS<'input>,
        qual_name: &Atom<'input>,
    ) -> Result<Option<Vec<Element<'input, 'arena>>>, Error<'input>> {
        let Some((selections, document)) = self.creation_internal()? else {
            return Ok(None);
        };
        let element = self.name_internal(ns, qual_name);
        let mut new_elements = Vec::with_capacity(selections.len());
        for selected in selections {
            let element = document.create_element(element.clone(), &self.allocator);
            new_elements.push(element);
            selected.append(*element);
        }
        Ok(Some(new_elements))
    }

    fn name_internal(&self, ns: &NS<'input>, qual_name: &Atom<'input>) -> ElementId<'input> {
        let (prefix, local_name) = match qual_name.split_once(':') {
            Some((prefix, local_name)) => (
                Some((*self.allocator.alloc_str(prefix)).into()),
                (*self.allocator.alloc_str(local_name)).into(),
            ),
            None => (None, qual_name.clone()),
        };
        let prefix = Prefix::new(ns.uri().clone(), prefix);
        ElementId::new(prefix, local_name)
    }

    fn creation_internal(
        &mut self,
    ) -> Result<Option<(Vec<Element<'input, 'arena>>, Document<'input, 'arena>)>, Error<'input>>
    {
        let Some(selections) = self.get_selections()? else {
            return Ok(None);
        };
        let Some(root) = self.root.find_element() else {
            return Ok(None);
        };
        let document = root.as_document();

        let selections = self
            .get_selection_elements(Some(selections))
            .collect::<Vec<_>>();
        Ok(Some((selections, document)))
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

    #[test]
    fn duplicate() {
        oxvg_ast::parse::roxmltree::parse(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <text>One</text>
    <text>Two</text>
</svg>"#,
            |root, allocator| {
                let mut actor = Actor::new(root, allocator).unwrap();

                actor.select("text").unwrap();
                actor.duplicate().unwrap();
                insta::assert_snapshot!(actor.root.serialize().unwrap());
                insta::assert_debug_snapshot!(actor.derive_state().unwrap());
            },
        )
        .unwrap();
    }

    #[test]
    fn wrap() {
        oxvg_ast::parse::roxmltree::parse(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <text>One</text>
    <text>Two</text>
</svg>"#,
            |root, allocator| {
                let mut actor = Actor::new(root, allocator).unwrap();

                actor.select("text").unwrap();
                actor.wrap(&"g".into()).unwrap();
                insta::assert_snapshot!(actor.root.serialize().unwrap());
                insta::assert_debug_snapshot!(actor.derive_state().unwrap());
            },
        )
        .unwrap();
    }

    #[test]
    fn clone() {
        oxvg_ast::parse::roxmltree::parse(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <text>One</text>
    <text>Two</text>
</svg>"#,
            |root, allocator| {
                let mut actor = Actor::new(root, allocator).unwrap();

                actor.select("text").unwrap();
                actor.clone().unwrap();
                insta::assert_snapshot!(actor.root.serialize().unwrap());
                insta::assert_debug_snapshot!(actor.derive_state().unwrap());
            },
        )
        .unwrap();
    }

    #[test]
    fn anchor_link() {
        oxvg_ast::parse::roxmltree::parse(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <text>One</text>
    <text>Two</text>
</svg>"#,
            |root, allocator| {
                let mut actor = Actor::new(root, allocator).unwrap();

                actor.select("text").unwrap();
                actor.anchor_link(&"#uri".into()).unwrap();
                insta::assert_snapshot!(actor.root.serialize().unwrap());
                insta::assert_debug_snapshot!(actor.derive_state().unwrap());
            },
        )
        .unwrap();
    }

    #[test]
    fn group() {
        oxvg_ast::parse::roxmltree::parse(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <text>One</text>
    <text>Two</text>
</svg>"#,
            |root, allocator| {
                let mut actor = Actor::new(root, allocator).unwrap();

                actor.select("text").unwrap();
                actor.group().unwrap();
                insta::assert_snapshot!(actor.root.serialize().unwrap());
                insta::assert_debug_snapshot!(actor.derive_state().unwrap());
            },
        )
        .unwrap();
    }
}
