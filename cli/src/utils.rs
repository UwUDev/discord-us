use rand::{distributions::Alphanumeric, Rng};

pub fn create_random_password (length: usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect()
}