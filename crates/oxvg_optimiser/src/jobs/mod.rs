use std::any::Any;

use oxvg_ast::{
    element::Element,
    node::Ref,
    visitor::{ContextFlags, Info, PrepareOutcome, Visitor},
};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::error::JobsError;

#[cfg(feature = "wasm")]
use tsify::Tsify;

/// A task for optimising an SVG document.
pub trait Job:
    for<'input, 'arena> Visitor<'input, 'arena, Error = JobsError<'input>> + std::fmt::Debug + Send
{
    /// Returns the canonical name of the job in `snake_case`
    fn name(&self) -> &'static str;

    /// Returns the canonical name of the job in `camelCase`
    fn name_camel(&self) -> &'static str;

    #[cfg(feature = "serde")]
    /// Returns the jobs configuration as a JSON value
    ///
    /// # Errors
    /// When JSON serialization fails
    fn params(&self) -> Result<serde_json::Value, serde_json::Error>;

    /// A dynamically safe method of cloning a job into a box
    fn box_clone(&self) -> Box<dyn Job>;

    #[cfg(feature = "napi")]
    /// A dynamically safe method of converting a job into a napi value
    ///
    /// # Errors
    /// If the conversion fails
    ///
    /// # Safety
    /// Calls napi's conversion method with a raw pointer
    unsafe fn to_napi_value(
        self: Box<Self>,
        raw_env: napi::sys::napi_env,
    ) -> napi::bindgen_prelude::Result<napi::sys::napi_value>;
}

#[cfg_attr(feature = "wasm", derive(Tsify))]
#[cfg_attr(feature = "wasm", tsify(from_wasm_abi, into_wasm_abi))]
#[derive(Debug)]
/// Each task for optimising an SVG document.
pub struct Jobs(pub Vec<Box<dyn Job>>);

#[cfg(feature = "napi")]
impl napi::bindgen_prelude::ToNapiValue for Jobs {
    unsafe fn to_napi_value(
        raw_env: napi::sys::napi_env,
        val: Self,
    ) -> napi::bindgen_prelude::Result<napi::sys::napi_value> {
        use napi::bindgen_prelude::*;
        let env = Env::from(raw_env);

        let plugins = val
            .0
            .into_iter()
            .map(|v| -> Result<_, _> {
                let mut obj = Object::new(&env)?;
                obj.set("name", v.name())?;
                obj.set("params", Job::to_napi_value(v, raw_env)?)?;
                Ok(obj)
            })
            .collect::<Result<Vec<_>, _>>()?;
        ToNapiValue::to_napi_value(raw_env, plugins)
    }
}

impl Jobs {
    /// # Errors
    /// When any job fails for the first time
    pub fn run<'input, 'arena>(
        &self,
        root: Ref<'input, 'arena>,
        info: &Info<'input, 'arena>,
    ) -> Result<(), JobsError<'input>> {
        let Some(root_element) = Element::from_parent(root) else {
            log::warn!("No elements found in the document, skipping");
            return Ok(());
        };

