//! Visitors for traversing and manipulating nodes of an xml document
use std::cell::RefCell;

use lightningcss::rules::CssRuleList;

use crate::{
    arena::Allocator,
    element::Element,
    is_element,
    node::{self, Ref},
    style,
};

#[derive(derive_more::Debug, Clone)]
/// Additional information about the current run of a visitor and it's context
pub struct Info<'input, 'arena> {
    /// The path of the file being processed. This should only be used for metadata purposes
    /// and not for any filesystem requests.
    pub path: Option<std::path::PathBuf>,
    /// How many times the document has been processed so far, i.e. when it's processed
    /// multiple times for further optimisation attempts
    pub multipass_count: usize,
    #[debug(skip)]
    /// The allocator for the parsed file. Used for storing and creating new nodes within
    /// the document.
    pub allocator: Allocator<'input, 'arena>,
}

impl<'input, 'arena> Info<'input, 'arena> {
    /// Creates an instance of info with a reference to `arena` that can be used for allocating
    /// new nodes
    pub fn new(allocator: Allocator<'input, 'arena>) -> Self {
        Self {
            path: None,
            multipass_count: 0,
            allocator,
        }
    }
}

#[derive(Debug)]
/// The context struct provides information about the document and it's effects on the visited node
pub struct Context<'input, 'arena, 'i> {
    /// A parsed stylesheet for all `<style>` nodes in the document, as a result of calling
    /// [`Context::query_has_stylesheet`].
    pub query_has_stylesheet_result: Vec<RefCell<CssRuleList<'input>>>,
    /// The root element of the document
    pub root: Element<'input, 'arena>,
    /// A set of boolean flags about the document and the visited node
    pub flags: ContextFlags,
    /// Info about how the program is using the document
    pub info: &'i Info<'input, 'arena>,
}

impl<'input, 'arena, 'i> Context<'input, 'arena, 'i> {
    /// Instantiates the context with the given fields.
    ///
    /// The visitor should update the context as it visits each node.
    pub fn new(
        root: Element<'input, 'arena>,
        flags: ContextFlags,
        info: &'i Info<'input, 'arena>,
    ) -> Self {
        Self {
            query_has_stylesheet_result: vec![],
            root,
            flags,
            info,
        }
    }

    /// Queries whether a `<script>` element is within the document
    pub fn query_has_script(&mut self, root: &Element<'_, '_>) {
        self.flags
            .set(ContextFlags::query_has_script_result, has_scripts(root));
    }

    /// Queries whether a `<style>` element is within the document
    pub fn query_has_stylesheet(&mut self, root: &Element<'input, '_>) {
        self.query_has_stylesheet_result = style::root(root).collect();
        self.flags.set(
            ContextFlags::query_has_stylesheet_result,
            !self.query_has_stylesheet_result.is_empty(),
        );
    }
}

bitflags! {
    /// A set of flags controlling how a visitor should run following [Visitor::prepare]
    pub struct PrepareOutcome: usize {
        /// Nothing of importance to consider following preparation.
        const none = 0;
        /// The visitor shouldn't run following preparation.
        const skip = 1 << 0;
    }
}

impl PrepareOutcome {
    /// A shorthand to check whether the skip flag is enabled
    pub fn can_skip(&self) -> bool {
        self.contains(Self::skip)
    }
}

bitflags! {
    #[derive(Debug, Clone, Default)]
    /// A set of boolean flags about the document and the visited node
    pub struct ContextFlags: usize {
        /// Whether this element is a `foreignObject` or a child of one
        const within_foreign_object = 1 << 0;
        /// Whether to skip over the element's children or not
        const skip_children = 1 << 1;
        /// Whether the document had a script element, script href, or on-* attrs when queried
        const query_has_script_result = 1 << 2;
        /// Whether the document had a non-empty stylesheet when queried
        const query_has_stylesheet_result = 1 << 3;
    }
}

impl ContextFlags {
    /// Prevents the children of the current node from being visited
    pub fn visit_skip(&mut self) {
        log::debug!("skipping children");
        self.set(Self::skip_children, true);
    }
}

/// A trait for visiting or transforming the DOM
#[allow(unused_variables)]
pub trait Visitor<'input, 'arena> {
    /// The type of errors which may be produced by the visitor
    type Error;

