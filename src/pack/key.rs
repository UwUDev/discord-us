use pbkdf2::{pbkdf2_hmac};
use sha2::{Sha256};
use rand::{thread_rng, RngCore};

#[derive(Clone)]
pub struct KeyDerivator {
    password: String,
}

unsafe impl Send for KeyDerivator {}

impl KeyDerivator {
    pub fn from_password(password: String) -> Self {
        Self {
            password,
        }
    }

    /// Derive a key from the password and the salt
    /// Return the derived key
    ///
    /// * `salt` - The salt to use for the derivation (16 bytes)
    pub fn derive_password(&self, salt: &[u8; 16]) -> [u8; 32] {
        let mut key = [0u8; 32];

        pbkdf2_hmac::<Sha256>(self.password.as_bytes(), salt, 10_000, &mut key);

        key
    }

    /// Create a new key from the password
    /// Return the derived key and the random generated salt
    pub fn create_key(&self) -> ([u8; 32], [u8; 16]) {
        let salt = Self::generate_salt();

        let key = self.derive_password(&salt);

        (key, salt)
    }

    /// Generate a random salt
    fn generate_salt() -> [u8; 16] {
        let mut salt = [0u8; 16];
        thread_rng().fill_bytes(&mut salt);
        salt
    }
}

