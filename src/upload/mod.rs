use std::{
    io::{Read, Error}
};

trait Uploader<T, R: Read> {
    fn do_upload(&self, reader: R, size: u64) -> Result<T, Error>;
}