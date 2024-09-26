use std::rc::Rc;

use markup5ever::local_name;
use oxvg_path::{convert, geometry::MakeArcs, Path};
use oxvg_selectors::Element;
use serde::Deserialize;

use crate::{Context, Job};

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
    fn use_style(&self, node: &Rc<rcdom::Node>) -> bool {
        let element = Element::new(node.clone());
        element.get_attr(&local_name!("d")).is_some()
    }

    fn run(&self, node: &Rc<rcdom::Node>, context: &Context) {
        let element = Element::new(node.clone());
        let Some(d) = element.get_attr(&local_name!("d")) else {
            return;
        };

        let style_info = convert::StyleInfo::gather(
            &context.style.computed(),
            element.get_attr(&local_name!("marker-start")).is_some()
                || element.get_attr(&local_name!("marker-end")).is_some(),
        );
        dbg!("ConvertPathData::run: gained style info", &style_info);

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

    insta::assert_snapshot!(test_config(
        r#"{ "convertPathData": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <path d="M 10,50 L 20,30"/>
    <path d="M 10,50 C 20,30 40,50 60,70"/>
    <path d="M 10,50 C 20,30 40,50 60,70 S 20,30 30,60"/>
    <path d="M 10,50 Q 30,60 30,70"/>
    <path d="M 10,50 Q 30,60 30,70 T 40,70"/>
    <path d="M 10,50 A 20,60 45 0,1 40,70"/>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "convertPathData": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <path d="M 10,50 M 20,60"/>
    <path d="M 10,50 20,60"/>
    <path d="M 10,50 L 20,30 L 40,60"/>
    <path d="M 10,50 L 20,30 40,60"/>
    <path d="M 10,50 C 20,30 40,50 60,70 C 40,40 50,60 70,80"/>
    <path d="M 10,50 C 20,30 40,50 60,70 40,40 50,60 70,80"/>
    <path d="M 10,50 C 20,30 40,50 60,70 S 30,30 40,50 S 60,70 80,100"/>
    <path d="M 10,50 C 20,30 40,50 60,70 S 30,30 40,50 60,70 80,100"/>
    <path d="M 10,50 Q 30,60 30,70 Q 40,70 50,90"/>
    <path d="M 10,50 Q 30,60 30,70 40,70 50,90"/>
    <path d="M 10,50 Q 30,60 30,70 T 40,70 T 50,90"/>
    <path d="M 10,50 Q 30,60 30,70 T 40,70 50,90"/>
    <path d="M 10,50 A 20,60 45 0,1 40,70 A 30,50 -30 1,1 50,70"/>
    <path d="M 10,50 A 20,60 45 0,1 40,70 30,50 -30 1,1 50,70"/>
    <style>
      .marker-mid { marker-mid: url(#); }
    </style>
    <path d="M0,0 0,5 0,10" class="marker-mid"/>
    <path d="M0,0 0,5 0,10" marker-mid="url(#)"/>
    <style>
      .linecap-round { stroke: black; stroke-linecap: round; }
      .linecap-butt { stroke: black; stroke-linecap: butt; }
    </style>
    <path d="M0,0 0,0" stroke="black" stroke-linecap="round"/>
    <path d="M0,0 0,0" class="linecap-round"/>
    <path d="M0,0 0,0" stroke="black" stroke-linecap="butt"/>
    <path d="M0,0 0,0" class="linecap-butt"/>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "convertPathData": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <path d="M 10,50 l 20,30 L 20,30"/>
    <path d="M 10,50 c 20,30 40,50 60,70 C 20,30 40,50 60,70"/>
    <path d="M 10,50 c 20,30 40,50 60,70 s 20,40 40,50 L 10,20"/>
    <path d="M 10,50 q 20,60 30,70 Q 20,60 30,70"/>
    <path d="M 10,50 q 20,60 30,70 t 40,70 L 10,20"/>
    <path d="M 10,50 a 20,60 45 0,1 40,70 A 20,60 45 0,1 40,70"/>
    <path d="M1 1 v8 c0-2 0-4 0-6"/>
</svg>"#
        )
    )?);

    // TODO: Rest of tests to be added in next commit

    Ok(())
}