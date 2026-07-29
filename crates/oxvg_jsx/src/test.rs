use std::{collections::HashMap, io::Write, marker::PhantomData};

use pretty_assertions::assert_eq;

use crate::{
    config::{
        ExpandProps, ExportType, Icon, JSXRuntime, JsxRuntimeImport, Template, VariablesString,
    },
    error::TemplateError,
    transform, Config,
};

fn test(input: &str, expected: &str) {
    test_config(&Config::default(), input, expected);
}
fn test_config(config: &Config, input: &str, expected: &str) {
    test_template(config, None, input, expected);
}
fn test_template(
    config: &Config,
    template: Option<&Template<Vec<u8>>>,
    input: &str,
    expected: &str,
) {
    let code = swc_core::common::GLOBALS.set(&swc_core::common::Globals::new(), || {
        oxvg_ast::parse::roxmltree::parse(input, |root, allocator| {
            let mut buf = Vec::new();
            transform(
                root,
                allocator,
                Some(Config {
                    oxvg: Some(false),
                    ..config.clone()
                }),
                None,
                template.cloned(),
                &mut buf,
            )
            .unwrap();
            String::from_utf8(buf).unwrap()
        })
        .unwrap()
    });
    assert_eq!(code, expected);
}

#[test]
fn test_default() {
    test(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
  <rect x="25" y="36" width="48" height="1" aria-label="Test" class="rect"></rect>
</svg>"#,
        r#"import * as React from "react";
const SvgComponent = (props)=><svg viewBox="0 0 100 100" {...props}><rect x={25} y={36} width={48} height={1} aria-label="Test" className="rect"/></svg>;
export default SvgComponent;
"#,
    );

    test(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg">
  <rect style="fill: rgb(255, 0, 0); stroke: yellow; font-family: Helvetica; --foo-bar: test"></rect>
</svg>
"#,
        r##"import * as React from "react";
const SvgComponent = (props)=><svg {...props}><rect style={{
        fill: "red",
        stroke: "#ff0",
        fontFamily: "Helvetica",
        "--foo-bar": "test"
    }}/></svg>;
export default SvgComponent;
"##,
    );
}

#[test]
fn with_title_overwrite_existing() {
    test_config(
        &Config {
            title_prop: Some(true),
            expand_props: Some(ExpandProps::None),
            ..Default::default()
        },
        r#"<svg aria-labelledby="title"><title id="title">test</title><g /></svg>"#,
        r#"import * as React from "react";
const SvgComponent = ({ title, titleId })=><svg aria-labelledby={titleId}>{title === undefined ? <title id={titleId || "title"}>test</title> : <title id={titleId || "title"}>{title}</title>}<g/></svg>;
export default SvgComponent;
"#,
    );
}

#[test]
fn with_desc_overwrite_existing() {
    test_config(
        &Config {
            desc_prop: Some(true),
            expand_props: Some(ExpandProps::None),
            ..Default::default()
        },
        r#"<svg aria-describedby="desc"><desc id="desc">test</desc><g /></svg>"#,
        r#"import * as React from "react";
const SvgComponent = ({ desc, descId })=><svg aria-describedby={descId}>{desc === undefined ? <desc id={descId || "desc"}>test</desc> : <desc id={descId || "desc"}>{desc}</desc>}<g/></svg>;
export default SvgComponent;
"#,
    );
}

#[test]
fn with_title_and_desc_overwrite_existing() {
    test_config(
        &Config {
            title_prop: Some(true),
            desc_prop: Some(true),
            expand_props: Some(ExpandProps::None),
            ..Default::default()
        },
        r#"<svg aria-labelledby="title" aria-describedby="desc"><title id="title">t</title><desc id="desc">d</desc><g /></svg>"#,
        r#"import * as React from "react";
const SvgComponent = ({ title, titleId, desc, descId })=><svg aria-labelledby={titleId} aria-describedby={descId}>{title === undefined ? <title id={titleId || "title"}>t</title> : <title id={titleId || "title"}>{title}</title>}{desc === undefined ? <desc id={descId || "desc"}>d</desc> : <desc id={descId || "desc"}>{desc}</desc>}<g/></svg>;
export default SvgComponent;
"#,
    );
}

