use std::fmt::Display;

use oxvg_ast::{
    element::Element,
    visitor::{ContextFlags, Info, PrepareOutcome, Visitor},
};
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

macro_rules! jobs {
    ($($name:ident: $job:ident$(< $($t:ty),* >)? $((is_default: $default:ident))?,)+) => {
        $(mod $name;)+

        $(pub use self::$name::$job;)+

        #[skip_serializing_none]
        #[derive(Deserialize, Serialize, Clone, Debug)]
        #[serde(rename_all = "camelCase", bound = "E: Element<'arena>")]
        /// Each task for optimising an SVG document.
        pub struct Jobs<'arena, E: Element<'arena>> {
            $(pub $name: Option<$job $( < 'arena, $($t),* >)?>),+
        }

        impl<'arena, E: Element<'arena>> Default for Jobs<'arena, E> {
            fn default() -> Self {
                macro_rules! is_default {
                    ($_job:ident $_default:ident) => { Some($_job::default()) };
                    ($_job:ident) => { None };
                }
                Self {
                    $($name: is_default!($job $($default)?)),+
                }
            }
        }

        impl<'arena, E: Element<'arena>> Jobs<'arena, E> {
            /// Runs each job in the config, returning the number of non-skipped jobs
            fn run_jobs(&mut self, element: &mut E, info: &Info<'arena, E>) -> Result<usize, String> {
                let mut count = 0;
                $(if let Some(job) = self.$name.as_mut() {
                    if !job.start(element, info)?.contains(PrepareOutcome::skip) {
                        count += 1;
                    }
                })+
                Ok(count)
            }

            /// Creates a clone of the jobs configuration for a new lifetime by cleaning up any
            /// associated data
            pub fn clone_for_lifetime<'a>(&self) -> Jobs<'a, E::Lifetimed<'a>> {
                macro_rules! is_lifetimed {
                    ($_name:ident $_job:ident $_t:ty) => { self.$_name.as_ref().map($_job::clone_for_lifetime) };
                    ($_name:ident $_job:ident) => { self.$_name.clone() };
                }
                Jobs {
                    $($name: is_lifetimed!($name $job $($($t)*)?)),+
                }
            }

            /// Overwrites `self`'s fields with the `Some` fields of `other`
            pub fn extend(&mut self, other: &Self) {
                $(if other.$name.is_some() {
                    self.$name = other.$name.clone();
                })+
            }

            /// Removes a job from the config via the specified name of the field, in `snake_case`
            pub fn omit(&mut self, name: &str) {
                match name {
                    $(stringify!($name) => self.$name = None,)+
                    _ => {}
                }
            }

            /// Produces a preset with nothing
            pub fn none() -> Self {
                Self {
                    $($name: None),+
                }
            }
        }
    };
}

jobs! {
    // Non default plugins
    precheck: Precheck,
    add_attributes_to_svg_element: AddAttributesToSVGElement,
    add_classes_to_svg: AddClassesToSVG,
    cleanup_list_of_values: CleanupListOfValues,
    prefix_ids: PrefixIds<E>,
    remove_attributes_by_selector: RemoveAttributesBySelector,
    remove_attrs: RemoveAttrs,
    remove_dimensions: RemoveDimensions,
    remove_elements_by_attr: RemoveElementsByAttr,
    remove_off_canvas_paths: RemoveOffCanvasPaths,
    remove_raster_images: RemoveRasterImages,
    remove_scripts: RemoveScripts,
    remove_style_element: RemoveStyleElement,
    remove_title: RemoveTitle,
    remove_view_box: RemoveViewBox,
    reuse_paths: ReusePaths<E>,

    // Default plugins
    remove_doctype: RemoveDoctype (is_default: true),
    remove_xml_proc_inst: RemoveXMLProcInst (is_default: true),
    remove_comments: RemoveComments (is_default: true),
    remove_deprecated_attrs: RemoveDeprecatedAttrs (is_default: true),
    remove_metadata: RemoveMetadata (is_default: true),
    cleanup_attributes: CleanupAttributes (is_default: true),
    merge_styles: MergeStyles<E> (is_default: true),
    inline_styles: InlineStyles<E> (is_default: true),
    minify_styles: MinifyStyles (is_default: true),
    cleanup_ids: CleanupIds<E> (is_default: true),
    remove_useless_defs: RemoveUselessDefs (is_default: true),
    cleanup_numeric_values: CleanupNumericValues (is_default: true),
    convert_colors: ConvertColors (is_default: true),
    remove_unknowns_and_defaults: RemoveUnknownsAndDefaults (is_default: true),
    remove_non_inheritable_group_attrs: RemoveNonInheritableGroupAttrs (is_default: true),
    remove_useless_stroke_and_fill: RemoveUselessStrokeAndFill (is_default: true),
    cleanup_enable_background: CleanupEnableBackground (is_default: true),
    remove_hidden_elems: RemoveHiddenElems<E> (is_default: true),
    remove_empty_text: RemoveEmptyText (is_default: true),
    convert_shape_to_path: ConvertShapeToPath (is_default: true),
    convert_ellipse_to_circle: ConvertEllipseToCircle (is_default: true),
    move_elems_attrs_to_group: MoveElemsAttrsToGroup (is_default: true),
    move_group_attrs_to_elems: MoveGroupAttrsToElems (is_default: true),
    collapse_groups: CollapseGroups (is_default: true),
    // NOTE: `apply_transforms` should be before `convert_path_data` in case the order is ever changed
    apply_transforms: ApplyTransforms (is_default: true),
    convert_path_data: ConvertPathData (is_default: true),
    convert_transform: ConvertTransform (is_default: true),
    remove_empty_attrs: RemoveEmptyAttrs (is_default: true),
    remove_empty_containers: RemoveEmptyContainers (is_default: true),
    merge_paths: MergePaths (is_default: true),
    sort_attrs: SortAttrs (is_default: true),
    sort_defs_children: SortDefsChildren (is_default: true),
    remove_desc: RemoveDesc (is_default: true),
}

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

impl<'arena, E: Element<'arena>> Jobs<'arena, E> {
    /// # Errors
    /// When any job fails for the first time
    pub fn run(self, root: &E::ParentChild, info: &Info<'arena, E>) -> Result<(), Error> {
        let Some(mut root_element) = <E as Element>::from_parent(root.clone()) else {
            log::warn!("No elements found in the document, skipping");
            return Ok(());
        };

        let mut jobs = self.clone();
        let count = jobs
            .run_jobs(&mut root_element, info)
            .map_err(Error::Generic)?;
        log::debug!("completed {count} jobs");
        Ok(())
    }

    /// Produces a preset focused on correctness
    pub fn safe() -> Self {
        Self {
            precheck: Some(Precheck::default()),
            ..Self::none()
        }
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
        implementations::{markup5ever::parse, shared::Element},
        serialize::{Node, Options},
    };

    let jobs: Jobs<Element> = serde_json::from_str(config_json)?;
    let arena = typed_arena::Arena::new();
    let dom = parse(
        svg.unwrap_or(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    test
</svg>"#,
        ),
        &arena,
    );
    jobs.run(&dom, &Info::new(&arena))?;
    Ok(dom.serialize_with_options(Options::default())?)
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
