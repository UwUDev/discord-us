pub mod crypt;
pub mod key;
pub mod container;

use std::ops::{Range, Sub};
use serde::{
    Serialize,
    Deserialize,
};
use crate::{
    fs::{
        SerializedFsNode,
        FsNode,
        Ref,
        IntoTree,
        dir::{
            DirEntryNode,
        },
    },
    pack::{
        container::{
            Container,
        },
    },
};


pub trait Size<A> {
    fn get_size(&self) -> A;
}

impl<A: Sub<Output=A> + Copy> Size<A> for Range<A> {
    fn get_size(&self) -> A {
        self.end - self.start
    }
}

/// A waterfall is a format containing metadata needed
/// to download a file tree from discord's server.
#[derive(Serialize, Deserialize)]
pub struct SerializableWaterfall {
    fs_root: SerializedFsNode<DirEntryNode>,
    container: Vec<Container>,
}

#[derive(Clone, Debug)]
pub struct Waterfall {
    pub fs_root: Ref<FsNode<DirEntryNode>>,
    pub containers: Vec<Container>,
}

impl Waterfall {
    pub fn new(fs_root: Ref<FsNode<DirEntryNode>>, containers: Vec<Container>) -> Self {
        Self { fs_root, containers }
    }

    pub fn as_serializable(&self) -> SerializableWaterfall {
        SerializableWaterfall {
            fs_root: (*self.fs_root.borrow()).clone().into(),
            container: self.containers.clone(),
        }
    }

    pub fn from_serializable(serializable: SerializableWaterfall) -> Self {
        Self {
            fs_root: serializable.fs_root.into_node(None),
            containers: serializable.container,
        }
    }
}