#[test]
fn with_dimensions_false() {
    test_config(
        &Config {
            dimensions: Some(false),
            ..Default::default()
        },
        r#"<svg width="100" height="100" viewBox="0 0 100 100"><g /></svg>"#,
        r#"import * as React from "react";
const SvgComponent = (props)=><svg viewBox="0 0 100 100" {...props}><g/></svg>;
export default SvgComponent;
"#,
    );
}

#[test]
fn with_icon_bool() {
    test_config(
        &Config {
            icon: Some(Icon::Bool(true)),
            ..Default::default()
        },
        "<svg><g /></svg>",
        r#"import * as React from "react";
const SvgComponent = (props)=><svg width="1em" height="1em" {...props}><g/></svg>;
export default SvgComponent;
"#,
    );
}

#[test]
fn with_icon_bool_native() {
    test_config(
        &Config {
            icon: Some(Icon::Bool(true)),
            native: Some(true),
            ..Default::default()
        },
        "<svg><g /></svg>",
        r#"import * as React from "react";
import Svg, { G } from "react-native-svg";
const SvgComponent = (props)=><Svg width={24} height={24} {...props}><G/></Svg>;
export default SvgComponent;
"#,
    );
}

#[test]
fn with_icon_string() {
    test_config(
        &Config {
            icon: Some(Icon::String("2em".into())),
            ..Default::default()
        },
        "<svg><g /></svg>",
        r#"import * as React from "react";
const SvgComponent = (props)=><svg width="2em" height="2em" {...props}><g/></svg>;
export default SvgComponent;
"#,
    );
}

#[test]
fn with_icon_number() {
    test_config(
        &Config {
            icon: Some(Icon::Number(24.0)),
            ..Default::default()
        },
        "<svg><g /></svg>",
        r#"import * as React from "react";
const SvgComponent = (props)=><svg width={24} height={24} {...props}><g/></svg>;
export default SvgComponent;
"#,
    );
}

#[test]
fn with_icon_and_expand_props_and_dimensions() {
    test_config(
        &Config {
            icon: Some(Icon::Bool(true)),
            expand_props: Some(ExpandProps::End),
            dimensions: Some(true),
            ..Default::default()
        },
        r##"<svg a="#000" b="#fff"/>"##,
        r##"import * as React from "react";
const SvgComponent = (props)=><svg a="#000" b="#fff" width="1em" height="1em" {...props}/>;
export default SvgComponent;
"##,
    );
}

#[test]
fn with_svg_props_new_attribute() {
    let mut svg_props = HashMap::new();
    svg_props.insert("data-testid".to_string(), "icon".to_string());

    test_config(
        &Config {
            svg_props: Some(svg_props),
            ..Default::default()
        },
        "<svg><g /></svg>",
        r#"import * as React from "react";
const SvgComponent = (props)=><svg data-testid="icon" {...props}><g/></svg>;
export default SvgComponent;
"#,
    );
}

#[test]
fn with_svg_props_override_existing() {
    let mut svg_props = HashMap::new();
    svg_props.insert("viewBox".to_string(), "0 0 24 24".to_string());

    test_config(
        &Config {
            svg_props: Some(svg_props),
            ..Default::default()
        },
        r#"<svg viewBox="0 0 100 100"><g /></svg>"#,
        r#"import * as React from "react";
const SvgComponent = (props)=><svg viewBox="0 0 24 24" {...props}><g/></svg>;
export default SvgComponent;
"#,
    );
}

#[test]
fn with_svg_props_expression_value() {
    let mut svg_props = HashMap::new();
    svg_props.insert("tabIndex".to_string(), "{tabIndex}".to_string());

    test_config(
        &Config {
            svg_props: Some(svg_props),
            ..Default::default()
        },
        "<svg><g /></svg>",
        r#"import * as React from "react";
const SvgComponent = (props)=><svg tabIndex={tabIndex} {...props}><g/></svg>;
export default SvgComponent;
"#,
    );
}

