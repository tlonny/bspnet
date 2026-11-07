mod builder;
mod packer;

pub use builder::{ATLAS_SIZE, Builder, Face};

#[cfg(test)]
#[path = "../test/packer.rs"]
mod test;
