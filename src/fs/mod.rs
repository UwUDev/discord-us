pub mod dir;
pub mod file;

use std::{
    io::{Read, Seek},
};
use crate::Size;

trait FsReadable: Read + Seek + Size + Send + Clone {}