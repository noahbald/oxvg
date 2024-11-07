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

use std::rc::Rc;

use lightningcss::stylesheet;
use oxvg_selectors::Element;
use rcdom::NodeData;
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
    fn prepare(&mut self, _document: &rcdom::RcDom) -> PrepareOutcome {
        PrepareOutcome::None
    }

    fn use_style(&self, _node: &Rc<rcdom::Node>) -> bool {
        false
    }

    fn run(&self, _node: &Rc<rcdom::Node>, _context: &Context) {}

    fn breakdown(&mut self, _document: &rcdom::RcDom) {}
}

#[derive(Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Jobs {
    add_attributes_to_svg_element: Option<AddAttributesToSVGElement>,
    add_classes_to_svg: Option<AddClassesToSVG>,
    apply_transforms: Option<ApplyTransforms>,
    cleanup_attributes: Option<CleanupAttributes>,
    cleanup_enable_background: Option<CleanupEnableBackground>,
    cleanup_ids: Option<CleanupIds>,
    cleanup_list_of_values: Option<CleanupListOfValues>,
    cleanup_numeric_values: Option<CleanupNumericValues>,
    collapse_groups: Option<CollapseGroups>,
    convert_colors: Option<ConvertColors>,
    convert_ellipse_to_circle: Option<ConvertEllipseToCircle>,
    convert_path_data: Option<ConvertPathData>,
    convert_shape_to_path: Option<ConvertShapeToPath>,
    convert_transform: Option<ConvertTransform>,
}

impl Jobs {
    pub fn run(self, root: &rcdom::RcDom) {
        let mut jobs: Vec<_> = self.into_iter().flatten().collect();
        jobs.retain_mut(|job| !job.prepare(root).can_skip());
        #[cfg(test)]
        let mut i = 0;

        #[cfg(test)]
        println!("~~ --- starting job");
        let root_element = Element::new(root.document.clone());
        let stylesheet = &oxvg_style::root_style(&root_element);
        let stylesheet =
            stylesheet::StyleSheet::parse(stylesheet, stylesheet::ParserOptions::default()).ok();
        root_element
            .depth_first()
            .filter(|child| matches!(child.node.data, NodeData::Element { .. }))
            .collect::<Vec<_>>()
            .into_iter()
            .for_each(|child| {
                #[cfg(test)]
                {
                    let (name, attrs) = match &child.node.data {
                        NodeData::Element { name, attrs, .. } => (
                            name.local.to_string(),
                            attrs.borrow().iter().fold(String::new(), |acc, attr| {
                                format!(r#"{acc} {}="{}""#, attr.name.local, attr.value)
                            }),
                        ),
                        _ => (String::default(), String::default()),
                    };
                    println!("--- element {i} <{name}{attrs}>",);
                }
                let use_style = jobs.iter().any(|job| job.use_style(&child.node));
                let mut computed_style = oxvg_style::ComputedStyles::default();
                if let Some(s) = &stylesheet {
                    if use_style {
                        computed_style.with_all(&child.node, &s.rules.0);
                    }
                } else if use_style {
                    computed_style.with_inline_style(&child.node);
                    computed_style.with_inherited(&child.node, &[]);
                }

                let context = Context {
                    style: computed_style,
                };
                jobs.iter().for_each(|job| job.run(&child.node, &context));
                #[cfg(test)]
                {
                    println!("{}", node_to_string(child.node.clone()).unwrap_or_default());
                    println!("---");
                    i += 1;
                }
            });
        #[cfg(test)]
        println!("~~ --- job ending\n\n");

        jobs.iter_mut().for_each(|job| job.breakdown(root));
        log::debug!("completed {} jobs", jobs.len());
    }
}

impl IntoIterator for Jobs {
    type Item = Option<Box<dyn Job>>;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        let jobs = self.clone();
        let jobs = vec![
            jobs.add_attributes_to_svg_element
                .map(|job| Box::new(job) as Box<dyn Job>),
            jobs.add_classes_to_svg
                .map(|job| Box::new(job) as Box<dyn Job>),
            jobs.apply_transforms
                .map(|job| Box::new(job) as Box<dyn Job>),
            jobs.cleanup_attributes
                .map(|job| Box::new(job) as Box<dyn Job>),
            jobs.cleanup_enable_background
                .map(|job| Box::new(job) as Box<dyn Job>),
            jobs.cleanup_ids.map(|job| Box::new(job) as Box<dyn Job>),
            jobs.cleanup_list_of_values
                .map(|job| Box::new(job) as Box<dyn Job>),
            jobs.cleanup_numeric_values
                .map(|job| Box::new(job) as Box<dyn Job>),
            jobs.collapse_groups
                .map(|job| Box::new(job) as Box<dyn Job>),
            jobs.convert_colors.map(|job| Box::new(job) as Box<dyn Job>),
            jobs.convert_ellipse_to_circle
                .map(|job| Box::new(job) as Box<dyn Job>),
            jobs.convert_path_data
                .map(|job| Box::new(job) as Box<dyn Job>),
            jobs.convert_shape_to_path
                .map(|job| Box::new(job) as Box<dyn Job>),
            jobs.convert_transform
                .map(|job| Box::new(job) as Box<dyn Job>),
        ];
        jobs.into_iter()
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
    use xml5ever::{
        driver::{parse_document, XmlParseOpts},
        tendril::TendrilSink,
    };

    let dom: rcdom::RcDom =
        parse_document(rcdom::RcDom::default(), XmlParseOpts::default()).one(svg.unwrap_or(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    test
</svg>"#,
        ));
    let jobs: Jobs = serde_json::from_str(config_json)?;
    jobs.run(&dom);
    node_to_string(dom.document)
}

#[cfg(test)]
pub(crate) fn node_to_string(node: Rc<rcdom::Node>) -> anyhow::Result<String> {
    use rcdom::SerializableHandle;
    use xml5ever::serialize::{serialize, SerializeOpts};

    let mut sink: std::io::BufWriter<_> = std::io::BufWriter::new(Vec::new());
    serialize(
        &mut sink,
        &std::convert::Into::<SerializableHandle>::into(node),
        SerializeOpts::default(),
    )?;

    let sink: Vec<_> = sink.into_inner()?;
    Ok(String::from_utf8_lossy(&sink).to_string())
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
