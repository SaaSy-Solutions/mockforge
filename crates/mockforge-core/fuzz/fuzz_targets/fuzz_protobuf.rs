#![no_main]

use libfuzzer_sys::fuzz_target;
use prost_reflect::DescriptorPool;

fuzz_target!(|data: &[u8]| {
    // Try to decode the fuzz input as a protobuf file descriptor set
    // File descriptor sets are binary protobuf messages that describe
    // the structure of other protobuf messages
    let mut pool = DescriptorPool::new();

    // Attempt to decode as file descriptor set
    // Should never panic, even with malformed descriptor sets
    let _ = pool.decode_file_descriptor_set(data);
});
