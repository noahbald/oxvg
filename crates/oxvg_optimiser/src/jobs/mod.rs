use lightningcss::stylesheet;
use oxvg_ast::element::Element;
use oxvg_ast::node::Node;
use serde::Deserialize;

macro_rules! jobs {
    ($($name:ident: $job:ident,)+) => {
        $(mod $name;)+

        $(pub use self::$name::$job;)+

        #[derive(Deserialize, Clone)]
        #[serde(rename_all = "camelCase")]
        pub struct Jobs {
            $($name: Option<Box<$job>>),+
        }

        impl Default for Jobs {
            fn default() -> Self {
                Self {
                    $($name: $job::optional_default()),+
                }
            }
        }

        impl Jobs {
            fn filter(&mut self, node: &impl Node) {
                $(if self.$name.as_mut().is_some_and(|j| j.prepare(node).can_skip()) {
                    self.$name = None;
                })+
            }

            fn run_jobs(&self, element: &impl Element, context: &Context) {
                $(if let Some(job) = self.$name.as_ref() {
                    job.run(element, context);
                })+
            }

            fn use_style(&self, element: &impl Element) -> bool {
                $(if let Some(job) = self.$name.as_ref() {
                    if job.use_style(element) {
                        return true;
                    }
                })+
                false
            }

            fn breakdown(&mut self, node: &impl Node) {
                $(if let Some(job) = self.$name.as_mut() {
                    job.breakdown(node)
                })+
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
    cleanup_attributes: CleanupAttributes,
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

pub struct Context {
    style: oxvg_style::ComputedStyles,
}

pub trait JobDefault {
    fn optional_default() -> Option<Box<Self>>;
}

pub trait Job: JobDefault {
    fn prepare(&mut self, _document: &impl Node) -> PrepareOutcome {
        PrepareOutcome::None
    }

    fn use_style(&self, _element: &impl Element) -> bool {
        false
    }

    fn run(&self, _element: &impl Element, _context: &Context) {}

    fn breakdown<N: Node>(&mut self, _document: &N) {}
}

impl Jobs {
    pub fn run<N: Node>(self, root: &N) {
        let mut jobs = self.clone();
        jobs.filter(root);
        let count = jobs.count();
        if count == 0 {
            log::debug!("All jobs were filtered out!");
            return;
        }
        #[cfg(test)]
        let mut i = 0;

        #[cfg(test)]
        println!("~~ --- starting job");
        let Some(root_element) = root.find_element() else {
            log::warn!("No elements found in the document, skipping");
            return;
        };

        let stylesheet = &oxvg_style::root_style(&root_element);
        let stylesheet =
            stylesheet::StyleSheet::parse(stylesheet, stylesheet::ParserOptions::default()).ok();
        std::iter::once(root_element.clone())
            .chain(root_element.depth_first())
            .collect::<Vec<_>>()
            .into_iter()
            .for_each(|child| {
                #[cfg(test)]
                {
                    println!("--- element {i} {child:?}",);
                }
                let use_style = self.use_style(&child);
                let mut computed_style = oxvg_style::ComputedStyles::default();
                if let Some(s) = &stylesheet {
                    if use_style {
                        computed_style.with_all(&child, &s.rules.0);
                    }
                } else if use_style {
                    computed_style.with_inline_style(&child);
                    computed_style.with_inherited(&child, &[]);
                }

                let context = Context {
                    style: computed_style,
                };
                jobs.run_jobs(&child, &context);
                #[cfg(test)]
                {
                    println!("{}", node_to_string(&child).unwrap_or_default());
                    println!("---");
                    i += 1;
                }
            });
        #[cfg(test)]
        println!("~~ --- job ending\n\n");

        jobs.breakdown(root);
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
    use oxvg_ast::{implementations::markup5ever::Node5Ever, parse::Node};

    let jobs: Jobs = serde_json::from_str(config_json)?;
    let dom: Node5Ever = Node::parse(svg.unwrap_or(
        r#"<svg xmlns="http://www.w3.org/2000/svg">
    test
</svg>"#,
    ))?;
    jobs.run(&dom);
    node_to_string(&dom)
}

#[cfg(test)]
pub(crate) fn node_to_string(node: &impl Node) -> anyhow::Result<String> {
    use oxvg_ast::serialize::Node;

    Node::serialize(node)
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
