use rand::{seq::SliceRandom, thread_rng};

/// Details about OXVG as a project
pub struct About<'a> {
    /// The current version of the executable
    pub version: &'a str,
    /// Contributors to the OXVG project
    // NOTE: To contributors, make a PR to add yourself
    pub authors: Vec<&'a str>,
    /// Licensing information
    pub license: &'a str,
}

impl Default for About<'_> {
    fn default() -> Self {
        // NOTE: To contributors, make a PR to add yourself
        // TODO: Fetch from github api?
        let mut authors = vec!["Noah <noahwbaldwin@gmail.com>"];
        authors.shuffle(&mut thread_rng());
        Self {
            version: "v0.1.0",
            authors,
            license: "Most oxvg source code is available under the MIT License.\n\nNotable Exceptions are\n- actions based on Inkscape are mostly licensed under the GNU License"
        }
    }
}

#[test]
fn test_about() {
    let about = About::default();
    insta::assert_snapshot!(about.version);
    insta::assert_snapshot!(about.license);
}
