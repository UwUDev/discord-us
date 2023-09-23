use std::io::Read;
use aes_gcm::{
    AeadInPlace, Aes256Gcm, KeyInit, AeadCore,
    aead::{OsRng, Error},
};
use crate::utils::read::Chunked;

/// Encrypted Chunks are composed in this way
/// 0      -     11 | 12   -   <chunk-size>-1 | <chunk-size> - <chunk-size> + 16 |
/// Nonce (12bytes) |      cypher text        |              Auth tag            |
/// It means metadata whe have 12 + 16 bytes of metadata
///
pub const METADATA_SIZE: u64 = 28;

#[derive(Clone)]
pub struct ChunkCipher {
    cipher: Aes256Gcm,
}

unsafe impl Send for ChunkCipher {}

/// ChunkCipher is a wrapper around aes_gcm::Aes256Gcm
/// It's used to encrypt and decrypt chunks
impl ChunkCipher {
    /// Create a new ChunkCipher
    ///
    /// * `key` - The key to use for encryption and decryption (32 bytes)
    pub fn new(key: &[u8; 32]) -> Self {
        Self {
            cipher: Aes256Gcm::new(key.into()),
        }
    }

    /// Encrypt a specific padded chunk
    /// 12 empty bytes before for Nonce
    /// 16 empty bytes after for Auth tag are composed in this way
    ///
    /// * `chunk` - The chunk to encrypt with content located at 12..chunk.len() - 16. Note: the chunk length must be a multiple of 16
    pub fn encrypt(&self, chunk: &mut [u8]) -> Result<(), Error> {
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

        let len = chunk.len();

        chunk[..12].copy_from_slice(nonce.as_slice()); // write nonce

        let content = &mut chunk[12..len - 16];

        let tag = self.cipher.encrypt_in_place_detached(
            &nonce,
            &[],
            content,
        )?;

        chunk[len - 16..].copy_from_slice(tag.as_slice()); // write tag

        Ok(())
    }

    /// Decrypt a specific encrypted chunk
    /// The chunk data will be mutated in place
    /// The decrypted data is accessible at 12..chunk.len() - 16
    pub fn decrypt(&self, chunk: &mut [u8]) -> Result<(), Error> {
        let nonce = &chunk[..12].to_vec();

        let tag = &chunk[chunk.len() - 16..].to_vec();

        let len = chunk.len();

        let content = &mut chunk[12..len - 16];

        self.cipher.decrypt_in_place_detached(
            nonce.as_slice().into(),
            &[],
            content,
            tag.as_slice().into(),
        )?;

        return Ok(());
    }
}

pub struct StreamCipher<R: Read> {
    cipher: ChunkCipher,
    reader: R,
    chunk_size: usize,
}

impl<R: Read> StreamCipher<R> {
    pub fn new(reader: R, chunk_size: usize, cipher: ChunkCipher) -> Self {
        Self {
            cipher,
            reader,
            chunk_size,
        }
    }
}

impl<R: Read> Chunked for StreamCipher<R> {
    fn process_next_chunk(&mut self) -> Option<Vec<u8>> {
        let mut chunk = /*[0u8;1<<16];*/vec![0u8; self.chunk_size];

        let mut read = 0;

        while read < self.chunk_size - METADATA_SIZE as usize {
            let r = self.reader.read(&mut chunk[(12 + read) as usize..(self.chunk_size - 16) as usize]).unwrap();

            if r == 0 {
                break;
            }

            read += r;
        }

        self.cipher.encrypt(&mut chunk).unwrap();

        Some(chunk.to_vec())
    }
}

#[cfg(test)]
mod test {
    use crate::pack::crypt::{
        ChunkCipher,
        METADATA_SIZE,
    };

    #[test]
    pub fn test() {
        const CHUNK_SIZE: usize = 1 << 16; // use chunk of 65536 bytes
        const VALUE: u8 = 5u8;

        let key: [u8; 32] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31];

        let cipher = ChunkCipher::new(&key);

        let size = CHUNK_SIZE - METADATA_SIZE as usize;
        let data = vec![VALUE; size];

        let mut chunk = [0u8; CHUNK_SIZE];

        chunk[12..CHUNK_SIZE - 16].copy_from_slice(data.as_slice());

        cipher.encrypt(&mut chunk).unwrap();

        assert!(!&chunk[12..(1 << 16) - 16].iter().all(|&x| x == VALUE));

        cipher.decrypt(&mut chunk).unwrap();

        assert!(&chunk[12..(1 << 16) - 16].iter().all(|&x| x == VALUE));
    }
}