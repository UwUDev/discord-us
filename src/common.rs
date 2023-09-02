use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct Waterfall {
    pub filename: String,
    pub password: String,
    pub size: u64,

    pub containers: Vec<Container>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Container {
    pub storage_url: String,
    pub chunk_size: u64,
    pub chunk_count: u64,

    pub salt: [u8; 16],

    pub bytes_range: [u64; 2],
}