        let count = self.run_jobs(&root_element, info)?;
        log::debug!("completed {count} jobs");
        Ok(())
    }

    /// Produces a preset focused on correctness
    pub fn safe() -> Self {
        Self(vec![
            Box::new(Precheck::default()),
            Box::new(RemoveDoctype::default()),
            Box::new(RemoveXMLProcInst::default()),
            Box::new(RemoveComments::default()),
            Box::new(RemoveDeprecatedAttrs::default()),
            Box::new(RemoveMetadata::default()),
            Box::new(RemoveEditorsNSData::default()),
            Box::new(CleanupAttrs::default()),
            Box::new(MergeStyles::default()),
            Box::new(InlineStyles::default()),
            Box::new(MinifyStyles::default()),
            Box::new(CleanupIds::default()),
            Box::new(RemoveUselessDefs::default()),
            Box::new(CleanupNumericValues::default()),
            Box::new(ConvertColors::default()),
            Box::new(RemoveUnknownsAndDefaults::default()),
            Box::new(RemoveNonInheritableGroupAttrs::default()),
            Box::new(RemoveUselessStrokeAndFill::default()),
            Box::new(CleanupEnableBackground::default()),
            Box::new(RemoveHiddenElems::default()),
            Box::new(RemoveEmptyText::default()),
            Box::new(ConvertShapeToPath::default()),
            Box::new(ConvertEllipseToCircle::default()),
            Box::new(MoveElemsAttrsToGroup::default()),
            Box::new(MoveGroupAttrsToElems::default()),
            Box::new(CollapseGroups::default()),
            Box::new(ApplyTransforms::default()),
            Box::new(ConvertPathData::default()),
            Box::new(ConvertTransform::default()),
            Box::new(RemoveEmptyAttrs::default()),
            Box::new(RemoveEmptyContainers::default()),
            Box::new(RemoveUnusedNS::default()),
            Box::new(MergePaths::default()),
            Box::new(SortAttrs::default()),
            Box::new(SortDefsChildren::default()),
            Box::new(RemoveDesc::default()),
        ])
    }

    /// Runs each job in the config, returning the number of non-skipped jobs
    fn run_jobs<'input, 'arena>(
        &self,
        element: &Element<'input, 'arena>,
        info: &Info<'input, 'arena>,
    ) -> Result<usize, JobsError<'input>> {
        let mut count = 0;
        for job in &self.0 {
            log::debug!(concat!("💼 starting ", stringify!($name)));
            match job.start_with_info(element, info, None) {
                Err(e) if e.is_important() => return Err(e),
                Err(e) => log::error!("{} failed {e}", stringify!($name)),
                Ok(r) => {
                    if !r.contains(PrepareOutcome::skip) {
                        count += 1;
                    }
                }
            }
        }
        Ok(count)
    }

    /// Overwrites `self`'s jobs with jobs of `other`
    pub fn extend(&mut self, other: Self) {
        for job in other.0 {
            self.replace(job);
        }
    }

    /// Overwrites `self`'s job with the job
    pub fn replace(&mut self, job: Box<dyn Job>) {
        for self_job in &mut self.0 {
            if (**self_job).type_id() == (*job).type_id() {
                *self_job = job;
                return;
            }
        }
        self.0.push(job);
    }

    /// Removes a job from the config via the specified name of the field, in `snake_case`
    pub fn omit(&mut self, name: &str) {
        self.0.retain(|job| job.name() != name);
    }

    /// Produces a preset with nothing
    pub fn none() -> Self {
        Self(vec![])
    }
}

impl Clone for Jobs {
    fn clone(&self) -> Self {
        Self(self.0.iter().map(|job| job.box_clone()).collect())
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for Jobs {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value: Option<serde_json::Value> = Deserialize::deserialize(deserializer)?;
        match value {
            Some(serde_json::Value::Object(value)) => {
                Self::from_oxvg_struct_config(Some(value)).map_err(serde::de::Error::custom)
            }
            Some(serde_json::Value::Array(value)) => {
                Self::from_svgo_plugin_config(Some(value)).map_err(serde::de::Error::custom)
            }
            Some(_) => Err(serde::de::Error::custom(
                "Expected OXVG config as object or SVGO config as array",
            )),
            None => Ok(Self::default()),
        }
    }
}
#[cfg(feature = "serde")]
impl Serialize for Jobs {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeSeq;
        #[derive(Serialize)]
        struct Plugin {
            name: &'static str,
            params: serde_json::Value,
        }
        let mut seq = serializer.serialize_seq(Some(self.0.len()))?;
        for job in &self.0 {
            seq.serialize_element(&Plugin {
                name: job.name_camel(),
                params: job.params().map_err(serde::ser::Error::custom)?,
            })?;
        }
        seq.end()
    }
}

