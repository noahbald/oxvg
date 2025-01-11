use std::fmt::Display;

use oxvg_ast::{
    element::Element,
    visitor::{ContextFlags, PrepareOutcome, Visitor},
};
use serde::Deserialize;

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
            /// Runs each job in the config, returning the number of non-skipped jobs
            fn run_jobs(&mut self, element: &mut E) -> Result<usize, String> {
                let mut count = 0;
                $(if let Some(job) = self.$name.as_mut() {
                    if !job.start(element)?.contains(PrepareOutcome::skip) {
                        count += 1;
                    }
                })+
                Ok(count)
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
    cleanup_ids: CleanupIds,
    remove_useless_defs: RemoveUselessDefs,
    cleanup_numeric_values: CleanupNumericValues,
    convert_colors: ConvertColors,
    remove_unknowns_and_defaults: RemoveUnknownsAndDefaults,
    remove_non_inheritable_group_attrs: RemoveNonInheritableGroupAttrs,
    remove_useless_stroke_and_fill: RemoveUselessStrokeAndFill,
    remove_view_box: RemoveViewBox,
    cleanup_enable_background: CleanupEnableBackground,
    remove_hidden_elems: RemoveHiddenElems<E>,
    remove_empty_text: RemoveEmptyText,
    convert_shape_to_path: ConvertShapeToPath,
    convert_ellipse_to_circle: ConvertEllipseToCircle,
    move_elems_attrs_to_group: MoveElemsAttrsToGroup,
    move_group_attrs_to_elems: MoveGroupAttrsToElems,
    collapse_groups: CollapseGroups,
    // NOTE: This one should be before `convert_path_data` in case the order is ever changed
    apply_transforms: ApplyTransforms,
    convert_path_data: ConvertPathData,
    convert_transform: ConvertTransform,
    remove_empty_attrs: RemoveEmptyAttrs,
    remove_empty_containers: RemoveEmptyContainers,
    merge_paths: MergePaths,
    sort_attrs: SortAttrs,
    sort_defs_children: SortDefsChildren,
}

pub trait JobDefault {
    fn optional_default() -> Option<Box<Self>>;
}

#[allow(unused_variables)]
pub trait Job<E: Element>: JobDefault + Visitor<E, Error = String> {}

#[derive(Debug)]
pub enum Error {
    Generic(String),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Generic(s) => s.fmt(f),
        }
    }
}

impl std::error::Error for Error {}

impl<E: Element> Jobs<E> {
    /// # Errors
    /// When any job fails for the first time
    pub fn run(self, root: &E::ParentChild) -> Result<(), Error> {
        let Some(mut root_element) = <E as Element>::from_parent(root.clone()) else {
            log::warn!("No elements found in the document, skipping");
            return Ok(());
        };

        let mut jobs = self.clone();
        let count = jobs.run_jobs(&mut root_element).map_err(Error::Generic)?;
        log::debug!("completed {count} jobs");
        Ok(())
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
    jobs.run(&dom)?;
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
