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
pub mod config;
pub mod p2p;

use ed25519_dalek::Keypair;
use rand::rngs::OsRng;

fn main() {
  simple_logger::init().unwrap();
  let mut csprng = OsRng {};
  p2p::Client::new(
    Keypair::generate(&mut csprng),
    config::get_config().boot_node,
  )
  .main();
}
