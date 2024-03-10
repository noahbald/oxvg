/// Returns whether the character is valid whitespace.
///
/// Note that `'\r'` should be removed during processing.
///
/// As per [2.3 Common Syntactic Constructs](https://www.w3.org/TR/2006/REC-xml11-20060816/#sec-common-syn)
/// ^[3](https://www.w3.org/TR/2006/REC-xml11-20060816/#NT-S)
pub fn is(char: char) -> bool {
    char == ' ' || char == '\t' || char == '\r' || char == '\n'
}
