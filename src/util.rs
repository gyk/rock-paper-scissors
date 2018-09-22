//! Helper functions

use std::fmt::Write;

use rand::{thread_rng, Rng};

/// Converts byte slice into lower hex string.
pub fn bytes_to_hex(bytes: &[u8]) -> String {
    let hex_len = bytes.len() * 2;
    let mut s = String::with_capacity(hex_len);
    for b in bytes.iter() {
        write!(&mut s, "{:x}", *b).expect("Unable to write");
    }
    s
}

/// Generates random bytes
pub fn gen_random_bytes(size: usize) -> Vec<u8> {
    let mut random_bytes = Vec::with_capacity(size);
    unsafe { random_bytes.set_len(size); }
    let mut rng = thread_rng();
    rng.fill(&mut random_bytes[..]);
    random_bytes
}
