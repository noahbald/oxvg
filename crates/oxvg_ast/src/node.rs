use std::{cell::RefCell, collections::HashMap, rc::Rc};

#[derive(Default, Debug)]
pub struct Root {
    pub children: Vec<Rc<RefCell<Child>>>,
}

#[derive(Default, Debug)]
pub struct Element {
    pub name: String,
    pub attributes: HashMap<String, String>,
    pub attributes_order: Vec<String>,
    pub children: Vec<Rc<RefCell<Child>>>,
    pub parent: Parent,
    pub is_self_closing: bool,
}

#[derive(Debug)]
pub enum Child {
    SGMLDeclaration { value: String },
    Doctype { data: String },
    Instruction { name: String, value: String },
    Comment { value: String },
    CData { value: String },
    Text { value: String },
    Element(Element),
}

#[derive(Debug)]
pub enum Parent {
    Root(Rc<RefCell<Root>>),
    Element(Rc<RefCell<Element>>),
}

impl Parent {
    pub fn push_child(&mut self, child: Child) {
        let child = Rc::new(RefCell::new(child));
        self.push_rc(&child);
    }

    pub fn push_rc(&mut self, child: &Rc<RefCell<Child>>) {
        match self {
            Self::Root(r) => {
                r.borrow_mut().children.push(Rc::clone(child));
            }
            Self::Element(e) => {
                e.borrow_mut().children.push(Rc::clone(child));
            }
        }
    }

    pub fn is_root(&self) -> bool {
        matches!(self, Self::Root(_))
    }
}

impl Default for Parent {
    fn default() -> Self {
        Parent::Root(Rc::new(RefCell::new(Root::default())))
    }
}
