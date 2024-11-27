mod add_attributes_to_svg_element;
mod add_classes_to_svg;
mod apply_transforms;
mod cleanup_attributes;
mod cleanup_enable_background;
mod cleanup_ids;
mod cleanup_list_of_values;
mod cleanup_numeric_values;
mod collapse_groups;
mod convert_colors;
mod convert_ellipse_to_circle;
mod convert_path_data;
mod convert_shape_to_path;
mod convert_transform;

use lightningcss::stylesheet;
use oxvg_ast::element::Element;
use oxvg_ast::node::Node;
use serde::Deserialize;

pub use self::add_attributes_to_svg_element::AddAttributesToSVGElement;
pub use self::add_classes_to_svg::AddClassesToSVG;
pub use self::apply_transforms::ApplyTransforms;
pub use self::cleanup_attributes::CleanupAttributes;
pub use self::cleanup_enable_background::CleanupEnableBackground;
pub use self::cleanup_ids::CleanupIds;
pub use self::cleanup_list_of_values::CleanupListOfValues;
pub use self::cleanup_numeric_values::CleanupNumericValues;
pub use self::collapse_groups::CollapseGroups;
pub use self::convert_colors::ConvertColors;
pub use self::convert_ellipse_to_circle::ConvertEllipseToCircle;
pub use self::convert_path_data::ConvertPathData;
pub use self::convert_shape_to_path::ConvertShapeToPath;
pub use self::convert_transform::ConvertTransform;

pub enum PrepareOutcome {
    None,
    Skip,
}

pub struct Context {
    style: oxvg_style::ComputedStyles,
}

pub trait Job {
    fn prepare(&mut self, _document: &impl Node) -> PrepareOutcome {
        PrepareOutcome::None
    }

    fn use_style(&self, _element: &impl Element) -> bool {
        false
    }

    fn run(&self, _element: &impl Element, _context: &Context) {}

    fn breakdown<N: Node>(&mut self, _document: &N) {}
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Jobs {
    add_attributes_to_svg_element: Option<Box<AddAttributesToSVGElement>>,
    add_classes_to_svg: Option<Box<AddClassesToSVG>>,
    apply_transforms: Option<Box<ApplyTransforms>>,
    cleanup_attributes: Option<Box<CleanupAttributes>>,
    cleanup_enable_background: Option<Box<CleanupEnableBackground>>,
    cleanup_ids: Option<Box<CleanupIds>>,
    cleanup_list_of_values: Option<Box<CleanupListOfValues>>,
    cleanup_numeric_values: Option<Box<CleanupNumericValues>>,
    collapse_groups: Option<Box<CollapseGroups>>,
    convert_colors: Option<Box<ConvertColors>>,
    convert_ellipse_to_circle: Option<Box<ConvertEllipseToCircle>>,
    convert_path_data: Option<Box<ConvertPathData>>,
    convert_shape_to_path: Option<Box<ConvertShapeToPath>>,
    convert_transform: Option<Box<ConvertTransform>>,
}

impl Jobs {
    pub fn run<N: Node>(self, root: &N) {
        let mut jobs = JobRunner::new(self);
        jobs.filter(root);
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
                let use_style = jobs.use_style(&child);
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
                jobs.run(&child, &context);
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
        log::debug!("completed {} jobs", jobs.flags.iter().count());
    }
}

impl Default for Jobs {
    fn default() -> Self {
        Self {
            add_attributes_to_svg_element: None,
            add_classes_to_svg: None,
            apply_transforms: Some(Box::new(ApplyTransforms::default())),
            cleanup_attributes: Some(Box::new(CleanupAttributes::default())),
            cleanup_enable_background: Some(Box::new(CleanupEnableBackground::default())),
            cleanup_ids: Some(Box::new(CleanupIds::default())),
            cleanup_list_of_values: None,
            cleanup_numeric_values: Some(Box::new(CleanupNumericValues::default())),
            collapse_groups: Some(Box::new(CollapseGroups::default())),
            convert_colors: Some(Box::new(ConvertColors::default())),
            convert_ellipse_to_circle: Some(Box::new(ConvertEllipseToCircle::default())),
            convert_path_data: Some(Box::new(ConvertPathData::default())),
            convert_shape_to_path: Some(Box::new(ConvertShapeToPath::default())),
            convert_transform: Some(Box::new(ConvertTransform::default())),
        }
    }
}

bitflags! {
    #[derive(PartialEq)]
    struct JobFlag: usize {
        // Non default plugins
        const add_attributes_to_svg_element = 0b0000_0001;
        const add_classes_to_svg = 0b_0000_0000_0000_0010;
        const cleanup_list_of_values = 0b0_0000_0100_0000;

        // Default plugins
        const cleanup_attributes = 0b_0000_0000_0000_1000;
        const cleanup_ids = 0b00_0000_0000_0000_0010_0000;
        const cleanup_numeric_values = 0b0_0000_1000_0000;
        const convert_colors = 0b0000_0000_0010_0000_0000;
        const cleanup_enable_background = 0b000_0001_0000;
        const convert_shape_to_path = 0b01_0000_0000_0000;
        const convert_ellipse_to_circle = 0b100_0000_0000;
        const collapse_groups = 0b000_0000_0001_0000_0000;
        // NOTE: This one should be before `convert_path_data` in case the order is ever changed
        const apply_transforms = 0b00_0000_0000_0000_0100;
        const convert_path_data = 0b0_0000_1000_0000_0000;
        const convert_transform = 0b0_0010_0000_0000_0000;
    }
}

struct JobRunner {
    flags: JobFlag,
    jobs: Jobs,
}

// TODO: Put this junk behind a macro
impl JobRunner {
    fn new(jobs: Jobs) -> Self {
        Self {
            jobs,
            flags: JobFlag::all(),
        }
    }