macro_rules! camel_case {
    ($value:expr) => {{
        static STR: &str = $value;
        static LEN: usize = len(STR);
        static BYTES: &[u8; LEN] = &convert_str::<LEN>(STR);
        // `$value` is given as `&str`, so the derived `BYTES` must be valid
        static RESULT: &str = unsafe { str::from_utf8_unchecked(BYTES) };
        RESULT
    }};
}
const fn len(value: &str) -> usize {
    let value = value.as_bytes();
    let len = value.len();
    let mut index = 0;
    let mut underscores = 0;
    while index < len {
        if value[index] == b'_' {
            underscores += 1;
        }
        index += 1;
    }
    len - underscores
}
const fn convert_str<const N: usize>(value: &str) -> [u8; N] {
    let mut bytes = [0; N];
    let source = value.as_bytes();
    let len = source.len();
    let mut bytes_index = 0;
    let mut source_index = 0;

    while source_index < len {
        let byte = source[source_index];
        source_index += 1;
        if byte == b'_' {
            bytes[bytes_index] = source[source_index].to_ascii_uppercase();
            source_index += 1;
        } else {
            bytes[bytes_index] = byte;
        }
        bytes_index += 1;
    }
    bytes
}

macro_rules! jobs {
    // $rest exhausted
    (@collect [$($result:expr,)*]) => { vec![$($result),*] };
    // Include default
    (@collect [$($result:expr,)*] $job:ident $default:tt $($rest:tt)*) => {
        jobs!(@collect [$($result,)* Box::new($job::default()),] $($rest)*)
    };
    // Skip non-default
    (@collect [$($result:expr,)*] $job:ident $($rest:tt)*) => {
        jobs!(@collect [$($result,)*] $($rest)*)
    };
    ($($name:ident: $job:ident$(< $($t:ty),* >)? $((is_default: $default:tt))?,)+) => {
        $(mod $name;)+

        $(pub use self::$name::$job;)+

        $(impl Job for $job {
            fn name(&self) -> &'static str {
                stringify!($name)
            }
            fn name_camel(&self) -> &'static str {
                camel_case!(stringify!($name))
            }
            #[cfg(feature = "serde")]
            fn params(&self) -> Result<serde_json::Value, serde_json::Error> {
                serde_json::to_value(self)
            }

            fn box_clone(&self) -> Box<dyn Job> {
                Box::new(self.clone())
            }

            #[cfg(feature = "napi")]
            unsafe fn to_napi_value(
                self: Box<Self>,
                raw_env: napi::sys::napi_env,
            ) -> napi::bindgen_prelude::Result<napi::sys::napi_value> {
                napi::bindgen_prelude::ToNapiValue::to_napi_value(raw_env, *self)
            }
        })+

        impl Default for Jobs {
            fn default() -> Self {
                Self(jobs!(@collect [] $($job $($default)?)*))
            }
        }

        #[cfg(feature = "napi")]
        impl napi::bindgen_prelude::FromNapiValue for Jobs {
            unsafe fn from_napi_value(env: napi::sys::napi_env, value: napi::sys::napi_value) -> napi::bindgen_prelude::Result<Self> {
                use napi::bindgen_prelude::*;
                let obj = unsafe { Object::from_napi_value(env, value)? };
                let mut result = Self::none();
                $(
                    if let Some(value) = obj.get::<$job>(camel_case!(stringify!($name)))?
                        .map(Box::new) {
                        result.0.push(value);
                    }
                )+
                Ok(result)
            }
        }

        impl Jobs {
            #[cfg(feature = "napi")]
            #[doc(hidden)]
            pub fn napi_from_svgo_plugin_config(value: Option<Vec<napi::bindgen_prelude::Unknown>>) -> napi::Result<Self> {
                use napi::{bindgen_prelude::{Status, Object}, ValueType};
                let Some(plugins) = value else { return Ok(Self::default()) };

                let mut oxvg_config = Self::none();
                for plugin in plugins {
                    match plugin.get_type()? {
                        ValueType::String => unsafe {
                            let svgo_name: String = plugin.cast()?;
                            oxvg_config
                                .from_svgo_plugin_string(&svgo_name)
                                .map_err(|_| napi::Error::new(Status::InvalidArg, format!("unknown job `{svgo_name}`")))?;
                        }
                        ValueType::Object => unsafe {
                            let plugin: Object = plugin.cast()?;
                            let svgo_name: Option<String> = plugin.get("name")?;
                            let Some(svgo_name) = svgo_name else {
                                return Err(napi::Error::new(Status::InvalidArg, "expected name to be string"));
                            };
                            let name = snake_case(&svgo_name);
                            match name.as_str() {
                                $(stringify!($name) => oxvg_config.0.push(Box::new(plugin.get::<$job>("params")?.unwrap_or_default())),)+
                                _ => return Err(napi::Error::new(Status::InvalidArg, format!("unknown job `{name}`"))),
                            }
                        }
                        _ => return Err(napi::Error::new(Status::InvalidArg, "unexpected type")),
                    }
                }
                Ok(oxvg_config)
            }

            /// Converts a JSON object of name-config pairs into [`Jobs`].
            /// This is the recommended approach to avoid dangerous orderings
            /// of jobs.
            ///
            /// Note that this will deduplicate any plugins listed.
            ///
            /// # Errors
            ///
            /// If a config file cannot be deserialized into jobs
            ///
            /// If you believe an errors should be fixed, please raise an issue
            /// [here](https://github.com/noahbald/oxvg/issues)
            #[cfg(feature = "serde")]
            pub fn from_oxvg_struct_config(value: Option<serde_json::Map<String, serde_json::Value>>) -> Result<Self, serde_json::Error> {
                let Some(mut obj) = value else {
                    return Ok(Self::default());
                };
                let mut result = Self::none();
                $(
                    if let Some(value) = obj.remove(camel_case!(stringify!($name))) {
                        let value = serde_json::from_value::<$job>(value)?;
                        result.0.push(Box::new(value));
                    }
                )+
                Ok(result)
            }

            /// Converts a JSON value of SVGO's `Config["plugins"]` into [`Jobs`].
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
            #[cfg(feature = "serde")]
            pub fn from_svgo_plugin_config(value: Option<Vec<serde_json::Value>>) -> Result<Self, serde_json::Error> {
                use serde::de::Error as _;

                let Some(plugins) = value else { return Ok(Self::default()) };

                let mut oxvg_config = Self(Vec::with_capacity(plugins.len()));
                for plugin in plugins {
                    match plugin {
                        serde_json::Value::String(svgo_name) => {
                            oxvg_config
                                .from_svgo_plugin_string(&svgo_name)
                                .map_err(|_| serde_json::Error::custom(format!("unknown job `{svgo_name}`")))?;
                        }
                        serde_json::Value::Object(mut plugin) => {
                            let svgo_name = plugin.remove("name").ok_or_else(|| serde_json::Error::missing_field("name"))?;
                            let serde_json::Value::String(svgo_name) = svgo_name else {
                                return Err(serde_json::Error::custom("expected name to be string"));
                            };
                            let name = snake_case(&svgo_name);
                            let params = plugin.remove("params");
                            if let Some(params) = params {
                                match name.as_str() {
                                    $(stringify!($name) => oxvg_config.0.push(Box::new(serde_json::from_value::<$job>(params)?)),)+
                                    _ => return Err(serde_json::Error::custom(format!("unknown job `{name}`"))),
                                }
                            } else {
                                match name.as_str() {
                                    $(stringify!($name) => oxvg_config.0.push(Box::new($job::default())),)+
                                    _ => return Err(serde_json::Error::custom(format!("unknown job `{name}`"))),
                                };
                            }
                        }
                        _ => return Err(serde_json::Error::custom(format!("unexpected type"))),
                    }
                }

                Ok(oxvg_config)
            }

            fn from_svgo_plugin_string(&mut self, svgo_name: &str) -> Result<(), ()> {
                if svgo_name == "preset-default" {
                    self.0.extend(Self::default().0);
                    return Ok(());
                }
                let name = snake_case(&svgo_name);
                match name.as_str() {
                    $(stringify!($name) => self.0.push(Box::new($job::default())),)+
                    _ => return Err(()),
                };
                Ok(())
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
        parse::roxmltree::{parse_with_options, ParsingOptions},
        serialize::{Node as _, Options, Space},
    };

    let jobs: Jobs = serde_json::from_str(config_json)?;
    parse_with_options(
        svg.unwrap_or(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    test
</svg>"#,
        ),
        ParsingOptions {
            allow_dtd: true,
            ..ParsingOptions::default()
        },
        |dom, allocator| {
            jobs.run(dom, &Info::new(allocator))
                .map_err(|e| anyhow::Error::msg(format!("{e}")))?;
            Ok(dom.serialize_with_options(Options {
                trim_whitespace: Space::Default,
                minify: true,
                ..Options::pretty()
            })?)
        },
    )?
}