#[test]
fn with_replace_attr_values() {
    let mut replace_attr_values = HashMap::new();
    replace_attr_values.insert("#000".to_string(), "currentColor".to_string());

    test_config(
        &Config {
            replace_attr_values: Some(replace_attr_values),
            ..Default::default()
        },
        r##"<svg><rect fill="#000" /><path stroke="#000" /></svg>"##,
        r#"import * as React from "react";
const SvgComponent = (props)=><svg {...props}><rect fill="currentColor"/><path stroke="currentColor"/></svg>;
export default SvgComponent;
"#,
    );
}

#[test]
fn with_expand_props_start() {
    test_config(
        &Config {
            expand_props: Some(ExpandProps::Start),
            ..Default::default()
        },
        r#"<svg viewBox="0 0 10 10"><g /></svg>"#,
        r#"import * as React from "react";
const SvgComponent = (props)=><svg {...props} viewBox="0 0 10 10"><g/></svg>;
export default SvgComponent;
"#,
    );
}

#[test]
fn jsx_runtime_classic_preact() {
    test_config(
        &Config {
            jsx_runtime: Some(JSXRuntime::ClassicPreact),
            ..Default::default()
        },
        "<svg><g /></svg>",
        r#"import { h } from "preact/compat";
const SvgComponent = (props)=><svg {...props}><g/></svg>;
export default SvgComponent;
"#,
    );
}

#[test]
fn with_native_and_title_prop() {
    test_config(
        &Config {
            native: Some(true),
            title_prop: Some(true),
            expand_props: Some(ExpandProps::None),
            ..Default::default()
        },
        "<svg><g /></svg>",
        r#"import * as React from "react";
import Svg, { G, Title } from "react-native-svg";
const SvgComponent = ({ title, titleId })=><Svg aria-labelledby={titleId}>{title ? <Title id={titleId}>{title}</Title> : null}<G/></Svg>;
export default SvgComponent;
"#,
    );
}

#[test]
fn with_native_ref_memo_and_expand_props() {
    test_config(
        &Config {
            native: Some(true),
            r#ref: Some(true),
            memo: Some(true),
            expand_props: Some(ExpandProps::End),
            ..Default::default()
        },
        "<svg><g /></svg>",
        r#"import * as React from "react";
import Svg, { G } from "react-native-svg";
import { forwardRef, memo } from "react";
const SvgComponent = (props, ref)=><Svg ref={ref} {...props}><G/></Svg>;
const ForwardRef = forwardRef(SvgComponent);
const Memo = memo(ForwardRef);
export default Memo;
"#,
    );
}

#[test]
fn with_typescript_default_template() {
    test_config(
        &Config {
            typescript: Some(true),
            expand_props: Some(ExpandProps::End),
            title_prop: Some(true),
            ..Default::default()
        },
        "<svg><g /></svg>",
        r#"import * as React from "react";
import { SVGProps } from "react";
interface SvgComponentProps {
    title?: string;
    titleId?: string;
}
const SvgComponent = ({ title, titleId, ...props }: SVGProps<SVGSVGElement> & SvgComponentProps)=><svg aria-labelledby={titleId} {...props}>{title ? <title id={titleId}>{title}</title> : null}<g/></svg>;
export default SvgComponent;
"#,
    );
}

#[test]
fn with_typescript_native_default_template() {
    test_config(
        &Config {
            typescript: Some(true),
            native: Some(true),
            expand_props: Some(ExpandProps::End),
            title_prop: Some(true),
            ..Default::default()
        },
        "<svg><g /></svg>",
        r#"import * as React from "react";
import Svg, { SVGProps, G, Title } from "react-native-svg";
interface SvgComponentProps {
    title?: string;
    titleId?: string;
}
const SvgComponent = ({ title, titleId, ...props }: SVGProps<SVGSVGElement> & SvgComponentProps)=><Svg aria-labelledby={titleId} {...props}>{title ? <Title id={titleId}>{title}</Title> : null}<G/></Svg>;
export default SvgComponent;
"#,
    );
}

