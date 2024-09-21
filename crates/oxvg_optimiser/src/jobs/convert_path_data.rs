use std::rc::Rc;

use markup5ever::local_name;
use oxvg_path::{convert, geometry::MakeArcs, Path};
use oxvg_selectors::Element;
use serde::Deserialize;

use crate::Job;

#[derive(Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ConvertPathData {
    remove_useless: Option<bool>,
    smart_arc_rounding: Option<bool>,
    straight_curves: Option<bool>,
    convert_to_q: Option<bool>,
    line_shorthands: Option<bool>,
    collapse_repeated: Option<bool>,
    curve_smooth_shorthands: Option<bool>,
    convert_to_z: Option<bool>,
    force_absolute_path: Option<bool>,
    negative_extra_space: Option<bool>,
    make_arcs: Option<MakeArcs>,
    float_precision: Option<i32>,
    utilize_absolute: Option<bool>,
    // TODO: Do we want to have apply_transforms as an option, or is it better to have this plugin
    // just *before* this one
    // apply_transforms: Option<bool>,
    // apply_transforms_stroked: Option<bool>,
    // transform_precision: Option<usize>,
}

impl Job for ConvertPathData {
    fn run(&self, node: &Rc<rcdom::Node>) {
        let element = Element::new(node.clone());
        let Some(d) = element.get_attr(&local_name!("d")) else {
            return;
        };

        let style = element
            .get_attr(&local_name!("style"))
            .map(|attr| attr.value)
            .unwrap_or_default();
        let style_info = convert::StyleInfo::gather(
            &style,
            element.get_attr(&local_name!("marker-start")).is_some()
                || element.get_attr(&local_name!("marker-end")).is_some(),
        );

        let path = match Path::parse(&d.value) {
            Ok(path) => path,
            Err(e) => {
                dbg!("ConvertPathData::run: failed to parse path", e);
                return;
            }
        };
        if path.0.is_empty() {
            return;
        }

        let path = convert::run(
            &path,
            &convert::Options {
                flags: self.into(),
                make_arcs: self.make_arcs.clone(),
                precision: self.float_precision.unwrap_or(*DEFAULT_FLOAT_PRECISION),
            },
            &style_info,
        );

        element.set_attr(&local_name!("d"), String::from(path).into());
    }
}

impl From<&ConvertPathData> for convert::Flags {
    fn from(val: &ConvertPathData) -> Self {
        use convert::Flags;

        let mut output = convert::Flags::default();
        if let Some(f) = val.remove_useless {
            output.set(Flags::remove_useless_flag, f);
        }
        if let Some(f) = val.smart_arc_rounding {
            output.set(Flags::smart_arc_rounding_flag, f);
        }
        if let Some(f) = val.straight_curves {
            output.set(Flags::straight_curves_flag, f);
        }
        if let Some(f) = val.convert_to_q {
            output.set(Flags::convert_to_q_flag, f);
        }
        if let Some(f) = val.line_shorthands {
            output.set(Flags::line_shorthands_flag, f);
        }
        if let Some(f) = val.collapse_repeated {
            output.set(Flags::collapse_repeated_flag, f);
        }
        if let Some(f) = val.curve_smooth_shorthands {
            output.set(Flags::curve_smooth_shorthands_flag, f);
        }
        if let Some(f) = val.convert_to_z {
            output.set(Flags::convert_to_z_flag, f);
        }
        if let Some(f) = val.force_absolute_path {
            output.set(Flags::force_absolute_path_flag, f);
        }
        if let Some(f) = val.negative_extra_space {
            output.set(Flags::negative_extra_space_flag, f);
        }
        if let Some(f) = val.utilize_absolute {
            output.set(Flags::utilize_absolute_flag, f);
        }
        output
    }
}

lazy_static! {
    static ref DEFAULT_MAKE_ARCS: MakeArcs = MakeArcs {
        threshold: 2.5,
        tolerance: 0.5,
    };
    static ref DEFAULT_FLOAT_PRECISION: i32 = 3;
}

#[test]
fn convert_path_data() -> anyhow::Result<()> {
    use crate::test_config;

    insta::assert_snapshot!(test_config(
        r#"{ "convertPathData": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- Optimise move commands -->
    <path d="M 10,50"/>
    <path d="M 10 50"/>
    <path d="M10 50"/>
    <path d="M10,50"/>
    <path d="M10-3.05176e-005"/>
    <path d="M10-50.2.30-2"/>
    <path d="M10-50l.2.30"/>
    <path d="M 10 , 50"/>
    <path d="M -10,-50"/>
    <path d="M -10 -50"/>
    <path d="M-10 -50"/>
    <path d="M-10-50"/>
    <path d="M-10,-50"/>
    <path d="M -10 , -50"/>
    <path d="..."/>
</svg>"#
        )
    )?);

    // TODO: Rest of tests to be added in next commit

    Ok(())
}