    fn filter(&mut self, node: &impl Node) {
        self.flags = self
            .flags
            .iter()
            .filter(|f| self.is_some(f) && !self.prepare(f, node).can_skip())
            .collect();
    }

    fn is_some(&self, flag: &JobFlag) -> bool {
        match *flag {
            JobFlag::add_attributes_to_svg_element => {
                self.jobs.add_attributes_to_svg_element.is_some()
            }
            JobFlag::add_classes_to_svg => self.jobs.add_classes_to_svg.is_some(),
            JobFlag::apply_transforms => self.jobs.apply_transforms.is_some(),
            JobFlag::cleanup_attributes => self.jobs.cleanup_attributes.is_some(),
            JobFlag::cleanup_enable_background => self.jobs.cleanup_enable_background.is_some(),
            JobFlag::cleanup_ids => self.jobs.cleanup_ids.is_some(),
            JobFlag::cleanup_list_of_values => self.jobs.cleanup_list_of_values.is_some(),
            JobFlag::cleanup_numeric_values => self.jobs.cleanup_numeric_values.is_some(),
            JobFlag::collapse_groups => self.jobs.collapse_groups.is_some(),
            JobFlag::convert_colors => self.jobs.convert_colors.is_some(),
            JobFlag::convert_ellipse_to_circle => self.jobs.convert_ellipse_to_circle.is_some(),
            JobFlag::convert_path_data => self.jobs.convert_path_data.is_some(),
            JobFlag::convert_shape_to_path => self.jobs.convert_shape_to_path.is_some(),
            JobFlag::convert_transform => self.jobs.convert_transform.is_some(),
            _ => unreachable!(),
        }
    }

