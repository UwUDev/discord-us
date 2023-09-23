use std::{ops::Range, io::Read};
use serde::{Deserialize, Serialize};
use hex_buffer_serde::{Hex as _, HexForm};
use crate::{
    pack::{
        Size,
        key::KeyDerivator,
        crypt::{ChunkCipher, METADATA_SIZE, StreamCipher},
    },
};


/// The metadata of a container
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContainerMeta {
    /// The size of each chunk
    /// => includes padding and metadata for chunks
    chunk_size: u64,

    /// The number of chunks in the container
    chunk_count: u64,

    #[serde(with = "HexForm")]
    salt: [u8; 16],

    bytes_range: Range<u64>,
}

unsafe impl Send for ContainerMeta {}

impl Size<u64> for ContainerMeta {
    fn get_size(&self) -> u64 {
        self.chunk_size * self.chunk_count
    }
}

/// A partial container is a container that is not yet fully uploaded
pub struct PartialContainer {
    meta: ContainerMeta,

    cipher: ChunkCipher,
}

unsafe impl Send for PartialContainer {}

impl PartialContainer {
    /// Create a new partial container
    ///
    /// * `chunk_size` - The size of each chunk
    /// * `container_max_size` - The maximum size of the container
    /// * `bytes_range` - The range of bytes stored in the container
    /// * `password` - The password to use for encryption
    pub fn new_container(
        chunk_size: u64,
        container_max_size: u64,
        bytes_range: Range<u64>,
        password: String,
    ) -> Result<Self, String> {
        // compute chunk count
        let range_size = bytes_range.get_size();

        let chunk_payload_size = chunk_payload_size(chunk_size);

        let chunk_count = (range_size + chunk_payload_size - 1) / chunk_payload_size; // round up

        if chunk_count * chunk_size > container_max_size {
            Err("Container max size exceeded")?
        }

        // derive key
        let (key, salt) = KeyDerivator::from_password(password).create_key();

        // create cipher
        let cipher = ChunkCipher::new(&key);

        Ok(Self {
            meta: ContainerMeta {
                chunk_size,
                chunk_count,
                salt,
                bytes_range,
            },
            cipher,
        })
    }

    pub fn encrypt_stream<R: Read> (&self, reader: R) -> StreamCipher<R> {
        StreamCipher::new(reader, self.meta.chunk_size as usize, self.cipher.clone())
    }

    /// Transform a partial container into a container
    ///
    /// * `public_url` - The public url of the container data's
    pub fn into_container(self, public_url: String) -> Container {
        Container {
            meta: self.meta,
            public_url,
        }
    }
}

/// The maximum number of chunks that can be stored in a container
///
/// * `max_container_size` - The maximum size of a container
pub fn max_chunk_count(max_container_size: u64, chunk_size: u64) -> u64 {
    max_container_size / chunk_size
}

/// The maximum number of bytes that can be stored in a chunk (payload)
///
/// * `chunk_size` - The size of a chunk
pub fn chunk_payload_size(chunk_size: u64) -> u64 {
    chunk_size - METADATA_SIZE
}


/// The maximum number of bytes that can be stored in a container (payload)
///
/// * `max_container_count` - The maximum number of chunks that can be stored in a container
/// * `chunk_size` - The size of a chunk
pub fn max_payload_size(max_container_count: u64, chunk_size: u64) -> u64 {
    max_chunk_count(max_container_count, chunk_size) * chunk_payload_size(chunk_size)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Container {
    meta: ContainerMeta,
    public_url: String,
}

#[derive(Debug, Clone, Copy)]
pub struct ChunkSplitter {
    pub(crate) chunk_size: u64,
    pub(crate) pad_size: u64,
    pub(crate) max_size: u64,
}

impl ChunkSplitter {
    pub fn new(chunk_size: u64, pad_size: u64, max_size: u64) -> Self {
        Self {
            chunk_size,
            pad_size,
            max_size,
        }
    }

    /// Converts a len into a multiple ranges
    /// representing each a container
    /// Each container contains X chunks of chunk_size
    /// And each chunk can be padded (means that the chunk size is bigger than the actual data)
    pub fn split_into_ranges(&self, len: u64) -> Vec<Range<u64>> {
        let payload_size = self.chunk_size - self.pad_size;

        let total_chunk_count = (len + payload_size - 1) / payload_size;

        let containers_per_range = self.max_size / self.chunk_size;

        let ranges_count = (total_chunk_count + containers_per_range - 1) / containers_per_range;

        let mut ranges: Vec<Range<u64>> = Vec::with_capacity(ranges_count as usize);

        for i in 0..ranges_count {
            let range_start = i * containers_per_range * (self.chunk_size - self.pad_size);

            let range_end = len.min((i + 1) * containers_per_range * (self.chunk_size - self.pad_size));

            ranges.push(range_start..range_end);
        }

        ranges
    }

    /// Adds padding to a range
    /// Convert the range with payload data into a range with padded data
    pub fn add_padding(&self, range: &Range<u64>) -> Range<u64> {
        let chunks_before = range.start / (self.chunk_size - self.pad_size);

        let size = range.end - range.start;

        let chunks_within = size / (self.chunk_size - self.pad_size);

        let start = range.start + chunks_before * self.pad_size;
        let mut end = range.end + chunks_within * self.pad_size;

        if end % self.chunk_size != 0 {
            end += self.chunk_size - (end % self.chunk_size);
        }

        return (start)..(end);
    }
}

#[cfg(test)]
mod test {
    #[test]
    pub fn test() {
        let splitter = super::ChunkSplitter {
            chunk_size: 1 << 16,
            pad_size: 36,
            max_size: 25 * 1024 * 1024,
        };

        let ranges = splitter.split_into_ranges(100 * 1000 * 1000);

        println!("{:?}", ranges);

        for range in ranges {
            println!("{:?}", splitter.add_padding(&range));
        }
    }
}