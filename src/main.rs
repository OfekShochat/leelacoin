extern crate futures;
extern crate libp2p;
extern crate miniz_oxide;
extern crate serde_json;
extern crate sha3;

pub mod block;
mod blockchain;
mod network;

fn main() {
  // let b = block::Block::new(
  //   "hello".to_string(),
  //   "hello".to_string(),
  //   5,
  //   "".to_string(),
  //   false,
  // );
  // println!("{}", b.verify());
  // let mut bc = blockchain::Chain::new();
  // println!("{}", bc.blocks[0].summary);
  // bc.add_block("from".to_string(), "to".to_string(), 5);
  // println!("{}", bc.blocks[0].summary);
  // println!("{}", bc.verify());
  network::main().unwrap();
}