#![feature(duration_constants)]

use std::net::TcpStream;

extern crate miniz_oxide;
extern crate serde_json;
extern crate sha3;
extern crate rand;
extern crate ed25519_dalek;
extern crate simple_logger;

pub mod block;
pub mod p2p;
mod blockchain;
mod network;

use ed25519_dalek::{Signature, Signer, Keypair}; // should remove
use rand::rngs::OsRng; // should remove

// const BOOT_NODES: [&str; 1] = ["127.0.0.1"];

fn main() {
  simple_logger::init().unwrap();
  let mut csprng = OsRng{};
  p2p::Listener::start(Keypair::generate(&mut csprng))
}
