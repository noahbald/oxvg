use lightningcss::stylesheet;
use oxvg_ast::element::Element;
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

            fn run_jobs(&self, element: &E, context: &Context<E>) {
                $(if let Some(job) = self.$name.as_ref() {
                    job.run(element, context);
                })+
            }

            fn use_style(&self, element: &E) -> bool {
                $(if let Some(job) = self.$name.as_ref() {
                    if job.use_style(element) {
                        return true;
                    }
                })+
                false
            }

            fn breakdown(&mut self, node: &E::ParentChild) {
                $(if let Some(job) = self.$name.as_mut() {
                    <$job $( < $($t),* >)? as Job<E>>::breakdown(job, node)
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
    remove_comments: RemoveComments,
    remove_metadata: RemoveMetadata,
    cleanup_attributes: CleanupAttributes,
    merge_styles: MergeStyles,
    inline_styles: InlineStyles<E>,
    minify_styles: MinifyStyles,
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

pub struct Context<E: Element> {
    style: oxvg_style::ComputedStyles,
    root: E,
    flags: ContextFlags,
}

bitflags! {
    #[derive(Debug, Clone)]
    pub struct ContextFlags: usize {
        /// Whether the document has a script element, script href, or on-* attrs
        const has_script_ref = 0b0001;
        /// Whether the document has a non-empty stylesheet
        const has_stylesheet = 0b0010;
    }
}

pub trait JobDefault {
    fn optional_default() -> Option<Box<Self>>;
}

pub trait Job<E: Element>: JobDefault {
    fn prepare(
        &mut self,
        _document: &E::ParentChild,
        _context_flags: &ContextFlags,
    ) -> PrepareOutcome {
        PrepareOutcome::None
    }

    fn use_style(&self, _element: &E) -> bool {
        false
    }

    fn run(&self, _element: &E, _context: &Context<E>) {}

    fn breakdown(&mut self, _document: &E::ParentChild) {}
}

impl<E: Element> Jobs<E> {
    pub fn run(self, root: &E::ParentChild) {
        let Some(root_element) = <E as Element>::find_element(root.clone()) else {
            log::warn!("No elements found in the document, skipping");
            return;
        };

        let mut context_flags = ContextFlags::empty();
        let stylesheet = oxvg_style::root_style(&root_element);
        context_flags.set(ContextFlags::has_stylesheet, !stylesheet.is_empty());
        context_flags.set(ContextFlags::has_script_ref, has_scripts(&root_element));

        let mut jobs = self.clone();
        jobs.filter(root, &context_flags);
        let count = jobs.count();
        if count == 0 {
            log::debug!("All jobs were filtered out!");
            return;
        }
        #[cfg(test)]
        let mut i = 0;

        #[cfg(test)]
        println!("~~ --- starting job");

        let stylesheet =
            stylesheet::StyleSheet::parse(&stylesheet, stylesheet::ParserOptions::default()).ok();

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
                    root: root_element.clone(),
                    flags: context_flags.clone(),
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
    use oxvg_ast::{
        implementations::markup5ever::{Element5Ever, Node5Ever},
        parse::Node,
    };

    let jobs: Jobs<Element5Ever> = serde_json::from_str(config_json)?;
    let dom: Node5Ever = Node::parse(svg.unwrap_or(
        r#"<svg xmlns="http://www.w3.org/2000/svg">
    test
</svg>"#,
    ))?;
    jobs.run(&dom);
    node_to_string(&dom)
}

#[cfg(test)]
pub(crate) fn node_to_string(node: &impl oxvg_ast::node::Node) -> anyhow::Result<String> {
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
