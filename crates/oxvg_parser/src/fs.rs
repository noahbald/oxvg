use std::path::PathBuf;

use rcdom::SerializableHandle;
use xml5ever::serialize::{serialize, SerializeOpts};

pub fn load_files(paths: &[PathBuf]) -> Vec<(PathBuf, Vec<u8>)> {
    paths.iter().flat_map(load_file).collect()
}

fn load_file(path: &PathBuf) -> Box<dyn Iterator<Item = (PathBuf, Vec<u8>)>> {
    let metadata = std::fs::metadata(path).unwrap();
    if metadata.is_symlink() {
        return load_file(&std::fs::read_link(path).unwrap());
    };
    if metadata.is_file() {
        return Box::new(vec![(path.clone(), std::fs::read(path).unwrap())].into_iter());
    }
    Box::new(
        std::fs::read_dir(path)
            .unwrap()
            .map(|dir| dir.unwrap().path())
            .filter(|path| path.ends_with(".svg"))
            .map(|path| (path.clone(), std::fs::read(path.clone()).unwrap())),
    )
}

pub fn write_file(path: &Option<PathBuf>, source: &PathBuf, dom: &rcdom::RcDom) {
    let document: SerializableHandle = dom.document.clone().into();
    let Some(path) = path else {
        serialize(&mut std::io::stdout(), &document, SerializeOpts::default()).unwrap();
        return;
    };

    let metadata = std::fs::metadata(path).ok();
    if metadata.clone().is_some_and(|data| data.is_symlink()) {
        return write_file(&Some(path.clone()), source, dom);
    };

    let mut sink = if metadata.is_some_and(|data| data.is_dir()) {
        let path = path.join(source.file_name().unwrap());
        std::fs::File::create(path).unwrap()
    } else {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::File::create(path).unwrap()
    };
    serialize(&mut sink, &document, SerializeOpts::default()).unwrap();
}