    fn prepare(&mut self, flag: &JobFlag, node: &impl Node) -> PrepareOutcome {
        match *flag {
            JobFlag::add_attributes_to_svg_element => self
                .jobs
                .add_attributes_to_svg_element
                .as_mut()
                .unwrap()
                .prepare(node),
            JobFlag::add_classes_to_svg => {
                self.jobs.add_classes_to_svg.as_mut().unwrap().prepare(node)
            }
            JobFlag::apply_transforms => self.jobs.apply_transforms.as_mut().unwrap().prepare(node),
            JobFlag::cleanup_attributes => {
                self.jobs.cleanup_attributes.as_mut().unwrap().prepare(node)
            }
            JobFlag::cleanup_enable_background => self
                .jobs
                .cleanup_enable_background
                .as_mut()
                .unwrap()
                .prepare(node),
            JobFlag::cleanup_ids => self.jobs.cleanup_ids.as_mut().unwrap().prepare(node),
            JobFlag::cleanup_list_of_values => self
                .jobs
                .cleanup_list_of_values
                .as_mut()
                .unwrap()
                .prepare(node),
            JobFlag::cleanup_numeric_values => self
                .jobs
                .cleanup_numeric_values
                .as_mut()
                .unwrap()
                .prepare(node),
            JobFlag::collapse_groups => self.jobs.collapse_groups.as_mut().unwrap().prepare(node),
            JobFlag::convert_colors => self.jobs.convert_colors.as_mut().unwrap().prepare(node),
            JobFlag::convert_ellipse_to_circle => self
                .jobs
                .convert_ellipse_to_circle
                .as_mut()
                .unwrap()
                .prepare(node),
            JobFlag::convert_path_data => {
                self.jobs.convert_path_data.as_mut().unwrap().prepare(node)
            }
            JobFlag::convert_shape_to_path => self
                .jobs
                .convert_shape_to_path
                .as_mut()
                .unwrap()
                .prepare(node),
            JobFlag::convert_transform => {
                self.jobs.convert_transform.as_mut().unwrap().prepare(node)
            }
            _ => unreachable!(),
        }
    }

    fn use_style(&self, element: &impl Element) -> bool {
        self.flags.iter().any(|flag| match flag {
            JobFlag::add_attributes_to_svg_element => self
                .jobs
                .add_attributes_to_svg_element
                .as_ref()
                .unwrap()
                .use_style(element),
            JobFlag::add_classes_to_svg => self
                .jobs
                .add_classes_to_svg
                .as_ref()
                .unwrap()
                .use_style(element),
            JobFlag::apply_transforms => self
                .jobs
                .apply_transforms
                .as_ref()
                .unwrap()
                .use_style(element),
            JobFlag::cleanup_attributes => self
                .jobs
                .cleanup_attributes
                .as_ref()
                .unwrap()
                .use_style(element),
            JobFlag::cleanup_enable_background => self
                .jobs
                .cleanup_enable_background
                .as_ref()
                .unwrap()
                .use_style(element),
            JobFlag::cleanup_ids => self.jobs.cleanup_ids.as_ref().unwrap().use_style(element),
            JobFlag::cleanup_list_of_values => self
                .jobs
                .cleanup_list_of_values
                .as_ref()
                .unwrap()
                .use_style(element),
            JobFlag::cleanup_numeric_values => self
                .jobs
                .cleanup_numeric_values
                .as_ref()
                .unwrap()
                .use_style(element),
            JobFlag::collapse_groups => self
                .jobs
                .collapse_groups
                .as_ref()
                .unwrap()
                .use_style(element),
            JobFlag::convert_colors => self
                .jobs
                .convert_colors
                .as_ref()
                .unwrap()
                .use_style(element),
            JobFlag::convert_ellipse_to_circle => self
                .jobs
                .convert_ellipse_to_circle
                .as_ref()
                .unwrap()
                .use_style(element),
            JobFlag::convert_path_data => self
                .jobs
                .convert_path_data
                .as_ref()
                .unwrap()
                .use_style(element),
            JobFlag::convert_shape_to_path => self
                .jobs
                .convert_shape_to_path
                .as_ref()
                .unwrap()
                .use_style(element),
            JobFlag::convert_transform => self
                .jobs
                .convert_transform
                .as_ref()
                .unwrap()
                .use_style(element),
            _ => unreachable!(),
        })
    }

