pub fn is_root(node: &rcdom::Node) -> bool {
    use rcdom::NodeData::Element;

    let parent_cell = node.parent.replace(None);
    let mut result = true;
    if let Some(parent) = &parent_cell {
        if let Some(parent) = parent.upgrade() {
            if let Element { .. } = parent.data {
                result = false;
            }
        };
        node.parent.replace(parent_cell);
    };
    result
}
