use std::{
    rc::Rc,
    cell::RefCell,
    collections::HashMap,
    path::{Component, Path, MAIN_SEPARATOR},
    fmt::{Debug, Formatter},
};
use serde::{
    ser::SerializeStruct,
    Serialize,
    Deserialize,
    Deserializer,
    Serializer,
    de::{DeserializeOwned},
};

pub mod dir;

type Ref<T> = Rc<RefCell<T>>;
type OptionalRef<T> = Option<Ref<T>>;

pub struct FsNode<T> {
    data: Option<T>,

    children: HashMap<String, Ref<FsNode<T>>>,
    name: String,

    parent: OptionalRef<FsNode<T>>,
    self_ref: OptionalRef<FsNode<T>>,
}

impl<T: Debug> Debug for FsNode<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "FsNode {{ name: {:?}, data: {:?}, children: {:?} }}", self.name, self.data, self.children)
    }
}

impl<T: Clone> Clone for FsNode<T> {
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
            children: self.children.clone(),
            name: self.name.clone(),

            parent: self.parent.clone(),
            self_ref: self.self_ref.clone(),
        }
    }
}


#[derive(Serialize, Deserialize)]
pub struct SerializedFsNode<T: Serialize + Clone> {
    name: String,
    data: Option<T>,
    children: Vec<SerializedFsNode<T>>,
}

impl<T: Serialize + Clone> Serialize for FsNode<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where
        S: Serializer {
        let serialized_node: SerializedFsNode<T> = self.clone().into();

        serialized_node.serialize(serializer)
    }
}

impl<T: Clone + Serialize> From<FsNode<T>> for SerializedFsNode<T> {
    fn from(value: FsNode<T>) -> Self {
        Self {
            name: value.name,
            data: value.data,
            children: value.children.values()
                .map(|node| node.borrow().clone().into())
                .collect::<Vec<_>>(),
        }
    }
}

impl<T: Clone + Serialize> SerializedFsNode<T> {
    pub fn into_node(&self, parent: OptionalRef<FsNode<T>>) -> Ref<FsNode<T>> {
        let mut node = FsNode::new(&self.name, parent);

        let mut borrowed = node.borrow_mut();

        if let Some(data) = &self.data {
            borrowed.set_data(data.clone());
        }

        for child in self.children.iter() {
            let name = child.name.clone();
            let child = child.into_node(Some(node.clone()));
            borrowed.children.insert(name, child);
        }

        drop(borrowed);

        node
    }
}

impl<T: Clone + Serialize> From<SerializedFsNode<T>> for Ref<FsNode<T>> {
    fn from(value: SerializedFsNode<T>) -> Self {
        value.into_node(None)
    }
}

pub trait IntoTree<T, C> {
    fn into_tree(&self, c: C) -> Ref<FsNode<T>>;
}

impl<T> FsNode<T> {
    fn root() -> Ref<Self> {
        Self::new(&String::from(""), None)
    }

    fn new(name: &String, parent: OptionalRef<FsNode<T>>) -> Ref<Self> {
        let node = Self {
            data: None,
            children: HashMap::new(),
            name: name.clone(),

            parent,
            self_ref: None,
        };

        let rc = Rc::new(RefCell::new(node));

        rc.borrow_mut().self_ref = Some(rc.clone());

        rc
    }

    pub fn get_child_or_create(&mut self, name: String) -> Ref<FsNode<T>> {
        match self.get_child(&name) {
            Some(node) => node,
            None => {
                let node = Self::new(&name, self.get_parent());
                self.children.insert(name, node.clone());
                node
            }
        }
    }

    pub fn get_parent(&self) -> OptionalRef<FsNode<T>> {
        self.parent.clone()
    }

    pub fn get_child(&self, name: &String) -> OptionalRef<FsNode<T>> {
        self.children.get(name).map(|node| node.clone())
    }

    pub fn get_data(&self) -> Option<&T> {
        self.data.as_ref()
    }

    pub fn set_data(&mut self, data: T) {
        self.data = Some(data);
    }

    pub fn find_recursive(&self, path: &Vec<String>) -> OptionalRef<FsNode<T>> {
        match path.len() {
            0 => self.self_ref.clone(),
            _ => {
                let child = self.get_child(&path[0])?;
                let child = child.borrow();
                child.find_recursive(&path[1..].to_vec())
            }
        }
    }

    pub fn find_recursive_create(&mut self, path: &Vec<String>) -> Ref<FsNode<T>> {
        match path.len() {
            0 => self.self_ref.clone().unwrap(),
            _ => {
                let child = self.get_child_or_create(path[0].clone());
                let mut child = child.borrow_mut();
                child.find_recursive_create(&path[1..].to_vec())
            }
        }
    }

    pub fn get_name(&self) -> &String {
        &self.name
    }

    pub fn get_path(&self) -> Vec<String> {
        let mut path = Vec::new();

        while let Some(parent) = self.get_parent() {
            path.push(parent.borrow().get_name().clone());
        }

        path.reverse();

        path
    }
}

pub trait AsPathVec {
    fn as_path_vec(&self) -> Vec<String>;
}

pub trait AsPathRelative {
    fn as_path_relative(&self, path: &Vec<String>) -> Vec<String>;
}

impl<T> AsPathVec for FsNode<T> {
    fn as_path_vec(&self) -> Vec<String> {
        self.get_path()
    }
}

impl AsPathVec for String {
    fn as_path_vec(&self) -> Vec<String> {
        Path::new(self).as_path_vec()
    }
}

impl AsPathVec for Path {
    fn as_path_vec(&self) -> Vec<String> {
        let mut path = Vec::new();

        for component in self.components() {
            match component {
                Component::Normal(name) => {
                    path.push(name.to_string_lossy().to_string());
                }
                Component::RootDir => {
                    path.push("/".to_string());
                }
                Component::ParentDir => {
                    path.pop();
                }
                _ => {}
            }
        }

        path
    }
}

impl AsPathRelative for Vec<String> {
    fn as_path_relative(&self, path: &Vec<String>) -> Vec<String> {
        let mut c = 0;
        for (i, component) in self.iter().enumerate() {
            if i >= path.len() || component != &path[i] {
                break;
            }
            c += 1;
        }

        self[c..].to_vec()
    }
}

impl<T: ?Sized + AsPathVec> AsPathRelative for T {
    fn as_path_relative(&self, path: &Vec<String>) -> Vec<String> {
        self.as_path_vec().as_path_relative(path)
    }
}

pub trait AsPathString {
    fn as_path_string(&self) -> String;
}

impl AsPathString for Vec<String> {
    fn as_path_string(&self) -> String {
        let mut path = String::new();

        for (i, component) in self.iter().enumerate() {
            if i > 0 {
                path.push(MAIN_SEPARATOR);
            }
            if component == "/" {
                continue;
            }
            path.push_str(component);
        }

        path
    }
}