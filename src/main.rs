#![feature(duration_constants)]

extern crate ed25519_dalek;
extern crate miniz_oxide;
extern crate rand;
extern crate serde_json;
extern crate sha3;
extern crate simple_logger;
#[macro_use]
extern crate lazy_static;

pub mod block;
mod blockchain;
mod network;
pub mod p2p;

use ed25519_dalek::{Keypair, Signature, Signer}; // should remove
use rand::rngs::OsRng; // should remove

// const BOOT_NODES: [&str; 1] = ["127.0.0.1"];

fn main() {
  simple_logger::init().unwrap();
  let mut csprng = OsRng {};
  p2p::Client::new(Keypair::generate(&mut csprng));
}
