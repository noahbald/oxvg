use lightningcss::stylesheet;
use oxvg_ast::{
    element::Element,
    visitor::{Context, ContextFlags, Visitor},
};
use serde::Deserialize;

use crate::utils::has_scripts;

macro_rules! jobs {
    ($($name:ident: $job:ident$(< $($t:ty),* >)?,)+) => {
        $(mod $name;)+

        $(pub use self::$name::$job;)+

        #[derive(Deserialize, Clone)]
        #[serde(rename_all = "camelCase", bound = "E: Element")]
        pub struct Jobs<E: Element> {
            $($name: Option<Box<$job $( < $($t),* >)?>>),+
        }

        impl<E: Element> Default for Jobs<E> {
            fn default() -> Self {
                Self {
                    $($name: $job::optional_default()),+
                }
            }
        }

        impl<E: Element> Jobs<E> {
            fn filter(&mut self, node: &E::ParentChild, context_flags: &ContextFlags) {
                $(if self.$name.as_mut().is_some_and(|j| <$job $( < $($t),* >)? as Job<E>>::prepare(j, node, context_flags).can_skip()) {
                    self.$name = None;
                })+
            }

            fn run_jobs(&mut self, element: &mut E, context: &mut Context<E>) -> Result<(), String> {
                $(if let Some(job) = self.$name.as_mut() {
                    job.visit(element, context)?;
                })+
                Ok(())
            }

            fn count(&self) -> usize {
                let mut i = 0;
                $(if self.$name.is_some() {
                    i += 1;
                })+
                i
            }
        }
    };
}

jobs! {
    // Non default plugins
    add_attributes_to_svg_element: AddAttributesToSVGElement,
    add_classes_to_svg: AddClassesToSVG,
    cleanup_list_of_values: CleanupListOfValues,

    // Default plugins
    remove_doctype: RemoveDoctype,
    remove_xml_proc_inst: RemoveXMLProcInst,
    remove_comments: RemoveComments,
    remove_metadata: RemoveMetadata,
    cleanup_attributes: CleanupAttributes,
    merge_styles: MergeStyles,
    inline_styles: InlineStyles<E>,
    minify_styles: MinifyStyles,
    move_elems_attrs_to_group: MoveElemsAttrsToGroup,
    cleanup_ids: CleanupIds,
    cleanup_numeric_values: CleanupNumericValues,
    convert_colors: ConvertColors,
    cleanup_enable_background: CleanupEnableBackground,
    convert_shape_to_path: ConvertShapeToPath,
    convert_ellipse_to_circle: ConvertEllipseToCircle,
    collapse_groups: CollapseGroups,
    // NOTE: This one should be before `convert_path_data` in case the order is ever changed
    apply_transforms: ApplyTransforms,
    convert_path_data: ConvertPathData,
    convert_transform: ConvertTransform,
}

pub enum PrepareOutcome {
    None,
    Skip,
}

pub trait JobDefault {
    fn optional_default() -> Option<Box<Self>>;
}

#[allow(unused_variables)]
pub trait Job<E: Element>: JobDefault + Visitor<E, Error = String> {
    fn prepare(
        &mut self,
        document: &E::ParentChild,
        context_flags: &ContextFlags,
    ) -> PrepareOutcome {
        PrepareOutcome::None
    }
}

impl<E: Element> Jobs<E> {
    pub fn run(self, root: &E::ParentChild) {
        let Some(mut root_element) = <E as Element>::from_parent(root.clone()) else {
            log::warn!("No elements found in the document, skipping");
            return;
        };

        let style_source = oxvg_ast::style::root_style(&root_element);
        let mut context: Context<'_, '_, E> = Context::new(root_element.clone());
        context
            .flags
            .set(ContextFlags::has_stylesheet, !style_source.is_empty());
        context
            .flags
            .set(ContextFlags::has_script_ref, has_scripts(&root_element));

        let mut jobs = self.clone();
        jobs.filter(root, &context.flags);
        let count = jobs.count();
        if count == 0 {
            log::debug!("All jobs were filtered out!");
            return;
        }

        let stylesheet = stylesheet::StyleSheet::parse(
            style_source.as_str(),
            stylesheet::ParserOptions::default(),
        )
        .ok();
        context.stylesheet = stylesheet;

        let _ = jobs.run_jobs(&mut root_element, &mut context);
        log::debug!("completed {count} jobs");
    }
}

impl PrepareOutcome {
    fn can_skip(&self) -> bool {
        matches!(self, Self::Skip)
    }
}

#[cfg(test)]
pub(crate) fn test_config_default_svg_comment(
    config_json: &str,
    comment: &str,
) -> anyhow::Result<String> {
    test_config(
        config_json,
        Some(&format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- {comment} -->
    test
</svg>"#
        )),
    )
}

#[cfg(test)]
pub(crate) fn test_config(config_json: &str, svg: Option<&str>) -> anyhow::Result<String> {
    use oxvg_ast::{
        implementations::markup5ever::{Element5Ever, Node5Ever},
        parse::Node,
        serialize,
    };

    let jobs: Jobs<Element5Ever> = serde_json::from_str(config_json)?;
    let dom: Node5Ever = Node::parse(svg.unwrap_or(
        r#"<svg xmlns="http://www.w3.org/2000/svg">
    test
</svg>"#,
    ))?;
    jobs.run(&dom);
    serialize::Node::serialize(&dom)
}

#[test]
fn test_jobs() -> anyhow::Result<()> {
    test_config(
        r#"{ "addAttributesToSvgElement": {
            "attributes": { "foo": "bar" }
        } }"#,
        None,
    )
    .map(|_| ())
}
