//! Collects and decodes CLR resources from a PE file.
//!
//! The [CLR header](crate::clr::header::ClrHeader) points at a sequence of the following:
//!
//! ```
//! struct ResourceContainer {
//!     pub length: u32,
//!     pub data: [u8; length],
//!     pub padding: [u8; _], // to 8 bytes
//! }
//! ```


pub fn collect_resource_containers(slice: &[u8]) -> Vec<Vec<u8>> {
    let mut rest = slice;
    let mut containers = Vec::new();
    while rest.len() >= 4 {
        let length_u32 = u32::from_le_bytes(rest[0..4].try_into().unwrap());
        rest = &rest[4..];

        let length: usize = length_u32.try_into().unwrap();
        if rest.len() < length {
            break;
        }
        containers.push(rest[..length].to_vec());
        rest = &rest[length..];

        // padding to u32
        let padding = (4 - (length % 4)) % 4;
        rest = &rest[padding..];
    }
    containers
}