#[test]
fn with_export_type_named_no_named_export() {
    test_config(
        &Config {
            export_type: Some(ExportType::Named),
            ..Default::default()
        },
        "<svg><g /></svg>",
        r#"import * as React from "react";
const SvgComponent = (props)=><svg {...props}><g/></svg>;
export { SvgComponent };
"#,
    );
}

#[test]
fn with_expand_props_and_svg_props() {
    let mut svg_props = HashMap::new();
    svg_props.insert("className".to_string(), "icon".to_string());

    test_config(
        &Config {
            expand_props: Some(ExpandProps::End),
            svg_props: Some(svg_props),
            ..Default::default()
        },
        "<svg><g /></svg>",
        r#"import * as React from "react";
const SvgComponent = (props)=><svg className="icon" {...props}><g/></svg>;
export default SvgComponent;
"#,
    );
}

// From here on out, tests are derived from https://github.com/gregberge/svgr/blob/v8.1.0/packages/babel-plugin-transform-svg-component/src/__snapshots__/index.test.ts.snap in order.
#[test]
fn jsx_runtime_classic_default_specifier() {
    test_config(
        &Config {
            jsx_runtime: Some(JSXRuntime::Classic),
            jsx_runtime_import: Some(JsxRuntimeImport {
                source_namespace: None,
                source: "hyperapp-jsx-pragma".into(),
                specifiers: None,
                namespace: None,
                default_specifier: Some("h".into()),
            }),
            ..Default::default()
        },
        "<svg><g /></svg>",
        r#"import h from "hyperapp-jsx-pragma";
const SvgComponent = (props)=><svg {...props}><g/></svg>;
export default SvgComponent;
"#,
    );
}

#[test]
fn jsx_runtime_classic_namespace() {
    test_config(
        &Config {
            jsx_runtime: Some(JSXRuntime::Classic),
            jsx_runtime_import: Some(JsxRuntimeImport {
                source_namespace: None,
                source: "preact".into(),
                specifiers: None,
                namespace: Some("Preact".into()),
                default_specifier: None,
            }),
            ..Default::default()
        },
        "<svg><g /></svg>",
        r#"import * as Preact from "preact";
const SvgComponent = (props)=><svg {...props}><g/></svg>;
export default SvgComponent;
"#,
    );
}

#[test]
fn jsx_runtime_classic_specifier() {
    test_config(
        &Config {
            jsx_runtime: Some(JSXRuntime::Classic),
            jsx_runtime_import: Some(JsxRuntimeImport {
                source_namespace: None,
                source: "preact".into(),
                specifiers: Some(vec!["h".into()]),
                namespace: None,
                default_specifier: None,
            }),
            ..Default::default()
        },
        "<svg><g /></svg>",
        r#"import { h } from "preact";
const SvgComponent = (props)=><svg {...props}><g/></svg>;
export default SvgComponent;
"#,
    );
}

#[test]
fn jsx_runtime_automatic() {
    test_config(
        &Config {
            jsx_runtime: Some(JSXRuntime::Automatic),
            ..Default::default()
        },
        "<svg><g /></svg>",
        r"const SvgComponent = (props)=><svg {...props}><g/></svg>;
export default SvgComponent;
",
    );
}

#[test]
fn jsx_runtime_classic() {
    test_config(
        &Config {
            jsx_runtime: Some(JSXRuntime::Classic),
            ..Default::default()
        },
        "<svg><g /></svg>",
        r#"import * as React from "react";
const SvgComponent = (props)=><svg {...props}><g/></svg>;
export default SvgComponent;
"#,
    );
}

