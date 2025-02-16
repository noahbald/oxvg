use oxvg_ast::{
    element::Element,
    visitor::{Context, ContextFlags, PrepareOutcome, Visitor},
};
use oxvg_path::{convert, geometry::MakeArcs, Path};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::struct_excessive_bools)]
pub struct ConvertPathData {
    #[serde(default = "flag_default_true")]
    remove_useless: bool,
    #[serde(default = "flag_default_true")]
    smart_arc_rounding: bool,
    #[serde(default = "flag_default_true")]
    straight_curves: bool,
    #[serde(default = "flag_default_true")]
    convert_to_q: bool,
    #[serde(default = "flag_default_true")]
    line_shorthands: bool,
    #[serde(default = "flag_default_true")]
    collapse_repeated: bool,
    #[serde(default = "flag_default_true")]
    curve_smooth_shorthands: bool,
    #[serde(default = "flag_default_true")]
    convert_to_z: bool,
    #[serde(default = "bool::default")]
    force_absolute_path: bool,
    #[serde(default = "flag_default_true")]
    negative_extra_space: bool,
    #[serde(default = "MakeArcs::default")]
    make_arcs: MakeArcs,
    #[serde(default = "Precision::default")]
    float_precision: Precision,
    #[serde(default = "flag_default_true")]
    utilize_absolute: bool,
    // TODO: Do we want to have apply_transforms as an option, or is it better to have this as a plugin
    // just *before* this one
    // apply_transforms: Option<bool>,
    // apply_transforms_stroked: Option<bool>,
    // transform_precision: Option<usize>,
}

impl Default for ConvertPathData {
    fn default() -> Self {
        ConvertPathData {
            remove_useless: flag_default_true(),
            smart_arc_rounding: flag_default_true(),
            straight_curves: flag_default_true(),
            convert_to_q: flag_default_true(),
            line_shorthands: flag_default_true(),
            collapse_repeated: flag_default_true(),
            curve_smooth_shorthands: flag_default_true(),
            convert_to_z: flag_default_true(),
            force_absolute_path: bool::default(),
            negative_extra_space: flag_default_true(),
            make_arcs: MakeArcs::default(),
            float_precision: Precision::default(),
            utilize_absolute: flag_default_true(),
        }
    }
}

#[derive(Clone, Default, Copy, Debug)]
pub struct Precision(pub oxvg_path::convert::Precision);

impl<E: Element> Visitor<E> for ConvertPathData {
    type Error = String;

    fn prepare(&mut self, _document: &E, _context_flags: &mut ContextFlags) -> PrepareOutcome {
        PrepareOutcome::use_style
    }

    fn use_style(&mut self, element: &E) -> bool {
        let d_name = "d".into();
        element.has_attribute_local(&d_name)
    }

    fn element(&mut self, element: &mut E, context: &mut Context<'_, '_, E>) -> Result<(), String> {
        let d_localname = "d".into();
        let Some(d) = element.get_attribute_local(&d_localname) else {
            return Ok(());
        };

        let style_info = convert::StyleInfo::gather(&context.computed_styles);
        log::debug!("ConvertPathData::run: gained style info {style_info:?}");

        let path = match Path::parse(d.as_ref()) {
            Ok(path) => path,
            Err(e) => {
                log::error!("failed to parse path: {e}\n{}", d.as_ref());
                return Ok(());
            }
        };
        drop(d);
        if path.0.is_empty() {
            return Ok(());
        }

        let path = convert::run(
            &path,
            &convert::Options {
                flags: self.into(),
                make_arcs: self.make_arcs.clone(),
                precision: self.float_precision.0,
            },
            &style_info,
        );

        element.set_attribute_local(d_localname, String::from(path).into());
        Ok(())
    }
}

