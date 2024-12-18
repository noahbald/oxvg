use crate::{
    element::Element,
    node::{self, Node},
};

#[derive(Debug)]
pub struct Context<'a, 'b, E: Element> {
    pub style: crate::style::ComputedStyles,
    pub stylesheet: Option<lightningcss::stylesheet::StyleSheet<'a, 'b>>,
    pub root: E,
    pub flags: ContextFlags,
}

impl<'a, 'b, E: Element> Context<'a, 'b, E> {
    pub fn new(root: E) -> Self {
        Self {
            style: crate::style::ComputedStyles::default(),
            stylesheet: None,
            root,
            flags: ContextFlags::empty(),
        }
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
        const use_computed_styles = 0b0100;
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
    fn document(&mut self, document: &mut E) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Exits the document
    ///
    /// # Errors
    /// Whether the visitor fails
    fn exit_document(&mut self, document: &mut E) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Visits a element
    ///
    /// # Errors
    /// Whether the visitor fails
    fn element(&mut self, element: &mut E, context: &Context<E>) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Exits a element
    ///
    /// # Errors
    /// Whether the visitor fails
    fn exit_element(&mut self, element: &mut E, context: &Context<E>) -> Result<(), Self::Error> {
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
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    fn use_style(&self, element: &E) -> bool {
        false
    }

    /// Visits an element and it's children
    ///
    /// # Errors
    /// If any of the visitor's methods fail
    fn visit(&mut self, element: &mut E, context: &mut Context<E>) -> Result<(), Self::Error> {
        match element.node_type() {
            node::Type::Document => {
                self.document(element)?;
                self.visit_children(&mut element.child_nodes_iter(), context)?;
                self.exit_document(element)
            }
            node::Type::Element => {
                log::debug!("visiting {element:?}");
                let mut children = element.child_nodes_iter();
                let mut element = element.clone();
                if self.use_style(&element) {
                    update_context_flag_styles(&element, context);
                }
                self.element(&mut element, context)?;
                self.visit_children(&mut children, context)?;
                log::debug!("left the {element:?}");
                self.exit_element(&mut element, context)
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
        children: &mut impl Iterator<Item = E::Child>,
        context: &mut Context<E>,
    ) -> Result<(), Self::Error> {
        for mut child in children {
            match child.node_type() {
                node::Type::Document | node::Type::Element => {
                    if let Some(mut child) = <E as Element>::new(child) {
                        self.visit(&mut child, context)?;
                    }
                }
                node::Type::Text | node::Type::CDataSection => self.text_or_cdata(&mut child)?,
                node::Type::Comment => self.comment(&mut child)?,
                node::Type::DocumentType => self.doctype(&mut child)?,
                node::Type::ProcessingInstruction => self.processing_instruction(&mut child)?,
                node::Type::Attribute | node::Type::DocumentFragment => {}
            }
        }
        Ok(())
    }
}

fn update_context_flag_styles<E: Element>(element: &E, context: &mut Context<E>) {
    use crate::style::ComputedStyles;

    let mut computed_style = ComputedStyles::default();
    if let Some(s) = &context.stylesheet {
        computed_style.with_all(element, &s.rules.0);
    } else {
        computed_style.with_inline_style(element);
        computed_style.with_inherited(element, &[]);
    }
    context.style = computed_style;
}