fn snake_case(value: &str) -> String {
    let mut result =
        String::with_capacity(value.len() + value.chars().filter(char::is_ascii_uppercase).count());
    for char in value.chars() {
        if char.is_ascii_uppercase() {
            result.push('_');
            result.push(char.to_ascii_lowercase());
        } else {
            result.push(char);
        }
    }
    result
}
#[cfg(test)]
mod test {
    use super::{test_config, Jobs};

    use serde::Serialize;

    #[test]
    fn test_jobs() {
        test_config(
            r#"{ "addAttributesToSvgElement": {
                "attributes": { "foo": "bar" }
            } }"#,
            None,
        )
        .unwrap();
    }

    #[test]
    fn serde_jobs_with_params() {
        let config = r#"[
  {
    "name": "addAttributesToSVGElement",
    "params": {
      "attributes": {
        "foo": "bar"
      }
    }
  }
]"#;
        let jobs: Jobs = serde_json::from_str(config).unwrap();
        assert_eq!(jobs.0.len(), 1);

        let jobs = serde_json::to_string_pretty(&jobs).unwrap();
        assert_eq!(jobs.as_str(), config);
    }

    #[test]
    fn serde_jobs_with_defaults() {
        let jobs: Jobs = serde_json::from_str(
            r#"[
  "removeDeprecatedAttrs"
]"#,
        )
        .unwrap();
        assert_eq!(jobs.0.len(), 1);

        let jobs = serde_json::to_string_pretty(&jobs).unwrap();
        assert_eq!(
            jobs.as_str(),
            r#"[
  {
    "name": "removeDeprecatedAttrs",
    "params": {
      "removeUnsafe": false
    }
  }
]"#
        );
    }

    #[test]
    fn serde_jobs_with_preset() {
        let jobs: Jobs = serde_json::from_str(
            r#"[
  "preset-default"
]"#,
        )
        .unwrap();
        assert_eq!(jobs.0.len(), 45);
    }

    #[test]
    fn serde_jobs_with_object() {
        let jobs: Jobs = serde_json::from_str(
            r#"{
  "addAttributesToSVGElement": {
      "attributes": {
        "foo": "bar"
      }
  }
}"#,
        )
        .unwrap();
        assert_eq!(jobs.0.len(), 1);

        let jobs = serde_json::to_string_pretty(&jobs).unwrap();
        assert_eq!(
            jobs.as_str(),
            r#"[
  {
    "name": "addAttributesToSVGElement",
    "params": {
      "attributes": {
        "foo": "bar"
      }
    }
  }
]"#
        );
    }

    #[test]
    fn serde_jobs_with_mix() {
        let jobs: Jobs = serde_json::from_str(
            r#"[
  "preset-default",
  "removeDeprecatedAttrs",
  {
    "name": "addAttributesToSVGElement",
    "params": {
      "attributes": {
        "foo": "bar"
      }
    }
  }
]"#,
        )
        .unwrap();
        assert_eq!(jobs.0.len(), 47);
    }
}
