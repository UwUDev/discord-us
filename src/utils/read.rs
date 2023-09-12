use std::{
    io::{Read},
    cmp::{min},
    ops::Range,
    marker::PhantomData,
};
use std::cmp::max;
use crate::{Size, ZeroSubstract};
use crate::utils::range::{Intersect, Ranged, RangedSort};

/// Lazy Read is a Read that is only opened when needed
pub trait LazyOpen<T> {
    fn open(&self) -> T;
}

/// Range Lazy Read is a readable interface that allow to open a Lazy Read stream with a specific range
pub trait RangeLazyOpen<T>: LazyOpen<T> {
    fn open_with_range(&self, range: Range<u64>) -> T;
}

// impl<T: Read, R: RangeLazyOpen<T> + Size> LazyOpen<T> for R {
//     fn open(&self) -> T {
//         self.open_with_range(Range { start: 0, end: self.get_size() })
//     }
// }

/// Represent a Stream I/O Using chunked operation
pub trait Chunked {
    fn process_next_chunk(&mut self) -> Option<Vec<u8>>;
}

pub trait ChunkSize {
    fn get_chunk_size(&self) -> u64;
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
                    return Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "No more chunks"));
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

        let to_read = min((self.omit_after - self.read) as usize, buf.len());

        if to_read <= 0 {
            return Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "No more bytes to read"));
        }

        let mut read = self.reader.read(&mut buf[..to_read])?;

        self.read += read as u64;

        Ok(read)
    }
}

pub struct ChunkedOmitStream<T: LazyOpen<R> + Size + ChunkSize, R: Chunked> {
    lazy_open: T,

    _phantom: PhantomData<R>,
}


impl<T: RangeLazyOpen<R> + Size + ChunkSize, R: Chunked> LazyOpen<OmitStream<ChunkedRead<R>>> for ChunkedOmitStream<T, R> {
    fn open(&self) -> OmitStream<ChunkedRead<R>> {
        let size = self.lazy_open.get_size();
        OmitStream::from(self.lazy_open.open().into(), 0, size)
    }
}

impl<T: RangeLazyOpen<R> + Size + ChunkSize, R: Chunked> RangeLazyOpen<OmitStream<ChunkedRead<R>>> for ChunkedOmitStream<T, R> {
    fn open_with_range(&self, range: Range<u64>) -> OmitStream<ChunkedRead<R>> {
        let chunk_size = self.lazy_open.get_chunk_size();

        let chunk_start = range.start / chunk_size;

        let chunk_end = (range.end + chunk_size - 1) / chunk_size;

        let start = chunk_start * chunk_size;
        let end = chunk_end * chunk_size;

        let stream = ChunkedRead::from(self.lazy_open.open_with_range(start..end));

        #[cfg(test)]
        println!("ChunkedOmitStream::open_with_range : opening stream from {} to {} (real={}->{})", start, end, range.start, range.end);

        OmitStream::from(stream, range.start - start, range.end - start)
    }
}

#[derive(Clone)]
pub struct MultiChunkedStream<R: RangeLazyOpen<C> + Ranged + Clone + ChunkSize, C: Chunked> {
    chunk_readers: Vec<R>,

    _phantom: PhantomData<C>,
}

impl<R: RangeLazyOpen<C> + Ranged + Clone + ChunkSize, C: Chunked> From<Vec<R>> for MultiChunkedStream<R, C> {
    fn from(value: Vec<R>) -> Self {
        Self {
            chunk_readers: value,
            _phantom: PhantomData,
        }
    }
}


impl<R: RangeLazyOpen<C> + Ranged + Clone + ChunkSize, C: Chunked> Size for MultiChunkedStream<R, C> {
    fn get_size(&self) -> u64 {
        self.chunk_readers.iter().map(|r| r.get_size()).sum()
    }
}

impl<R: RangeLazyOpen<C> + Ranged + Clone + ChunkSize, C: Chunked> LazyOpen<MultiChunkedReader<R, C>> for MultiChunkedStream<R, C> {
    fn open(&self) -> MultiChunkedReader<R, C> {
        let size = self.get_size();
        self.open_with_range(Range { start: 0, end: size })
    }
}

