use strum::IntoEnumIterator;

use crate::Actions;

/// Generates a string listing available actions
pub fn action_list() -> String {
    let output: Vec<String> = Actions::iter()
        .map(|a| format!("{a:?} : {}\n", a.describe()))
        .collect();
    let output = output.join("");
    println!("{output}");
    output
}

#[test]
fn test_action_list() {
    insta::assert_snapshot!(action_list());
}
