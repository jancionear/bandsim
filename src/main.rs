// I don't like the .flatten() function, it's unintuitive
#![allow(clippy::manual_flatten)]

/// The whole bandsim is cfg(test), otherwise there's hundreds of "unused" warnings for things that are only used in the tests.
#[cfg(test)]
pub mod bandsim;

fn main() {
    println!("Run `cargo test` to test the bandwidth scheduler");
}
