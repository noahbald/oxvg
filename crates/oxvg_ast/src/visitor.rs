//! Visitors for traversing and manipulating nodes of an xml document
use lightningcss::rules::CssRuleList;

use crate::{
    arena::Arena,
    element::{data::ElementId, Element},
    node::{self, Ref},
};

use crate::style::{self, ComputedStyles, ElementData};

#[cfg(feature = "selectors")]
use crate::selectors::Selector;

#[derive(derive_more::Debug, Clone)]
/// Additional information about the current run of a visitor and it's context
pub struct Info<'arena> {
    /// The path of the file being processed. This should only be used for metadata purposes
    /// and not for any filesystem requests.
    pub path: Option<std::path::PathBuf>,
    /// How many times the document has been processed so far, i.e. when it's processed
    /// multiple times for further optimisation attempts
    pub multipass_count: usize,
    #[debug(skip)]
    /// The allocator for the parsed file. Used for storing and creating new nodes within
    /// the document.
    pub arena: Arena<'arena>,
}

impl<'arena> Info<'arena> {
    /// Creates an instance of info with a reference to `arena` that can be used for allocating
    /// new nodes
    pub fn new(arena: Arena<'arena>) -> Self {
        Self {
            path: None,
            multipass_count: 0,
            arena,
        }
    }
}

#[derive(Debug)]
/// The context struct provides information about the document and it's effects on the visited node
pub struct Context<'arena, 'i> {
    /// Uses the style sheet to compute what css properties are applied to the node
    pub computed_styles: crate::style::ComputedStyles<'arena>,
    /// A parsed stylesheet for all `<style>` nodes in the document
    pub stylesheet: Option<CssRuleList<'arena>>,
    /// A collection of the inline style and presentation attributes for each element in the document
    pub element_styles: &'i std::collections::HashMap<Element<'arena>, ElementData<'arena>>,
    /// The root element of the document
    pub root: Element<'arena>,
    /// A set of boolean flags about the document and the visited node
    pub flags: ContextFlags,
    /// Info about how the program is using the document
    pub info: &'i Info<'arena>,
}

impl<'arena, 'i> Context<'arena, 'i> {
    /// Instantiates the context with the given fields.
    ///
    /// The visitor should update the context as it visits each node.
    pub fn new(
        root: Element<'arena>,
        flags: ContextFlags,
        element_styles: &'i std::collections::HashMap<Element<'arena>, ElementData<'arena>>,
        info: &'i Info<'arena>,
    ) -> Self {
        Self {
            computed_styles: crate::style::ComputedStyles::default(),
            stylesheet: None,
            element_styles,
            root,
            flags,
            info,
        }
    }
}

bitflags! {
    /// A set of flags controlling how a visitor should run following [Visitor::prepare]
    pub struct PrepareOutcome: usize {
        /// Nothing of importance to consider following preparation.
        const none = 0b000_0000_0000;
        /// The visitor shouldn't run following preparation.
        const skip = 0b000_0000_0001;
        /// Style information should be added to context while visiting
        const use_style = 0b000_0010;
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
        /// Whether the document has a script element, script href, or on-* attrs
        const has_script_ref = 0b0001;
        /// Whether the document has a non-empty stylesheet
        const has_stylesheet = 0b0010;
        /// Whether the computed styles will be used for each element
        const use_style = 0b0100;
        /// Whether this element is a `foreignObject` or a child of one
        const within_foreign_object = 0b1000;
        /// Whether to skip over the element's children or not
        const skip_children = 0b1_0000;
    }
}

impl ContextFlags {
    #[cfg(feature = "selectors")]
    /// Queries whether a `<script>` element is within the document
    pub fn query_has_script<'arena>(&mut self, root: &Element<'arena>) {
        self.set(Self::has_script_ref, has_scripts(root));
    }

    #[cfg(all(feature = "selectors"))]
    /// Queries whether a `<style>` element is within the document
    pub fn query_has_stylesheet<'arena>(&mut self, root: &Element<'arena>) {
        self.set(Self::has_stylesheet, style::root(root).next().is_some());
    }

    /// Prevents the children of the current node from being visited
    pub fn visit_skip(&mut self) {
        log::debug!("skipping children");
        self.set(Self::skip_children, true);
    }
}

