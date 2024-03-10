// [2.3 Common Syntactic Constructs](https://www.w3.org/TR/2006/REC-xml11-20060816/#sec-common-syn)

pub fn is(char: char) -> bool {
    // [3]
    if char == '\r' {
        // println!("Warning, carraige returns should be stripped from document");
    }
    char == ' ' || char == '\t' || char == '\r' || char == '\n'
}