#[test]
fn specify_import_source() {
    test_config(
        &Config {
            memo: Some(true),
            r#ref: Some(true),
            // NOTE: SVGR doesn't expose this, we do in `source_namespace` field.
            // import_source: "preact/compat",
            jsx_runtime_import: Some(JsxRuntimeImport {
                source_namespace: Some("preact/compat".into()),
                source: "preact".into(),
                specifiers: Some(vec!["h".into()]),
                namespace: None,
                default_specifier: None,
            }),
            ..Default::default()
        },
        "<svg><g /></svg>",
        r#"import { forwardRef, memo } from "preact/compat";
import { h } from "preact";
const SvgComponent = (props, ref)=><svg ref={ref} {...props}><g/></svg>;
const ForwardRef = forwardRef(SvgComponent);
const Memo = memo(ForwardRef);
export default Memo;
"#,
    );
}

#[test]
fn template_basic() {
    test_template(
        &Config::default(),
        Some(&Template::String(
            |w, VariablesString { jsx, .. }, _| -> Result<_, _> {
                write!(
                    w,
                    r#"import * as React from "react";
const MyComponent = () => {jsx}
export default MyComponent
"#
                )
                .map_err(|_| TemplateError)
            },
            PhantomData,
        )),
        "<svg><g /></svg>",
        r#"import * as React from "react";
const MyComponent = () => <svg {...props}><g/></svg>
export default MyComponent
"#,
    );
}

#[test]
fn template_jsx() {
    test_template(
        &Config::default(),
        Some(&Template::String(
            |w, VariablesString { jsx, .. }, _| {
                write!(
                    w,
                    r#"import * as React from "react";
const MyComponent = () => <main>{{{jsx}}}</main>
export default MyComponent
"#
                )
                .map_err(|_| TemplateError)
            },
            PhantomData,
        )),
        "<svg><g /></svg>",
        r#"import * as React from "react";
const MyComponent = () => <main>{<svg {...props}><g/></svg>}</main>
export default MyComponent
"#,
    );
}

#[test]
fn template_typescript() {
    test_template(
        &Config::default(),
        Some(&Template::String(
            |w, VariablesString { jsx, .. }, _| {
                write!(
                    w,
                    r#"import * as React from "react";
const MyComponent = (props: React.SVGProps<SVGSVGElement>) => {jsx}
export default MyComponent
"#
                )
                .map_err(|_| TemplateError)
            },
            PhantomData,
        )),
        "<svg><g /></svg>",
        r#"import * as React from "react";
const MyComponent = (props: React.SVGProps<SVGSVGElement>) => <svg {...props}><g/></svg>
export default MyComponent
"#,
    );
}

#[test]
fn template_comment() {
    test_template(
        &Config::default(),
        Some(&Template::String(
            |w, VariablesString { jsx, .. }, _| {
                write!(
                    w,
                    r"/**
 * Comment
 */
const MyComponent = () => {jsx}
export default MyComponent
"
                )
                .map_err(|_| TemplateError)
            },
            PhantomData,
        )),
        "<svg><g /></svg>",
        r"/**
 * Comment
 */
const MyComponent = () => <svg {...props}><g/></svg>
export default MyComponent
",
    );
}

// NOTE: "template that does not return an array" skipped here —
// depends on Babel's `tmpl`, which will not be implemented.

#[test]
fn template_type_annotation() {
    test_template(
        &Config {
            typescript: Some(true),
            expand_props: Some(ExpandProps::None),
            ..Config::default()
        },
        Some(&Template::String(
            |w,
             VariablesString {
                 jsx,
                 imports,
                 interfaces,
                 component_name,
                 exports,
                 ..
             },
             _| {
                write!(
                    w,
                    r"
{imports}
{interfaces}
interface Props {{ x?: string }}
export const {component_name}: React.FC<Props> = ({{ x }}) => {{
  return ({jsx});
}}
{exports}"
                )
                .map_err(|_| TemplateError)
            },
            PhantomData,
        )),
        "<svg><g /></svg>",
        r#"
import * as React from "react";


interface Props { x?: string }
export const SvgComponent: React.FC<Props> = ({ x }) => {
  return (<svg><g/></svg>);
}
export default SvgComponent;
"#,
    );
}