/// A trait for visiting or transforming the DOM
#[allow(unused_variables)]
pub trait Visitor<'arena> {
    /// The type of errors which may be produced by the visitor
    type Error;

    /// Visits the document
    ///
    /// # Errors
    /// Whether the visitor fails
    fn document(
        &self,
        document: &Element<'arena>,
        context: &Context<'arena, '_>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Exits the document
    ///
    /// # Errors
    /// Whether the visitor fails
    fn exit_document(
        &self,
        document: &Element<'arena>,
        context: &Context<'arena, '_>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Visits a element
    ///
    /// # Errors
    /// Whether the visitor fails
    fn element(
        &self,
        element: &Element<'arena>,
        context: &mut Context<'arena, '_>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Exits a element
    ///
    /// # Errors
    /// Whether the visitor fails
    fn exit_element(
        &self,
        element: &Element<'arena>,
        context: &mut Context<'arena, '_>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Visits the doctype
    ///
    /// # Errors
    /// Whether the visitor fails
    fn doctype(&self, doctype: Ref<'arena>) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Visits the text of a style element
    ///
    /// # Errors
    /// Whether the visitor fails
    fn style(&self, style: Ref<'arena>) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Visits a text or cdata node
    ///
    /// # Errors
    /// Whether the visitor fails
    fn text_or_cdata(&self, node: Ref<'arena>) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Visits a comment
    ///
    /// # Errors
    /// Whether the visitor fails
    fn comment(&self, comment: Ref<'arena>) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Visits a processing instruction
    ///
    /// # Errors
    /// Whether the visitor fails
    fn processing_instruction(
        &self,
        processing_instruction: Ref<'arena>,
        context: &Context<'arena, '_>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    /// For implementors, determines whether style information should
    /// be gathered and added to context prior to visiting an element.
    fn use_style(&self, element: &Element<'arena>) -> bool {
        false
    }

    /// After analysing the document, determines whether any extra features such as
    /// style parsing or ignoring the tree is needed
    ///
    /// # Errors
    /// Whether the visitor fails
    fn prepare(
        &self,
        document: &Element<'arena>,
        info: &Info<'arena>,
        context_flags: &mut ContextFlags,
    ) -> Result<PrepareOutcome, Self::Error> {
        Ok(PrepareOutcome::none)
    }

    /// Creates context for root and visits it
    ///
    /// # Errors
    /// If any of the visitor's methods fail
    fn start(
        &self,
        root: &mut Element<'arena>,
        info: &Info<'arena>,
        flags: Option<ContextFlags>,
    ) -> Result<PrepareOutcome, Self::Error> {
        let mut flags = flags.unwrap_or_default();
        let prepare_outcome = self.prepare(root, info, &mut flags)?;
        if prepare_outcome.contains(PrepareOutcome::skip) {
            return Ok(prepare_outcome);
        }
        let element_styles = &mut std::collections::HashMap::new();
        if prepare_outcome.contains(PrepareOutcome::use_style) {
            let stylesheet = flag_style_source(&mut flags, root);
            *element_styles = ElementData::new(root);
            let mut context = Context::new(root.clone(), flags, element_styles, info);
            context.stylesheet = stylesheet;
            self.visit(root, &mut context)?;
        } else {
            self.visit(
                root,
                &mut Context::new(root.clone(), flags, element_styles, info),
            )?;
        };

        Ok(prepare_outcome)
    }

    /// Visits an element and it's children
    ///
    /// # Errors
    /// If any of the visitor's methods fail
    fn visit<'i>(
        &self,
        element: &Element<'arena>,
        context: &mut Context<'arena, 'i>,
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
                        && *element.qual_name() == ElementId::ForeignObject;
                if is_root_foreign_object {
                    context.flags.set(ContextFlags::within_foreign_object, true);
                }
                let use_style = context.flags.contains(ContextFlags::use_style);
                if use_style && self.use_style(element) {
                    context.computed_styles = ComputedStyles::default().with_all(
                        element,
                        &context.stylesheet,
                        context.element_styles,
                    );
                } else {
                    context.computed_styles = ComputedStyles::default();
                    context.flags.set(ContextFlags::use_style, false);
                }
                self.element(element, context)?;
                context.flags.set(ContextFlags::use_style, use_style);

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
        parent: &Element<'arena>,
        context: &mut Context<'arena, '_>,
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
                node::Type::Attribute | node::Type::DocumentFragment => Ok(()),
            })
    }
}

fn flag_style_source<'arena>(
    flags: &mut ContextFlags,
    root: &Element<'arena>,
) -> Option<CssRuleList<'arena>> {
    let style_source = CssRuleList(style::root(root).peekable().collect());
    let has_stylesheet = !style_source.0.is_empty();
    flags.set(ContextFlags::use_style, true);
    flags.set(ContextFlags::has_stylesheet, has_stylesheet);
    if has_stylesheet {
        Some(style_source)
    } else {
        None
    }
}

#[cfg(feature = "selectors")]
/// Returns whether any potential scripting is contained in the document,
/// including one of the following
///
/// - A `<script>` element
/// - An `onbegin`, `onend`, `on...`, etc. attribute
/// - A `href="javascript:..."` URL
///
/// # Panics
///
/// If the internal selector fails to build
pub fn has_scripts<'arena>(root: &Element<'arena>) -> bool {
    // PERF: Find a way to lazily evaluate selector
    root.select_with_selector(Selector::new( "script,a[href^='javascript:'],[onbegin],[onend],[onrepeat],[onload],[onabort],[onerror],[onresize],[onscroll],[onunload],[onzoom],[oncopy],[oncut],[onpaste],[oncancel],[oncanplay],[oncanplaythrough],[onchange],[onclick],[onclose],[oncuechange],[ondblclick],[ondrag],[ondragend],[ondragenter],[ondragleave],[ondragover],[ondragstart],[ondrop],[ondurationchange],[onemptied],[onended],[onfocus],[oninput],[oninvalid],[onkeydown],[onkeypress],[onkeyup],[onloadeddata],[onloadedmetadata],[onloadstart],[onmousedown],[onmouseenter],[onmouseleave],[onmousemove],[onmouseout],[onmouseup],[onmousewheel],[onpause],[onplay],[onplaying],[onprogress],[onratechange],[onreset],[onseeked],[onseeking],[onselect],[onshow],[onstalled],[onsubmit],[onsuspend],[ontimeupdate],[ontoggle],[onvolumechange],[onwaiting],[onactivate],[onfocusin],[onfocusout],[onmouseover]" ).expect("known selector")).next().is_some()
}