impl From<&mut ConvertPathData> for convert::Flags {
    fn from(val: &mut ConvertPathData) -> Self {
        use convert::Flags;

        let mut output = convert::Flags::default();
        output.set(Flags::remove_useless_flag, val.remove_useless);
        output.set(Flags::smart_arc_rounding_flag, val.smart_arc_rounding);
        output.set(Flags::straight_curves_flag, val.straight_curves);
        output.set(Flags::convert_to_q_flag, val.convert_to_q);
        output.set(Flags::line_shorthands_flag, val.line_shorthands);
        output.set(Flags::collapse_repeated_flag, val.collapse_repeated);
        output.set(
            Flags::curve_smooth_shorthands_flag,
            val.curve_smooth_shorthands,
        );
        output.set(Flags::convert_to_z_flag, val.convert_to_z);
        output.set(Flags::force_absolute_path_flag, val.force_absolute_path);
        output.set(Flags::negative_extra_space_flag, val.negative_extra_space);
        output.set(Flags::utilize_absolute_flag, val.utilize_absolute);
        output
    }
}

const fn flag_default_true() -> bool {
    true
}

#[derive(Debug)]
enum DeserializePrecisionError {
    OutOfRange,
    InvalidType,
}

impl std::fmt::Display for DeserializePrecisionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OutOfRange => f.write_str("number out of range for i32"),
            Self::InvalidType => f.write_str("expected null, i32, or false"),
        }
    }
}

impl serde::de::StdError for DeserializePrecisionError {}

impl<'de> Deserialize<'de> for Precision {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        match value {
            Value::Null => Ok(Self(oxvg_path::convert::Precision::None)),
            Value::Number(x) => match x.as_i64() {
                Some(x) => Ok(Self(oxvg_path::convert::Precision::Enabled(
                    x.try_into().map_err(|_| {
                        serde::de::Error::custom(DeserializePrecisionError::OutOfRange)
                    })?,
                ))),
                None => Err(serde::de::Error::custom(
                    DeserializePrecisionError::OutOfRange,
                )),
            },
            Value::Bool(x) if !x => Ok(Self(oxvg_path::convert::Precision::Disabled)),
            _ => Err(serde::de::Error::custom(
                DeserializePrecisionError::InvalidType,
            )),
        }
    }
}

impl Serialize for Precision {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self.0 {
            oxvg_path::convert::Precision::None => Value::Null.serialize(serializer),
            oxvg_path::convert::Precision::Disabled => false.serialize(serializer),
            oxvg_path::convert::Precision::Enabled(n) => n.serialize(serializer),
        }
    }
}

