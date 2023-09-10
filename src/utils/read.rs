use std::{io::{Read}, cmp::{min}, alloc, slice};
use std::alloc::Layout;
use crate::Size;

pub trait LazyRead<T: Read> {
    fn open(&self) -> T;
}

/// Represent a Stream I/O Using chunked operation
pub trait Chunked {
    fn process_next_chunk(&mut self) -> Option<Vec<u8>>;
}

/// Allows to stream a Chunked object
pub struct ChunkedRead<T: Chunked> {
    buf: Vec<u8>,
    buf_position: usize,

    chunked: T,
}

impl<T: Chunked> From<T> for ChunkedRead<T> {
    fn from(chunked: T) -> Self {
        Self {
            buf: Vec::new(),
            buf_position: 0,

            chunked,
        }
    }
}

impl<T: Chunked> Read for ChunkedRead<T> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let mut read = 0usize;

        while read < buf.len() {
            if self.buf_position < self.buf.len() {
                let to_read = min(buf.len() - read, self.buf.len() - self.buf_position);

                buf[read..read + to_read].copy_from_slice(&self.buf[self.buf_position..self.buf_position + to_read]);

                self.buf_position += to_read;

                read += to_read;
            } else {
                if let Some(chunk) = self.chunked.process_next_chunk() {
                    self.buf = chunk;
                    self.buf_position = 0;
                } else {
                    break;
                }
            }
        }

        Ok(read)
    }
}

/// Skip a certain amount of bytes before and after reading
struct OmitStream<T: Read> {
    read: u64,

    reader: T,

    omit_before: u64,
    omit_after: u64,
}

impl<T: Read> OmitStream<T> {
    pub fn from(reader: T, omit_before: u64, omit_after: u64) -> Self {
        Self {
            read: 0,

            reader,

            omit_before,
            omit_after,
        }
    }
}

impl<T: Read> Read for OmitStream<T> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {

        // omit before
        while self.read < self.omit_before {
            let to_omit = min((self.omit_before - self.read) as usize, buf.len());

            let read = self.reader.read(&mut buf[..to_omit])?; // read into void

            self.read += read as u64;
        }

        let mut read = self.reader.read(buf)?;

        self.read += read as u64;

        // omit after
        if self.read > self.omit_after {
            let to_omit = min((self.read - self.omit_after) as usize, buf.len());
            read -= to_omit;
            self.read -= to_omit as u64;
        }

        Ok(read)
    }
}

#[cfg(test)]
mod test {
    use crate::utils::read::{
        Chunked, ChunkedRead,
    };
    use std::io::Read;

    struct TestChunked {
        current_chunk: usize,
    }

    impl Chunked for TestChunked {
        fn process_next_chunk(&mut self) -> Option<Vec<u8>> {
            if self.current_chunk < 10 {
                self.current_chunk += 1;
                Some(vec![0u8; 10])
            } else {
                None
            }
        }
    }

    #[test]
    pub fn test() {
        let mut reader = ChunkedRead::from(TestChunked {
            current_chunk: 0,
        });

        let mut buf = [0u8; 49];

        reader.read(&mut buf).unwrap();

        assert_eq!(buf, [0u8; 49]);

        let mut buf = [0u8; 26];

        reader.read(&mut buf).unwrap();

        assert_eq!(buf, [0u8; 26]);

        let mut buf = [0u8; 25];

        reader.read(&mut buf).unwrap();

        assert_eq!(buf, [0u8; 25]);
    }
}