use ebml::vint::{read_vint, write_vint};
use proptest::prelude::*;

#[test]
fn test_read_vint_fuzzing() {
    dotenv::dotenv().ok();
    env_logger::try_init().ok();
    proptest!(|(buf: Vec<u8>)| {
        read_vint(&buf, 0).ok();
    });
}

#[test]
fn test_write_vint_fuzzing() {
    dotenv::dotenv().ok();
    env_logger::try_init().ok();
    proptest!(|(i: i64)| {
        write_vint(i).ok();
    });
}
