use std::collections::HashMap;

use lightningcss::stylesheet;

use crate::{
    element::Element,
    node::{self, Node},
    selectors::Selector,
    style::{self, ComputedStyles, ElementData},
};

#[derive(Debug, Default, Clone)]
pub struct Info {
    pub path: Option<std::path::PathBuf>,
    pub multipass_count: usize,
}

#[derive(Debug)]
pub struct Context<'i, 'o, E: Element> {
    pub computed_styles: crate::style::ComputedStyles<'i>,
    pub stylesheet: Option<lightningcss::stylesheet::StyleSheet<'i, 'o>>,
    pub element_styles: &'i HashMap<E, ElementData<E>>,
    pub root: E,
    pub flags: ContextFlags,
    pub info: &'i Info,
}

impl<'i, E: Element> Context<'i, '_, E> {
    pub fn new(
        root: E,
        flags: ContextFlags,
        element_styles: &'i HashMap<E, ElementData<E>>,
        info: &'i Info,
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

impl PrepareOutcome {
    pub fn can_skip(&self) -> bool {
        self.contains(Self::skip)
    }
}

bitflags! {
    pub struct PrepareOutcome: usize {
        const none = 0b000_0000_0000;
        const skip = 0b000_0000_0001;
        const use_style = 0b000_0010;
    }
}

bitflags! {
    #[derive(Debug, Clone, Default)]
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
    pub fn query_has_script<E: Element>(&mut self, root: &E) {
        self.set(Self::has_script_ref, has_scripts(root));
    }

    pub fn query_has_stylesheet<E: Element>(&mut self, root: &E) {
        self.set(Self::has_stylesheet, !style::root(root).is_empty());
    }

    pub fn visit_skip(&mut self) {
        log::debug!("skipping children");
        self.set(Self::skip_children, true);
    }
}

/// A trait for visiting or transforming the DOM
#[allow(unused_variables)]
pub trait Visitor<E: Element> {
    type Error;

    /// Visits the document
    ///
    /// # Errors
    /// Whether the visitor fails
    fn document(&mut self, document: &mut E, context: &Context<E>) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Exits the document
    ///
    /// # Errors
    /// Whether the visitor fails
    fn exit_document(&mut self, document: &mut E, context: &Context<E>) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Visits a element
    ///
    /// # Errors
    /// Whether the visitor fails
    fn element(&mut self, element: &mut E, context: &mut Context<E>) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Exits a element
    ///
    /// # Errors
    /// Whether the visitor fails
    fn exit_element(
        &mut self,
        element: &mut E,
        context: &mut Context<E>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Visits the doctype
    ///
    /// # Errors
    /// Whether the visitor fails
    fn doctype(&mut self, doctype: &mut <E as Node>::Child) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Visits a text or cdata node
    ///
    /// # Errors
    /// Whether the visitor fails
    fn text_or_cdata(&mut self, node: &mut <E as Node>::Child) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Visits a comment
    ///
    /// # Errors
    /// Whether the visitor fails
    fn comment(&mut self, comment: &mut <E as Node>::Child) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Visits a processing instruction
    ///
    /// # Errors
    /// Whether the visitor fails
    fn processing_instruction(
        &mut self,
        processing_instruction: &mut <E as Node>::Child,
        context: &Context<E>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    fn use_style(&mut self, element: &E) -> bool {
        false
    }

    fn prepare(&mut self, document: &E, context_flags: &mut ContextFlags) -> PrepareOutcome {
        PrepareOutcome::none
    }

    /// Creates context for root and visits it
    ///
    /// # Errors
    /// If any of the visitor's methods fail
    fn start(&mut self, root: &mut E, info: &Info) -> Result<PrepareOutcome, Self::Error> {
        let element_styles = &mut HashMap::new();
        let mut flags = ContextFlags::empty();
        let prepare_outcome = self.prepare(root, &mut flags);
        if prepare_outcome.contains(PrepareOutcome::skip) {
            return Ok(prepare_outcome);
        }
        if prepare_outcome.contains(PrepareOutcome::use_style) {
            let style_source = style::root(root);
            flags.set(ContextFlags::use_style, true);
            flags.set(ContextFlags::has_stylesheet, !style_source.is_empty());
            let stylesheet = stylesheet::StyleSheet::parse(
                style_source.as_str(),
                stylesheet::ParserOptions::default(),
            )
            .ok();
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
        &mut self,
        element: &mut E,
        context: &mut Context<'i, '_, E>,
    ) -> Result<(), Self::Error> {
        match element.node_type() {
            node::Type::Document => {
                self.document(element, context)?;
                self.visit_children(element, context)?;
                self.exit_document(element, context)
            }
            node::Type::Element => {
                log::debug!("visiting {element:?}");
                let use_style = context.flags.contains(ContextFlags::use_style);
                if use_style && self.use_style(element) {
                    context.computed_styles = ComputedStyles::<'i>::default().with_all(
                        element,
                        &context.stylesheet,
                        context.element_styles,
                    );
                } else {
                    context.flags.set(ContextFlags::use_style, false);
                }
                let is_root_foreign_object =
                    !context.flags.contains(ContextFlags::within_foreign_object)
                        && element.prefix().is_none()
                        && element.local_name().as_ref() == "foreignObject";
                if is_root_foreign_object {
                    context.flags.set(ContextFlags::within_foreign_object, true);
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
        &mut self,
        parent: &mut E,
        context: &mut Context<'_, '_, E>,
    ) -> Result<(), Self::Error> {
        // NOTE: We use `child_nodes` for a clone instead of using `try_for_each_child`
        // Otherwise the visitor will not be able to borrow it's parent's children
        parent
            .child_nodes()
            .into_iter()
            .try_for_each(|mut child| match child.node_type() {
                node::Type::Document | node::Type::Element => {
                    if let Some(mut child) = <E as Element>::new(child) {
                        self.visit(&mut child, context)
                    } else {
                        Ok(())
                    }
                }
                node::Type::Text | node::Type::CDataSection => self.text_or_cdata(&mut child),
                node::Type::Comment => self.comment(&mut child),
                node::Type::DocumentType => self.doctype(&mut child),
                node::Type::ProcessingInstruction => {
                    self.processing_instruction(&mut child, context)
                }
                node::Type::Attribute | node::Type::DocumentFragment => Ok(()),
            })
    }
}

/// # Panics
///
/// If the built-in selector fails to construct
pub fn has_scripts<E: Element>(root: &E) -> bool {
    // PERF: Find a way to lazily evaluate selector
    root
            .find_element().map(|e| e.select_with_selector(Selector::new( "script,a[href^='javascript:'],[onbegin],[onend],[onrepeat],[onload],[onabort],[onerror],[onresize],[onscroll],[onunload],[onzoom],[oncopy],[oncut],[onpaste],[oncancel],[oncanplay],[oncanplaythrough],[onchange],[onclick],[onclose],[oncuechange],[ondblclick],[ondrag],[ondragend],[ondragenter],[ondragleave],[ondragover],[ondragstart],[ondrop],[ondurationchange],[onemptied],[onended],[onfocus],[oninput],[oninvalid],[onkeydown],[onkeypress],[onkeyup],[onloadeddata],[onloadedmetadata],[onloadstart],[onmousedown],[onmouseenter],[onmouseleave],[onmousemove],[onmouseout],[onmouseup],[onmousewheel],[onpause],[onplay],[onplaying],[onprogress],[onratechange],[onreset],[onseeked],[onseeking],[onselect],[onshow],[onstalled],[onsubmit],[onsuspend],[ontimeupdate],[ontoggle],[onvolumechange],[onwaiting],[onactivate],[onfocusin],[onfocusout],[onmouseover]" ).unwrap()))
            .is_some_and(|mut e| e.next().is_some())
}
