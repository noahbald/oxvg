//! HACK: Because we can't store [`StyleAttribute`]/[`StyleSheet`] in ast, these functions
//! will temporarily wrap declarations so that they can be minified.
use lightningcss::{
    declaration::DeclarationBlock,
    error::{Error, MinifyErrorKind},
    rules::CssRuleList,
    stylesheet::{MinifyOptions, ParserOptions, StyleAttribute, StyleSheet},
};

pub(crate) fn style<'i>(style: &mut DeclarationBlock<'i>) {
    let mut stub = StyleAttribute::parse("", ParserOptions::default()).unwrap();
    std::mem::swap(style, &mut stub.declarations);
    stub.minify(MinifyOptions::default());
    std::mem::swap(style, &mut stub.declarations);
}

pub(crate) fn style_list<'i>(style: &mut CssRuleList<'i>) -> Result<(), Error<MinifyErrorKind>> {
    let mut stub = StyleSheet::parse("", ParserOptions::default()).unwrap();
    std::mem::swap(style, &mut stub.rules);
    stub.minify(MinifyOptions::default())?;
    std::mem::swap(style, &mut stub.rules);
    Ok(())
}