impl<R: RangeLazyOpen<C> + Ranged + Clone + ChunkSize, C: Chunked> RangeLazyOpen<MultiChunkedReader<R, C>> for MultiChunkedStream<R, C> {
    fn open_with_range(&self, range: Range<u64>) -> MultiChunkedReader<R, C> {
        let mut sorted_chunk_readers = self.chunk_readers.clone();

        sorted_chunk_readers.retain_mut(|r| r.get_range().is_intersecting(&range));
        sorted_chunk_readers.sort_ranges();

        MultiChunkedReader {
            sorted_chunk_readers,
            range: range,
            cursor: 0,

            current_stream: Default::default(),

            chunk_readers_offset: 0,

            _phantom: PhantomData,
        }
    }
}

pub struct MultiChunkedReader<R: RangeLazyOpen<C> + Ranged + Clone + ChunkSize, C: Chunked> {
    sorted_chunk_readers: Vec<R>,

    cursor: u64,

    range: Range<u64>,

    current_stream: Option<(Range<u64>, OmitStream<ChunkedRead<C>>)>,

    chunk_readers_offset: usize,

    _phantom: PhantomData<C>,
}

impl<R: RangeLazyOpen<C> + Ranged + Clone + ChunkSize, C: Chunked> MultiChunkedReader<R, C> {
    fn find_next_reader(&mut self) -> Option<(R, Range<u64>)> {
        for i in self.chunk_readers_offset..self.sorted_chunk_readers.len() {
            let reader = &self.sorted_chunk_readers[i];

            let range = reader.get_range();

            if range.contains(&self.cursor) {
                self.chunk_readers_offset = i + 1;
                return Some((reader.clone(), range.clone()));
            }
        }
        None
    }
}

impl<R: RangeLazyOpen<C> + Ranged + Clone + ChunkSize, C: Chunked> Read for MultiChunkedReader<R, C> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let mut read = 0;

        while read < buf.len() {
            if let Some((range, stream)) = &mut self.current_stream {
                if self.cursor < range.end {
                    let r = stream.read(&mut buf[read..])?; // -> copy into new buffer
                    self.cursor += r as u64;
                    read += r;
                    continue;
                }
            }

            let (next_reader, range) = self.find_next_reader().ok_or(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "No more readers"))?;

            let relative_start = self.range.start.zero_substract(range.start);
            let relative_end = range.end.zero_substract(self.range.end);

            let omit_stream = ChunkedOmitStream {
                lazy_open: next_reader,
                _phantom: PhantomData,
            };

            #[cfg(test)]
            println!("Find_next_reader : opening stream from {} to {} (real={}->{})", relative_start, range.end - relative_end, range.start, range.end);

            let read = omit_stream.open_with_range(relative_start..(range.end - relative_end));

            self.current_stream = Some((range, read));
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

#[cfg(test)]
mod test2 {
    use std::ops::Range;
    use crate::utils::read::{Chunked, ChunkedOmitStream, ChunkSize, LazyOpen, RangeLazyOpen};
    use crate::Size;
    use std::io::Read;

    struct TestChunked {
        cursor: u8,
        stop: u8,
    }

    impl Chunked for TestChunked {
        fn process_next_chunk(&mut self) -> Option<Vec<u8>> {
            let mut c = self.cursor;
            if c == self.stop {
                return None;
            }

            let chunk = Vec::from(
                [c + 0, c + 1, c + 2, c + 3, c + 4, c + 5, c + 6, c + 7, c + 8, c + 9]
            );

            self.cursor += 10;

            Some(chunk)
        }
    }

    struct B {}

    impl Size for B {
        fn get_size(&self) -> u64 {
            250
        }
    }

    impl ChunkSize for B {
        fn get_chunk_size(&self) -> u64 {
            10
        }
    }

    impl LazyOpen<TestChunked> for B {
        fn open(&self) -> TestChunked {
            TestChunked {
                cursor: 0,
                stop: 250,
            }
        }
    }

    impl RangeLazyOpen<TestChunked> for B {
        fn open_with_range(&self, range: Range<u64>) -> TestChunked {
            println!("open_with_range {:?}", range);
            TestChunked {
                cursor: range.start as u8,
                stop: range.end as u8,
            }
        }
    }

    #[test]
    pub fn test() {
        let r = ChunkedOmitStream {
            lazy_open: B {},
            _phantom: Default::default(),
        };

        let rg = Range {
            start: 69u64,
            end: 215u64,
        };

        let mut reader = r.open_with_range(rg.clone());

        let mut buf = [0u8; 10];

        let mut remaining = rg.end - rg.start;

        while remaining > 0 {
            let r = reader.read(&mut buf).unwrap();

            println!("remaining {} | r {} | buf {:?}", remaining, r, &buf[..r]);

            remaining -= r as u64;
        }
    }
}