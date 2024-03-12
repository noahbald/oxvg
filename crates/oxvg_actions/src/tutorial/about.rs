use rand::{seq::SliceRandom, thread_rng};

pub struct About<'a> {
    pub version: &'a str,
    pub authors: Vec<&'a str>,
    pub license: &'a str,
}

pub fn about<'a>() -> About<'a> {
    let mut about = About::default();
    about.authors.shuffle(&mut thread_rng());
    about
}

impl Default for About<'_> {
    fn default() -> Self {
        Self {
            version: "v0.1.0",
            authors: vec!["Noah <noahwbaldwin@gmail.com>"],
            license: "Most oxvg source code is available under the MIT License.\n\nNotable Exceptions are\n- actions based on Inkscape are mostly licensed under the GNU License"
        }
    }
}

#[test]
fn test_about() {
    let about = about();
    insta::assert_snapshot!(about.version);
    insta::assert_snapshot!(about.license);
}
