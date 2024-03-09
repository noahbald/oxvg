// [2.4 Character Data](https://www.w3.org/TR/2006/REC-xml11-20060816/#syntax)

use crate::{diagnostics::SVGError, file_reader::FileReader};

pub fn char_data(file_reader: &mut FileReader) -> Result<String, Box<SVGError>> {
    // [14]
    let mut text: String = "".into();
    while let Some(&char) = file_reader.peek() {
        match char {
            '&' => break,
            '<' => break,
            _ => {}
        }
        text.push(char);
        file_reader.next();
    }

    Ok(text)
}
