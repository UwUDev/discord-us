use std::ops::Range;
use serde::{Deserialize, Serialize};
use hex_buffer_serde::{Hex as _, HexForm};
use crate::{
    pack::{
        Size,
        key::KeyDerivator,
        crypt::{ChunkCipher, METADATA_SIZE},
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

pub struct Container {
    meta: ContainerMeta,
    public_url: String,
}