    /// Visits the document
    ///
    /// # Errors
    /// Whether the visitor fails
    fn document(
        &self,
        document: &Element<'input, 'arena>,
        context: &Context<'input, 'arena, '_>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Exits the document
    ///
    /// # Errors
    /// Whether the visitor fails
    fn exit_document(
        &self,
        document: &Element<'input, 'arena>,
        context: &Context<'input, 'arena, '_>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Visits a element
    ///
    /// # Errors
    /// Whether the visitor fails
    fn element(
        &self,
        element: &Element<'input, 'arena>,
        context: &mut Context<'input, 'arena, '_>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Exits a element
    ///
    /// # Errors
    /// Whether the visitor fails
    fn exit_element(
        &self,
        element: &Element<'input, 'arena>,
        context: &mut Context<'input, 'arena, '_>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Visits the doctype
    ///
    /// # Errors
    /// Whether the visitor fails
    fn doctype(&self, doctype: Ref<'input, 'arena>) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Visits the text of a style element
    ///
    /// # Errors
    /// Whether the visitor fails
    fn style(&self, style: Ref<'input, 'arena>) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Visits a text or cdata node
    ///
    /// # Errors
    /// Whether the visitor fails
    fn text_or_cdata(&self, node: Ref<'input, 'arena>) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Visits a comment
    ///
    /// # Errors
    /// Whether the visitor fails
    fn comment(&self, comment: Ref<'input, 'arena>) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Visits a processing instruction
    ///
    /// # Errors
    /// Whether the visitor fails
    fn processing_instruction(
        &self,
        processing_instruction: Ref<'input, 'arena>,
        context: &Context<'input, 'arena, '_>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    /// After analysing the document, determines whether any extra features such as
    /// style parsing or ignoring the tree is needed
    ///
    /// # Errors
    /// Whether the visitor fails
    fn prepare(
        &self,
        document: &Element<'input, 'arena>,
        context: &mut Context<'input, 'arena, '_>,
    ) -> Result<PrepareOutcome, Self::Error> {
        Ok(PrepareOutcome::none)
    }

    /// Creates context for root and visits it
    ///
    /// # Errors
    /// If any of the visitor's methods fail
    fn start(
        &self,
        root: &mut Element<'input, 'arena>,
        info: &Info<'input, 'arena>,
        flags: Option<ContextFlags>,
    ) -> Result<PrepareOutcome, Self::Error> {
        let flags = flags.unwrap_or_default();
        let mut context = Context::new(root.clone(), flags, info);
        let prepare_outcome = self.prepare(root, &mut context)?;
        if prepare_outcome.contains(PrepareOutcome::skip) {
            return Ok(prepare_outcome);
        }
        self.visit(root, &mut context)?;

        Ok(prepare_outcome)
    }

    /// Visits an element and it's children
    ///
    /// # Errors
    /// If any of the visitor's methods fail
    fn visit(
        &self,
        element: &Element<'input, 'arena>,
        context: &mut Context<'input, 'arena, '_>,
    ) -> Result<(), Self::Error> {
        match element.node_type() {
            node::Type::Document => {
                self.document(element, context)?;
                self.visit_children(element, context)?;
                self.exit_document(element, context)
            }
            node::Type::Element => {
                log::debug!("visiting {element:?}");
                let is_root_foreign_object =
                    !context.flags.contains(ContextFlags::within_foreign_object)
                        && is_element!(element, ForeignObject);
                if is_root_foreign_object {
                    context.flags.set(ContextFlags::within_foreign_object, true);
                }
                self.element(element, context)?;

                if context.flags.contains(ContextFlags::skip_children) {
                    context.flags.set(ContextFlags::skip_children, false);
                } else {
                    self.visit_children(element, context)?;
                }
                log::debug!("left the {element:?}");
                self.exit_element(element, context)?;
                if is_root_foreign_object {
                    context
                        .flags
                        .set(ContextFlags::within_foreign_object, false);
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// Visits the children of an element
    ///
    /// # Errors
    /// If any of the visitor's methods fail
    fn visit_children(
        &self,
        parent: &Element<'input, 'arena>,
        context: &mut Context<'input, 'arena, '_>,
    ) -> Result<(), Self::Error> {
        parent
            .child_nodes_iter()
            .try_for_each(|child| match child.node_type() {
                node::Type::Document | node::Type::Element => {
                    if let Some(child) = Element::new(child) {
                        self.visit(&child, context)
                    } else {
                        Ok(())
                    }
                }
                node::Type::Style => self.style(child),
                node::Type::Text | node::Type::CDataSection => self.text_or_cdata(child),
                node::Type::Comment => self.comment(child),
                node::Type::DocumentType => self.doctype(child),
                node::Type::ProcessingInstruction => self.processing_instruction(child, context),
                node::Type::DocumentFragment => Ok(()),
            })
    }
}

/// Returns whether any potential scripting is contained in the document,
/// including one of the following
///
/// - A `<script>` element
/// - An `onbegin`, `onend`, `on...`, etc. attribute
/// - A `href="javascript:..."` URL
pub fn has_scripts(root: &Element<'_, '_>) -> bool {
    use crate::attribute::{data::Attr, AttributeGroup};

    let event = AttributeGroup::event();
    root.breadth_first().any(|element| {
        is_element!(element, Script)
            || element.attributes().into_iter().any(|attr| {
                if let Attr::Href(href) = &*attr {
                    is_element!(element, A) && href.trim_start().starts_with("javascript:")
                } else {
                    attr.name().attribute_group().intersects(event)
                }
            })
    })
}

/// Returns whether any `<style>` elements are contained in the document,
/// including one of the following
pub fn has_stylesheet(root: &Element<'_, '_>) -> bool {
    root.breadth_first()
        .any(|element| is_element!(element, Style) && !element.is_empty())
}
