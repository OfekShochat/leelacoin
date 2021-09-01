#![feature(duration_constants)]

extern crate futures;
extern crate libp2p;
extern crate miniz_oxide;
extern crate serde_json;
extern crate sha3;

pub mod block;
mod blockchain;
mod network;

fn main() {
  network::main().unwrap();
}