#[test]
#[allow(clippy::too_many_lines)]
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

    insta::assert_snapshot!(test_config(
        r#"{ "convertPathData": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <path d="M 10.3467,50.09 L 10.0000,50.20"/>
    <path d="m 10 10 l 1 1 M 20 20"/>
    <path d="m 0 0 l .1133 1 l .1133 2 l .1133 3"/>
    <path d="m 0 0 l .0025 3 .0025 2 .0025 3 .0025 2"/>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "convertPathData": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <path d="M 10,50 L 10,50"/>
    <path d="M 10,50 L 20,50"/>
    <path d="M 10,50 L 10,60"/>
    <path d="M 10,50 L 20,30 10,30"/>
    <path d="M 10,50 L 20,30 20,20"/>
    <path d="M 10,50 L 20,30 10,30 40,50"/>
    <path d="M 10,50 L 20,30 20,20 40,50"/>
    <path d="M 10,50 L 20,50 L 30,50"/>
    <path d="M 10,50 L 20,50 30,50"/>
    <path d="M 10,50 L 20,50 L 30,50 L 40,50"/>
    <path d="M 10,50 L 10,60 L 10,70"/>
    <path d="M 10,50 L 10,60 10,70"/>
    <path d="M 10,50 L 10,60 L 10,70 L 10,80"/>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "convertPathData": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <path d="m 0,0"/>
    <path d="m 0,0l 0,0"/>
    <path d="m 0,0h 0"/>
    <path d="m 0,0v 0"/>
    <path d="m 0,0c 0,0 0,0 0,0 s 0,0 0,0"/>
    <path d="m 0,0q 0,0 0,0 t 0,0"/>
    <path d="m 0,0a 25,25 -30 0,1 0,0"/>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "convertPathData": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <path d="M100,200 C200,200 300,200 400,200"/>
    <path d="M100,200 C100,200 250,200 250,200 S300,200 400,200"/>
    <path d="M100,200 C100,200 250,200 250,200 S300,300 400,210"/>
    <path d="M100,200 S250,250 250,250 S400,250 500,250"/>
    <path d="M100,200 Q200,200 300,200"/>
    <path d="M100,200 Q400,200 600,200 T800,200"/>
    <path d="M100,200 Q400,200 600,200 T800,300"/>
    <path d="M100,200 Q200,200 200,300 T200,500 T300,500"/>
    <path d="M100,200 Q400,200 600,200 T800,200 T900,300"/>
    <path d="M100,200 T800,300"/>
    <path d="M100,200 A0,150 0 0,0 150,150"/>
    <path d="M100,200 A150,0 0 0,0 150,150"/>
    <path d="M100,200 c-2.5 10.5-4 21-4 32 0 64 63.5 128 127.5 128s32.5 0 96.5 0 128-64 128-128-64-128-128-128"/>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "convertPathData": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <path d="M100,200 C100,100 450,100 250,200 C50,300 400,300 400,200"/>
    <path d="M100,200 S250,100 250,200 C250,300 300,250 400,200"/>
    <path d="M100,200 C100,200 250,100 250,200"/>
    <path d="M200,300 Q400,50 600,300 Q 800,550 1000,300"/>
    <path d="M200,300 Q400,50 600,300 T1000,300 Q1200,50 1400,300"/>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "convertPathData": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <path d="m100,200 300,400 z m100,200 L 300,400"/>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "convertPathData": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <path d="M10 50h30h-30"/>
    <path d="M10 50h-30h30"/>
    <path d="M10 50h-30h-50"/>
    <path d="M10 50h30h50"/>
    <path d="M10 50v30v-30"/>
    <path d="M10 50v-30v30"/>
    <path d="M10 50v-30v-50"/>
    <path d="M10 50v30v50"/>
    <path d="M10 50L10 80L10 0"/>
    <path d="M10 50L10 10L10 80"/>
    <path d="M10 50l10 10l20 20l10 10"/>
    <path d="M10 50L80 50L0 50"/>
    <path d="M10 50L0 50L80 50"/>
    <path d="M10 50L0 50M80 50M30 10L10 80"/>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "convertPathData": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <path d="M213 543q0 -41 20 -66.5q20 -25.5 50 -45.5l49 228q-54 -4 -86.5 -34q-32.5 -30 -32.5 -82zt0 0zM371 48z" />
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "convertPathData": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <path d="M0 0L0 0c2.761 0 5 2.239 5 5"/>
    <path d="M0 0L0 0c2.761 0 5 2.239 5 5l5-5"/>
    <path d="M15 10c-2.761 0-5-2.239-5-5s2.239-5 5-5s5 2.239 5 5l-5 5"/>
    <path d="M41.008 0.102c1.891 0.387 3.393 1.841 3.849 3.705"/>
    <path d="M7.234 19.474C6.562 19.811 5.803 20 5 20c-2.761 0-5-2.239-5-5 0-1.767 0.917-3.32 2.301-4.209"/>
    <path d="M60 0c-2.761 0-5 2.239-5 5s2.239 5 5 5s5-2.239 5-5S62.761 0 60 0z"/>
    <path d="M15 23.54 c-2.017,0 -3.87,-.7 -5.33,-1.87 -.032,-.023 -.068,-.052 -.11,-.087 .042,.035 .078,.064 .11,.087 .048,.04 .08,.063 .08,.063 "/>
    <path d="M-9.5,82.311c-2.657,0-4.81-2.152-4.81-4.811c0-2.656,2.153-4.811,4.81-4.811S-4.69,74.844-4.69,77.5 C-4.69,80.158-6.843,82.311-9.5,82.311z"/>
    <path d="M1.5,13.4561 C1.5,15.3411 3.033,16.8751 4.918,16.8751 C6.478,16.8751 7.84,15.8201 8.229,14.3101 Z"/>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "convertPathData": { "floatPrecision": 2 } }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <path d="M.49 8.8c-.3-.75-.44-1.55-.44-2.35 0-3.54 2.88-6.43 6.43-6.43 3.53 0 6.42 2.88 6.42 6.43 0 .8-.15 1.6-.43 2.35"/>
    <path d="M13.4 6.62c0-2.5-1.98-4.57-4.4-4.57S4.6 4.1 4.6 6.62"/>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "convertPathData": { "floatPrecision": 0 } }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <path d="M.49 8.8c-.3-.75-.44-1.55-.44-2.35 0-3.54 2.88-6.43 6.43-6.43 3.53 0 6.42 2.88 6.42 6.43 0 .8-.15 1.6-.43 2.35"/>
    <path d="M13.4 6.62c0-2.5-1.98-4.57-4.4-4.57S4.6 4.1 4.6 6.62"/>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "convertPathData": { "floatPrecision": 8 } }"#,
        Some(
            r#"<svg width="100" height="100" viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink">
    <path d="M33.027833,1.96545901 C33.097408,2.03503401 38.0413624,6.97898843 38.0413624,6.97898842 C38.0413625,6.97898834 38.0094318,4.0346712 38.0094318,4.0346712 L34,0.0252395624 L34,0 L13,0 L13,2 L33.062374,2 Z"></path>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "convertPathData": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 36 36">
    <path d="M32 4a4 4 0 00-4-4H8a4 4 0 01-4 4v28a4 4 0 014 4h20a4 4 0 004-4V4z"/>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "convertPathData": {} }"#,
        Some(
            r#"<svg viewBox="0 0 1200 400" xmlns="http://www.w3.org/2000/svg">
    <path d="M300 200 h-150 a150 150 0 1 0 150 -150 z" fill="red" stroke="blue" stroke-width="5" />
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "convertPathData": {} }"#,
        Some(
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 48 32">
  <path d="M 6 12 v -6 h 6 a 3 3 0 0 1 -6 6 z" />
  <path d="M 18 12 v -6 h 6 a 3 3 0 0 1 -6 6 z" stroke="#f00" stroke-width="4" />
  <path d="M 30 12 v -6 h 6 a 3 3 0 0 1 -6 6 z" stroke="#f00" stroke-width="4" stroke-linejoin="round" stroke-linecap="round" />
</svg>"##
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "convertPathData": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="48" height="48">
    <path d="M6 32.845 6 14.766 6 32.845"/>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "convertPathData": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24">
  <path d="M1 1m1 1"/>
  <path fill="black" d="M8.5 12Zm0 8q3.35 0 5.675-2.325"/>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "convertPathData": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="48" height="48">
    <path d="M 6 6 h 0.1 h 0.2"/>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "convertPathData": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="48" height="48">
    <path d="M 6 6 h 0.0005"/>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "convertPathData": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10">
    <path d="m 1 1 a 10000 10000 0 0 0 8 0" stroke="black"/>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "convertPathData": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10">
    <path d="m 1 1 a 10.567 10.567 0 0 0 1 0" stroke="black"/>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "convertPathData": {} }"#,
        Some(
            r#"<svg viewBox="0 0 20 20">
    <path d="M0 0q2 0 5 5t5 5q5 0 5 5"/>
    <path d="M0 0q2 0 5 5t5 5q2 0 5-2"/>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "convertPathData": {} }"#,
        Some(
            r#"<svg>
    <path d="m 0 12 C 4 4 8 4 12 12"/>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "convertPathData": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1 20">
  <path d="M-6.3 9.9q.7-4.5.2-5-.5-.5-1.5-.5l0 0q-.4 0-2 .3"/>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "convertPathData": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 9 9">
    <marker id="a" stroke="red" viewBox="0 0 5 5">
        <circle cx="2" cy="2" r="1"/>
    </marker>
    <marker id="b" stroke="green" viewBox="0 0 5 5">
        <circle cx="2" cy="2" r="0.5"/>
    </marker>
    <path marker-start="url(#a)" d="M5 5h0"/>
    <path marker-start="url(#b)" d="M5 5"/>
</svg>"#
        )
    )?);

    Ok(())
}
