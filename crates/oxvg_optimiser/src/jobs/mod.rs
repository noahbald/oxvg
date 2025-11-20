use oxvg_ast::{
    element::Element,
    node::Ref,
    visitor::{ContextFlags, Info, PrepareOutcome, Visitor},
};
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::error::JobsError;

#[cfg(feature = "wasm")]
use tsify::Tsify;

macro_rules! jobs {
    ($($name:ident: $job:ident$(< $($t:ty),* >)? $((is_default: $default:ident))?,)+) => {
        $(mod $name;)+

        $(pub use self::$name::$job;)+

        #[skip_serializing_none]
        #[cfg_attr(feature = "napi", napi(object))]
        #[cfg_attr(feature = "wasm", derive(Tsify))]
        #[cfg_attr(feature = "wasm", tsify(from_wasm_abi, into_wasm_abi))]
        #[derive(Deserialize, Serialize, Clone, Debug)]
        #[serde(rename_all = "camelCase")]
        /// Each task for optimising an SVG document.
        pub struct Jobs {
            $(
                #[doc=concat!("See [`", stringify!($job), "`]")]
                pub $name: Option<$job $( < 'arena, $($t),* >)?>
            ),+
        }

        impl Default for Jobs {
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

        impl Jobs {
            /// Runs each job in the config, returning the number of non-skipped jobs
            fn run_jobs<'input, 'arena>(
                &self,
                element: &mut Element<'input, 'arena>,
                info: &Info<'input, 'arena>
            ) -> Result<usize, JobsError<'input>> {
                let mut count = 0;
                $(if let Some(job) = self.$name.as_ref() {
                    log::debug!(concat!("💼 starting ", stringify!($name)));
                    if !job.start(element, info, None)?.contains(PrepareOutcome::skip) {
                        count += 1;
                    }
                })+
                Ok(count)
            }

            /// Converts a JSON value of SVGO's `Config["plugins"]` into [`Jobs`].
            ///
            /// Note that this will deduplicate any plugins listed.
            ///
            /// # Errors
            ///
            /// If a config file cannot be deserialized into jobs. This may fail even if
            /// the config is valid for SVGO, such as if
            ///
            /// - The config contains custom plugins
            /// - The plugin parameters are incompatible with OXVG
            /// - The underlying deserialization process fails
            ///
            /// If you believe an errors should be fixed, please raise an issue
            /// [here](https://github.com/noahbald/oxvg/issues)
            pub fn from_svgo_plugin_config(value: Option<Vec<serde_json::Value>>) -> Result<Self, serde_json::Error> {
                use serde::de::Error as _;

                let Some(plugins) = value else { return Ok(Self::default()) };

                let to_snake_case = |name: &str| {
                    let mut output = String::with_capacity(name.len());
                    for char in name.chars() {
                        if char.is_lowercase() {
                            output.push(char);
                        } else {
                            output.push('_');
                            output.extend(char.to_lowercase());
                        }
                    }
                    output
                };

                let mut oxvg_config = serde_json::Map::new();
                for plugin in plugins {
                    if let serde_json::Value::String(svgo_name) = plugin {
                        if svgo_name == "preset-default" {
                            macro_rules! is_default {
                                ($_name:ident $_job:ident $_default:ident) => {
                                    oxvg_config.insert(
                                        String::from(stringify!($_name)),
                                        serde_json::to_value($_job::default())?
                                    )
                                };
                                ($_name:ident $_job:ident) => { () };
                            }
                            $(is_default!($name $job $($default)?);)+
                            continue;
                        }
                        let name = to_snake_case(&svgo_name);
                        match name.as_str() {
                            $(stringify!($name) => oxvg_config.insert(
                                svgo_name,
                                serde_json::to_value($job::default())?
                            ),)+
                            _ => return Err(serde_json::Error::custom(format!("unknown job `{name}`"))),
                        };
                    } else if let serde_json::Value::Object(mut plugin) = plugin {
                        let svgo_name = plugin.remove("name").ok_or_else(|| serde_json::Error::missing_field("name"))?;
                        let serde_json::Value::String(svgo_name) = svgo_name else {
                            return Err(serde_json::Error::custom("expected name to be string"));
                        };
                        let params = plugin.remove("params");
                        if let Some(params) = params {
                            oxvg_config.insert(svgo_name, params);
                        } else {
                            let name = to_snake_case(&svgo_name);
                            match name.as_str() {
                                $(stringify!($name) => oxvg_config.insert(
                                    svgo_name,
                                    serde_json::to_value($job::default())?
                                ),)+
                                _ => return Err(serde_json::Error::custom(format!("unknown job `{name}`"))),
                            };
                        }
                    }
                }

                serde_json::from_value(serde_json::Value::Object(oxvg_config))
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
    add_attributes_to_s_v_g_element: AddAttributesToSVGElement,
    add_classes_to_s_v_g_element: AddClassesToSVGElement,
    cleanup_list_of_values: CleanupListOfValues,
    convert_one_stop_gradients: ConvertOneStopGradients,
    convert_style_to_attrs: ConvertStyleToAttrs,
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
    reuse_paths: ReusePaths,
    remove_x_m_l_n_s: RemoveXMLNS,

    // Default plugins
    remove_doctype: RemoveDoctype (is_default: true),
    remove_x_m_l_proc_inst: RemoveXMLProcInst (is_default: true),
    remove_comments: RemoveComments (is_default: true),
    remove_deprecated_attrs: RemoveDeprecatedAttrs (is_default: true),
    remove_metadata: RemoveMetadata (is_default: true),
    remove_editors_n_s_data: RemoveEditorsNSData (is_default: true),
    cleanup_attrs: CleanupAttrs (is_default: true),
    merge_styles: MergeStyles (is_default: true),
    inline_styles: InlineStyles (is_default: true),
    minify_styles: MinifyStyles (is_default: true),
    cleanup_ids: CleanupIds (is_default: true),
    remove_useless_defs: RemoveUselessDefs (is_default: true),
    cleanup_numeric_values: CleanupNumericValues (is_default: true),
    convert_colors: ConvertColors (is_default: true),
    remove_unknowns_and_defaults: RemoveUnknownsAndDefaults (is_default: true),
    remove_non_inheritable_group_attrs: RemoveNonInheritableGroupAttrs (is_default: true),
    remove_useless_stroke_and_fill: RemoveUselessStrokeAndFill (is_default: true),
    cleanup_enable_background: CleanupEnableBackground (is_default: true),
    remove_hidden_elems: RemoveHiddenElems (is_default: true),
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
    remove_unused_n_s: RemoveUnusedNS (is_default: true),
    merge_paths: MergePaths (is_default: true),
    sort_attrs: SortAttrs (is_default: true),
    sort_defs_children: SortDefsChildren (is_default: true),
    remove_desc: RemoveDesc (is_default: true),

    // Final non-default plugins
    prefix_ids: PrefixIds, // Should run after `cleanup_ids`
    remove_xlink: RemoveXlink, // Should remove xlinks added by other jobs
}

impl Jobs {
    /// # Errors
    /// When any job fails for the first time
    pub fn run<'input, 'arena>(
        &self,
        root: Ref<'input, 'arena>,
        info: &Info<'input, 'arena>,
    ) -> Result<(), JobsError<'input>> {
        let Some(mut root_element) = Element::from_parent(root) else {
            log::warn!("No elements found in the document, skipping");
            return Ok(());
        };

        let count = self.run_jobs(&mut root_element, info)?;
        log::debug!("completed {count} jobs");
        Ok(())
    }

    /// Produces a preset focused on correctness
    pub fn safe() -> Self {
        Self {
            precheck: Some(Precheck::default()),
            remove_doctype: Some(RemoveDoctype::default()),
            remove_x_m_l_proc_inst: Some(RemoveXMLProcInst::default()),
            remove_comments: Some(RemoveComments::default()),
            remove_deprecated_attrs: Some(RemoveDeprecatedAttrs::default()),
            remove_metadata: Some(RemoveMetadata::default()),
            remove_editors_n_s_data: Some(RemoveEditorsNSData::default()),
            cleanup_attrs: Some(CleanupAttrs::default()),
            merge_styles: Some(MergeStyles::default()),
            inline_styles: Some(InlineStyles::default()),
            minify_styles: Some(MinifyStyles::default()),
            cleanup_ids: Some(CleanupIds::default()),
            remove_useless_defs: Some(RemoveUselessDefs::default()),
            cleanup_numeric_values: Some(CleanupNumericValues::default()),
            convert_colors: Some(ConvertColors::default()),
            remove_unknowns_and_defaults: Some(RemoveUnknownsAndDefaults::default()),
            remove_non_inheritable_group_attrs: Some(RemoveNonInheritableGroupAttrs::default()),
            remove_useless_stroke_and_fill: Some(RemoveUselessStrokeAndFill::default()),
            cleanup_enable_background: Some(CleanupEnableBackground::default()),
            remove_hidden_elems: Some(RemoveHiddenElems::default()),
            remove_empty_text: Some(RemoveEmptyText::default()),
            convert_shape_to_path: Some(ConvertShapeToPath::default()),
            convert_ellipse_to_circle: Some(ConvertEllipseToCircle::default()),
            move_elems_attrs_to_group: Some(MoveElemsAttrsToGroup::default()),
            move_group_attrs_to_elems: Some(MoveGroupAttrsToElems::default()),
            collapse_groups: Some(CollapseGroups::default()),
            apply_transforms: Some(ApplyTransforms::default()),
            convert_path_data: Some(ConvertPathData::default()),
            convert_transform: Some(ConvertTransform::default()),
            remove_empty_attrs: Some(RemoveEmptyAttrs::default()),
            remove_empty_containers: Some(RemoveEmptyContainers::default()),
            remove_unused_n_s: Some(RemoveUnusedNS::default()),
            merge_paths: Some(MergePaths::default()),
            sort_attrs: Some(SortAttrs::default()),
            sort_defs_children: Some(SortDefsChildren::default()),
            remove_desc: Some(RemoveDesc::default()),
            ..Self::none()
        }
    }
}

#[cfg(test)]
#[macro_export]
#[doc(hidden)]
macro_rules! test_config {
    ($config_json:literal, comment: $comment:literal$(,)?) => {
        $crate::jobs::test_config(
            $config_json,
            Some(concat!(
                r#"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- "#,
                $comment,
                r#" -->
    test
</svg>"#
            )),
        )
    };
    ($config_json:literal, svg: $svg:literal$(,)?) => {
        $crate::jobs::test_config($config_json, Some($svg))
    };
    ($config_json:literal) => {
        $crate::jobs::test_config($config_json, None)
    };
}

#[cfg(test)]
pub(crate) fn test_config(config_json: &str, svg: Option<&'static str>) -> anyhow::Result<String> {
    use oxvg_ast::{
        arena::Allocator,
        parse::roxmltree::parse,
        serialize::Node as _,
        serialize::Options,
        xmlwriter::{Indent, Space},
    };
    use roxmltree;

    let jobs: Jobs = serde_json::from_str(config_json)?;
    let xml = roxmltree::Document::parse_with_options(
        svg.unwrap_or(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    test
</svg>"#,
        ),
        roxmltree::ParsingOptions {
            allow_dtd: true,
            ..roxmltree::ParsingOptions::default()
        },
    )
    .unwrap();
    let values = Allocator::new_values();
    let mut arena = Allocator::new_arena();
    let mut allocator = Allocator::new(&mut arena, &values);
    let dom = parse(&xml, &mut allocator).unwrap();
    jobs.run(dom, &Info::new(allocator))
        .map_err(|e| anyhow::Error::msg(format!("{e}")))?;
    Ok(dom.serialize_with_options(Options {
        trim_whitespace: Space::Default,
        minify: true,
        ..Options::pretty()
    })?)
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
