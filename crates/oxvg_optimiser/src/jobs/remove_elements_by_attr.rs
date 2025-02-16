use itertools::Itertools;
use oxvg_ast::{
    class_list::ClassList,
    element::Element,
    visitor::{Context, Visitor},
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct RemoveElementsByAttr {
    #[serde(default = "Vec::new")]
    pub id: Vec<String>,
    #[serde(default = "Vec::new")]
    pub class: Vec<String>,
}

impl Default for RemoveElementsByAttr {
    fn default() -> Self {
        RemoveElementsByAttr {
            id: Vec::new(),
            class: Vec::new(),
        }
    }
}

impl<E: Element> Visitor<E> for RemoveElementsByAttr {
    type Error = String;

    fn element(&mut self, element: &mut E, _context: &mut Context<E>) -> Result<(), String> {
        if !self.id.is_empty() {
            if let Some(id) = element.get_attribute_local(&"id".into()) {
                let id: &str = id.as_ref();
                if self.id.iter().map(String::as_str).contains(id) {
                    element.remove();
                    return Ok(());
                }
            }
        }

        if !self.class.is_empty() {
            for class in element.class_list().values() {
                let class: &str = class.as_ref();
                if self.class.iter().map(String::as_str).contains(class) {
                    element.remove();
                    return Ok(());
                }
            }
        }

        Ok(())
    }
}

#[test]
fn remove_elements_by_attr() -> anyhow::Result<()> {
    use crate::test_config;

    insta::assert_snapshot!(test_config(
        r#"{ "removeElementsByAttr": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="19" height="19" viewBox="0 0 19 19">
    <rect id="someID" width="19" height="19"/>
    <path id="close" d="M1093.5,31.792l-0.72.721-8.27-8.286-8.28,8.286-0.72-.721,8.28-8.286-8.28-8.286,0.72-.721,8.28,8.286,8.27-8.286,0.72,0.721-8.27,8.286Z" transform="translate(-1075 -14)"/>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeElementsByAttr": { "id": ["someID"] } }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="19" height="19" viewBox="0 0 19 19">
    <rect id="someID" width="19" height="19"/>
    <path id="close" d="M1093.5,31.792l-0.72.721-8.27-8.286-8.28,8.286-0.72-.721,8.28-8.286-8.28-8.286,0.72-.721,8.28,8.286,8.27-8.286,0.72,0.721-8.27,8.286Z" transform="translate(-1075 -14)"/>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeElementsByAttr": { "id": ["someID", "anotherID"] } }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="19" height="19" viewBox="0 0 19 19">
    <rect id="someID" width="19" height="19"/>
    <path id="anotherID" d="M1093.5,31.792l-0.72.721-8.27-8.286-8.28,8.286-0.72-.721,8.28-8.286-8.28-8.286,0.72-.721,8.28,8.286,8.27-8.286,0.72,0.721-8.27,8.286Z" transform="translate(-1075 -14)"/>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeElementsByAttr": { "class": ["someClass"] } }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="19" height="19" viewBox="0 0 19 19">
    <rect class="someClass" width="19" height="19"/>
    <path class="close" d="M1093.5,31.792l-0.72.721-8.27-8.286-8.28,8.286-0.72-.721,8.28-8.286-8.28-8.286,0.72-.721,8.28,8.286,8.27-8.286,0.72,0.721-8.27,8.286Z" transform="translate(-1075 -14)"/>
    <rect class="someClass extraClass"/>
    <rect class="SOMEclass case-sensitive"/>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeElementsByAttr": { "class": ["someClass", "anotherClass"] } }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="19" height="19" viewBox="0 0 19 19">
    <rect class="someClass" width="19" height="19"/>
    <path class="anotherClass" d="M1093.5,31.792l-0.72.721-8.27-8.286-8.28,8.286-0.72-.721,8.28-8.286-8.28-8.286,0.72-.721,8.28,8.286,8.27-8.286,0.72,0.721-8.27,8.286Z" transform="translate(-1075 -14)"/>
    <rect class="someClass extraClass"/>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeElementsByAttr": { "id": ["someID"], "class": ["someClass"] } }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="19" height="19" viewBox="0 0 19 19">
    <rect class="someClass" width="19" height="19"/>
    <path class="someClass extraClass" d="M1093.5,31.792l-0.72.721-8.27-8.286-8.28,8.286-0.72-.721,8.28-8.286-8.28-8.286,0.72-.721,8.28,8.286,8.27-8.286,0.72,0.721-8.27,8.286Z" transform="translate(-1075 -14)"/>
    <rect class="anotherClass"/>
    <path id="someID" class="anotherID"/>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeElementsByAttr": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="19" height="19" viewBox="0 0 19 19">
    <rect class="some-class" width="19" height="19"/>
</svg>"#
        ),
    )?);

    Ok(())
}
