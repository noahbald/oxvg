use lightningcss::properties::transform::Matrix;
use oxvg_ast::{get_attribute_mut, set_attribute};
use oxvg_collections::attribute::{
    core_attrs::Number,
    inheritable::Inheritable,
    transform::{SVGTransform, SVGTransformList},
    AttrId,
};

use crate::{Action, Actor, Error};

impl<'input> Actor<'input, '_> {
    #[allow(clippy::many_single_char_names)]
    /// Appends the `matrix` function to the element's `transform` attribute.
    ///
    /// # Errors
    ///
    /// When root element is missing.
    ///
    /// # Spec
    ///
    #[doc = include_str!("../spec/manipulate/matrix.md")]
    pub fn matrix(
        &mut self,
        a: Number,
        b: Number,
        c: Number,
        d: Number,
        e: Number,
        f: Number,
    ) -> Result<(), Error<'input>> {
        self.state
            .record(&Action::Matrix(a, b, c, d, e, f), &self.allocator);
        self.append_transform(&SVGTransform::Matrix(Matrix { a, b, c, d, e, f }))
    }

    /// Appends the `translate` function to the element's `transform` attribute.
    ///
    /// # Errors
    ///
    /// When root element is missing.
    ///
    /// # Spec
    ///
    #[doc = include_str!("../spec/manipulate/translate.md")]
    pub fn translate(&mut self, x: Number, y: Option<Number>) -> Result<(), Error<'input>> {
        self.state.record(&Action::Translate(x, y), &self.allocator);
        self.append_transform(&SVGTransform::Translate(x, y.unwrap_or_default()))
    }

    /// Appends the `scale` function to the element's `transform` attribute.
    ///
    /// # Errors
    ///
    /// When root element is missing.
    ///
    /// # Spec
    ///
    #[doc = include_str!("../spec/manipulate/scale.md")]
    pub fn scale(&mut self, x: Number, y: Option<Number>) -> Result<(), Error<'input>> {
        self.state.record(&Action::Scale(x, y), &self.allocator);
        self.append_transform(&SVGTransform::Scale(x, y.unwrap_or(x)))
    }

    /// Appends the `rotate` function to the element's `transform` attribute.
    ///
    /// # Errors
    ///
    /// When root element is missing.
    ///
    /// # Spec
    ///
    #[doc = include_str!("../spec/manipulate/rotate.md")]
    pub fn rotate(
        &mut self,
        angle: Number,
        origin: Option<(Number, Number)>,
    ) -> Result<(), Error<'input>> {
        self.state
            .record(&Action::Rotate(angle, origin), &self.allocator);
        let (x, y) = origin.unwrap_or((0.0, 0.0));
        self.append_transform(&SVGTransform::Rotate(angle, x, y))
    }

    /// Appends the `skewX` function to the element's `transform` attribute.
    ///
    /// # Errors
    ///
    /// When root element is missing.
    ///
    /// # Spec
    ///
    #[doc = include_str!("../spec/manipulate/skewX.md")]
    pub fn skew_x(&mut self, angle: Number) -> Result<(), Error<'input>> {
        self.state.record(&Action::SkewX(angle), &self.allocator);
        self.append_transform(&SVGTransform::SkewX(angle))
    }

    /// Appends the `skewY` function to the element's `transform` attribute.
    ///
    /// # Errors
    ///
    /// When root element is missing.
    ///
    /// # Spec
    ///
    #[doc = include_str!("../spec/manipulate/skewY.md")]
    pub fn skew_y(&mut self, angle: Number) -> Result<(), Error<'input>> {
        self.state.record(&Action::SkewY(angle), &self.allocator);
        self.append_transform(&SVGTransform::SkewY(angle))
    }

    fn append_transform(&mut self, transform: &SVGTransform) -> Result<(), Error<'input>> {
        let Some(selections) = self.get_selections()? else {
            return Ok(());
        };
        for selection in selections {
            #[allow(clippy::cast_sign_loss)]
            let Some(node) = self.allocator.get(selection as usize) else {
                continue;
            };
            let Some(element) = node.element() else {
                continue;
            };
            if !element.qual_name().is_permitted_attribute(&AttrId::Style) {
                continue;
            }
            if let Some(transform_list) = get_attribute_mut!(element, Transform)
                .as_deref_mut()
                .and_then(Inheritable::option_mut)
            {
                transform_list.0.push(transform.clone());
            } else {
                set_attribute!(
                    element,
                    Transform(Inheritable::Defined(SVGTransformList(vec![
                        transform.clone()
                    ])))
                );
            };
        }
        Ok(())
    }
}
