#![cfg_attr(feature="clippy", feature(plugin))]
#![cfg_attr(feature="clippy", plugin(clippy))]

extern crate byteorder;
extern crate bytes;

mod constants;
mod encoder;

pub use encoder::EncoderWriter;

#[cfg(test)]
mod tests;
