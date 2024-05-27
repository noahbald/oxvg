mod add_attributes_to_svg_element;
mod add_classes_to_svg;
mod cleanup_attributes;
mod cleanup_enable_background;
mod cleanup_ids;

use std::rc::Rc;

use serde::Deserialize;

pub use self::add_attributes_to_svg_element::AddAttributesToSVGElement;
pub use self::add_classes_to_svg::AddClassesToSVG;
pub use self::cleanup_attributes::CleanupAttributes;
pub use self::cleanup_enable_background::CleanupEnableBackground;
pub use self::cleanup_ids::CleanupIds;

pub enum PrepareOutcome {
    None,
    Skip,
}

pub trait Job {
    fn prepare(&mut self, _document: &rcdom::RcDom) -> PrepareOutcome {
        PrepareOutcome::None
    }

    fn run(&self, _node: &Rc<rcdom::Node>) {}

    fn breakdown(&mut self, _document: &rcdom::RcDom) {}
}

#[derive(Deserialize, Default, Clone)]
pub struct Jobs {
    add_attributes_to_svg_element: Option<AddAttributesToSVGElement>,
    add_classes_to_svg: Option<AddClassesToSVG>,
    cleanup_attributes: Option<CleanupAttributes>,
    cleanup_enable_background: Option<CleanupEnableBackground>,
    cleanup_ids: Option<CleanupIds>,
}

impl Jobs {
    pub fn run(self, root: &rcdom::RcDom) {
        let mut jobs: Vec<_> = self.into_iter().flatten().collect();
        jobs.retain_mut(|job| job.prepare(root).can_skip());

        root.document
            .children
            .borrow()
            .iter()
            .for_each(|child| Jobs::crawl(&jobs, child));

        jobs.iter_mut().for_each(|job| job.breakdown(root));
    }

    fn crawl(jobs: &Vec<Box<dyn Job>>, child: &Rc<rcdom::Node>) {
        for job in jobs {
            job.run(child);
        }
        child
            .children
            .borrow()
            .iter()
            .for_each(|child| Jobs::crawl(jobs, child));
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
            jobs.cleanup_attributes
                .map(|job| Box::new(job) as Box<dyn Job>),
            jobs.cleanup_enable_background
                .map(|job| Box::new(job) as Box<dyn Job>),
            jobs.cleanup_ids.map(|job| Box::new(job) as Box<dyn Job>),
        ];
        jobs.into_iter()
    }
}

impl PrepareOutcome {
    fn can_skip(&self) -> bool {
        matches!(self, Self::Skip)
    }
}

#[test]
fn test_jobs() -> Result<(), serde_json::Error> {
    use xml5ever::{
        driver::{parse_document, XmlParseOpts},
        tendril::TendrilSink,
    };

    let dom: rcdom::RcDom =
        parse_document(rcdom::RcDom::default(), XmlParseOpts::default()).one("<svg></svg>");
    let jobs: Jobs = serde_json::from_str(
        r#"{ "add_attributes_to_svg_element": {
            "attributes": { "foo": "bar" }
        } }"#,
    )?;
    jobs.run(&dom);
    Ok(())
}
