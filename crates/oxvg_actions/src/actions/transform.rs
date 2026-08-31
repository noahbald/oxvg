use lightningcss::properties::transform::Matrix;
use oxvg_ast::{get_attribute_mut, set_attribute};
use oxvg_collections::attribute::{
    AttrId,
    core_attrs::Number,
    inheritable::Inheritable,
    transform::{SVGTransform, SVGTransformList},
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
        self.effect_history(&Action::Matrix(a, b, c, d, e, f));
        self.append_transform(&SVGTransform::Matrix(Matrix { a, b, c, d, e, f }))?;
        self.effect_document()
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
        self.effect_history(&Action::Translate(x, y));
        self.append_transform(&SVGTransform::Translate(x, y.unwrap_or_default()))?;
        self.effect_document()
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
        self.effect_history(&Action::Scale(x, y));
        self.append_transform(&SVGTransform::Scale(x, y.unwrap_or(x)))?;
        self.effect_document()
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
        self.effect_history(&Action::Rotate(angle, origin));
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
        self.effect_history(&Action::SkewY(angle));
        self.append_transform(&SVGTransform::SkewY(angle))?;
        self.effect_document()
    }

    fn append_transform(&mut self, transform: &SVGTransform) -> Result<(), Error<'input>> {
        let selections = self.get_selections()?;
        for element in self.get_selection_elements(selections) {
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
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use oxvg_ast::serialize::Node as _;

    use crate::Actor;

    #[test]
    fn transforms() {
        oxvg_ast::parse::roxmltree::parse(
            r#"<svg xmlns="http://www.w3.org/2000/svg"/>"#,
            |root, allocator| {
                let mut actor = Actor::new(root, allocator).unwrap();

                actor.select("svg").unwrap();
                actor.matrix(1.0, 2.0, 3.0, 4.0, 5.0, 6.0).unwrap();
                actor.translate(1.0, Some(1.0)).unwrap();
                actor.scale(1.0, Some(1.0)).unwrap();
                actor.rotate(90.0, Some((1.0, 1.0))).unwrap();
                actor.skew_x(1.0).unwrap();
                actor.skew_y(1.0).unwrap();
                insta::assert_snapshot!(actor.root.serialize().unwrap());
                insta::assert_debug_snapshot!(actor.derive_state().unwrap());
            },
        )
        .unwrap();
    }
}