#[test]
fn transforms_whole_program() {
    test(
        "<svg><g /></svg>",
        r#"import * as React from "react";
const SvgComponent = (props)=><svg {...props}><g/></svg>;
export default SvgComponent;
"#,
    );
}

#[test]
fn with_desc_prop() {
    test_config(
        &Config {
            desc_prop: Some(true),
            expand_props: Some(ExpandProps::None),
            ..Default::default()
        },
        "<svg><g /></svg>",
        r#"import * as React from "react";
const SvgComponent = ({ desc, descId })=><svg aria-describedby={descId}>{desc ? <desc id={descId}>{desc}</desc> : null}<g/></svg>;
export default SvgComponent;
"#,
    );
}

#[test]
fn with_desc_prop_and_expand_props() {
    test_config(
        &Config {
            desc_prop: Some(true),
            expand_props: Some(ExpandProps::End),
            ..Default::default()
        },
        "<svg><g /></svg>",
        r#"import * as React from "react";
const SvgComponent = ({ desc, descId, ...props })=><svg aria-describedby={descId} {...props}>{desc ? <desc id={descId}>{desc}</desc> : null}<g/></svg>;
export default SvgComponent;
"#,
    );
}

#[test]
fn with_expand_props() {
    test_config(
        &Config {
            expand_props: Some(ExpandProps::End),
            ..Default::default()
        },
        "<svg><g /></svg>",
        r#"import * as React from "react";
const SvgComponent = (props)=><svg {...props}><g/></svg>;
export default SvgComponent;
"#,
    );
}

#[test]
fn with_memo_option() {
    test_config(
        &Config {
            memo: Some(true),
            ..Default::default()
        },
        "<svg><g /></svg>",
        r#"import * as React from "react";
import { memo } from "react";
const SvgComponent = (props)=><svg {...props}><g/></svg>;
const Memo = memo(SvgComponent);
export default Memo;
"#,
    );
}

#[test]
fn with_named_export_and_export_type_option() {
    test_config(
        &Config {
            export_type: Some(ExportType::Named),
            named_export: Some("ReactComponent".into()),
            ..Default::default()
        },
        "<svg><g /></svg>",
        r#"import * as React from "react";
const SvgComponent = (props)=><svg {...props}><g/></svg>;
export { SvgComponent as ReactComponent };
"#,
    );
}

// NOTE: "with namedExport option and previousExport state" skipped here —
// depends on Babel's `state.caller.previousExport`, which will not be implemented.

#[test]
fn with_native_and_expand_props_option() {
    test_config(
        &Config {
            native: Some(true),
            expand_props: Some(ExpandProps::End),
            ..Default::default()
        },
        "<svg><g /></svg>",
        r#"import * as React from "react";
import Svg, { G } from "react-native-svg";
const SvgComponent = (props)=><Svg {...props}><G/></Svg>;
export default SvgComponent;
"#,
    );
}

#[test]
fn with_native_option() {
    test_config(
        &Config {
            native: Some(true),
            ..Default::default()
        },
        "<svg><g /></svg>",
        r#"import * as React from "react";
import Svg, { G } from "react-native-svg";
const SvgComponent = (props)=><Svg {...props}><G/></Svg>;
export default SvgComponent;
"#,
    );
}

#[test]
fn with_native_ref_and_expand_props_option() {
    test_config(
        &Config {
            native: Some(true),
            r#ref: Some(true),
            expand_props: Some(ExpandProps::End),
            ..Default::default()
        },
        "<svg><g /></svg>",
        r#"import * as React from "react";
import Svg, { G } from "react-native-svg";
import { forwardRef } from "react";
const SvgComponent = (props, ref)=><Svg ref={ref} {...props}><G/></Svg>;
const ForwardRef = forwardRef(SvgComponent);
export default ForwardRef;
"#,
    );
}