    fn run(&self, element: &impl Element, context: &Context) {
        for flag in self.flags.iter() {
            match flag {
                JobFlag::add_attributes_to_svg_element => {
                    self.jobs
                        .add_attributes_to_svg_element
                        .as_ref()
                        .unwrap()
                        .run(element, context);
                }
                JobFlag::add_classes_to_svg => self
                    .jobs
                    .add_classes_to_svg
                    .as_ref()
                    .unwrap()
                    .run(element, context),
                JobFlag::apply_transforms => self
                    .jobs
                    .apply_transforms
                    .as_ref()
                    .unwrap()
                    .run(element, context),
                JobFlag::cleanup_attributes => self
                    .jobs
                    .cleanup_attributes
                    .as_ref()
                    .unwrap()
                    .run(element, context),
                JobFlag::cleanup_enable_background => self
                    .jobs
                    .cleanup_enable_background
                    .as_ref()
                    .unwrap()
                    .run(element, context),
                JobFlag::cleanup_ids => self
                    .jobs
                    .cleanup_ids
                    .as_ref()
                    .unwrap()
                    .run(element, context),
                JobFlag::cleanup_list_of_values => self
                    .jobs
                    .cleanup_list_of_values
                    .as_ref()
                    .unwrap()
                    .run(element, context),
                JobFlag::cleanup_numeric_values => self
                    .jobs
                    .cleanup_numeric_values
                    .as_ref()
                    .unwrap()
                    .run(element, context),
                JobFlag::collapse_groups => self
                    .jobs
                    .collapse_groups
                    .as_ref()
                    .unwrap()
                    .run(element, context),
                JobFlag::convert_colors => self
                    .jobs
                    .convert_colors
                    .as_ref()
                    .unwrap()
                    .run(element, context),
                JobFlag::convert_ellipse_to_circle => self
                    .jobs
                    .convert_ellipse_to_circle
                    .as_ref()
                    .unwrap()
                    .run(element, context),
                JobFlag::convert_path_data => self
                    .jobs
                    .convert_path_data
                    .as_ref()
                    .unwrap()
                    .run(element, context),
                JobFlag::convert_shape_to_path => self
                    .jobs
                    .convert_shape_to_path
                    .as_ref()
                    .unwrap()
                    .run(element, context),
                JobFlag::convert_transform => self
                    .jobs
                    .convert_transform
                    .as_ref()
                    .unwrap()
                    .run(element, context),
                _ => unreachable!(),
            }
        }
    }

    fn breakdown(&mut self, document: &impl Node) {
        for flag in self.flags.iter() {
            match flag {
                JobFlag::add_attributes_to_svg_element => {
                    self.jobs
                        .add_attributes_to_svg_element
                        .as_mut()
                        .unwrap()
                        .breakdown(document);
                }
                JobFlag::add_classes_to_svg => self
                    .jobs
                    .add_classes_to_svg
                    .as_mut()
                    .unwrap()
                    .breakdown(document),
                JobFlag::apply_transforms => self
                    .jobs
                    .apply_transforms
                    .as_mut()
                    .unwrap()
                    .breakdown(document),
                JobFlag::cleanup_attributes => self
                    .jobs
                    .cleanup_attributes
                    .as_mut()
                    .unwrap()
                    .breakdown(document),
                JobFlag::cleanup_enable_background => self
                    .jobs
                    .cleanup_enable_background
                    .as_mut()
                    .unwrap()
                    .breakdown(document),
                JobFlag::cleanup_ids => self.jobs.cleanup_ids.as_mut().unwrap().breakdown(document),
                JobFlag::cleanup_list_of_values => self
                    .jobs
                    .cleanup_list_of_values
                    .as_mut()
                    .unwrap()
                    .breakdown(document),
                JobFlag::cleanup_numeric_values => self
                    .jobs
                    .cleanup_numeric_values
                    .as_mut()
                    .unwrap()
                    .breakdown(document),
                JobFlag::collapse_groups => self
                    .jobs
                    .collapse_groups
                    .as_mut()
                    .unwrap()
                    .breakdown(document),
                JobFlag::convert_colors => self
                    .jobs
                    .convert_colors
                    .as_mut()
                    .unwrap()
                    .breakdown(document),
                JobFlag::convert_ellipse_to_circle => self
                    .jobs
                    .convert_ellipse_to_circle
                    .as_mut()
                    .unwrap()
                    .breakdown(document),
                JobFlag::convert_path_data => self
                    .jobs
                    .convert_path_data
                    .as_mut()
                    .unwrap()
                    .breakdown(document),
                JobFlag::convert_shape_to_path => self
                    .jobs
                    .convert_shape_to_path
                    .as_mut()
                    .unwrap()
                    .breakdown(document),
                JobFlag::convert_transform => self
                    .jobs
                    .convert_transform
                    .as_mut()
                    .unwrap()
                    .breakdown(document),
                _ => unreachable!(),
            }
        }
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