#[test]
fn with_native_and_ref_option() {
    test_config(
        &Config {
            native: Some(true),
            r#ref: Some(true),
            expand_props: Some(ExpandProps::None),
            ..Default::default()
        },
        "<svg><g /></svg>",
        r#"import * as React from "react";
import Svg, { G } from "react-native-svg";
import { forwardRef } from "react";
const SvgComponent = (_, ref)=><Svg ref={ref}><G/></Svg>;
const ForwardRef = forwardRef(SvgComponent);
export default ForwardRef;
"#,
    );
}

#[test]
fn with_ref_and_expand_props() {
    test_config(
        &Config {
            r#ref: Some(true),
            expand_props: Some(ExpandProps::End),
            ..Default::default()
        },
        "<svg><g /></svg>",
        r#"import * as React from "react";
import { forwardRef } from "react";
const SvgComponent = (props, ref)=><svg ref={ref} {...props}><g/></svg>;
const ForwardRef = forwardRef(SvgComponent);
export default ForwardRef;
"#,
    );
}

#[test]
fn with_ref_option() {
    test_config(
        &Config {
            r#ref: Some(true),
            expand_props: Some(ExpandProps::None),
            ..Default::default()
        },
        "<svg><g /></svg>",
        r#"import * as React from "react";
import { forwardRef } from "react";
const SvgComponent = (_, ref)=><svg ref={ref}><g/></svg>;
const ForwardRef = forwardRef(SvgComponent);
export default ForwardRef;
"#,
    );
}

#[test]
fn with_title_prop_desc_prop_and_expand_props() {
    test_config(
        &Config {
            title_prop: Some(true),
            desc_prop: Some(true),
            expand_props: Some(ExpandProps::End),
            ..Default::default()
        },
        "<svg><g /></svg>",
        r#"import * as React from "react";
const SvgComponent = ({ title, titleId, desc, descId, ...props })=><svg aria-labelledby={titleId} aria-describedby={descId} {...props}>{title ? <title id={titleId}>{title}</title> : null}{desc ? <desc id={descId}>{desc}</desc> : null}<g/></svg>;
export default SvgComponent;
"#,
    );
}

#[test]
fn with_title_prop() {
    test_config(
        &Config {
            title_prop: Some(true),
            expand_props: Some(ExpandProps::None),
            ..Default::default()
        },
        "<svg><g /></svg>",
        r#"import * as React from "react";
const SvgComponent = ({ title, titleId })=><svg aria-labelledby={titleId}>{title ? <title id={titleId}>{title}</title> : null}<g/></svg>;
export default SvgComponent;
"#,
    );
}

#[test]
fn with_title_prop_and_desc_prop() {
    test_config(
        &Config {
            title_prop: Some(true),
            desc_prop: Some(true),
            expand_props: Some(ExpandProps::None),
            ..Default::default()
        },
        "<svg><g /></svg>",
        r#"import * as React from "react";
const SvgComponent = ({ title, titleId, desc, descId })=><svg aria-labelledby={titleId} aria-describedby={descId}>{title ? <title id={titleId}>{title}</title> : null}{desc ? <desc id={descId}>{desc}</desc> : null}<g/></svg>;
export default SvgComponent;
"#,
    );
}

#[test]
fn with_title_prop_and_expand_props() {
    test_config(
        &Config {
            title_prop: Some(true),
            expand_props: Some(ExpandProps::End),
            ..Default::default()
        },
        "<svg><g /></svg>",
        r#"import * as React from "react";
const SvgComponent = ({ title, titleId, ...props })=><svg aria-labelledby={titleId} {...props}>{title ? <title id={titleId}>{title}</title> : null}<g/></svg>;
export default SvgComponent;
"#,
    );
}

#[test]
fn with_both_memo_and_ref_option() {
    test_config(
        &Config {
            memo: Some(true),
            r#ref: Some(true),
            expand_props: Some(ExpandProps::None),
            ..Default::default()
        },
        "<svg><g /></svg>",
        r#"import * as React from "react";
import { forwardRef, memo } from "react";
const SvgComponent = (_, ref)=><svg ref={ref}><g/></svg>;
const ForwardRef = forwardRef(SvgComponent);
const Memo = memo(ForwardRef);
export default Memo;
"#,
    );
}
