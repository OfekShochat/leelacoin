use crate::block::{Block, DataPoint};
use serde_json::to_string;

const BLOCK_SIZE: usize = 1;

pub struct Chain {
  pub blocks: Vec<Block>,
  data_buffer: Vec<DataPoint>,
}

impl Chain {
  pub fn new() -> Chain {
    let mut chain = Chain {
      blocks: vec![],
      data_buffer: vec![],
    };
    chain.create_genesis();
    chain
  }

  pub fn from_vec(blocks: Vec<Block>) -> Chain {
    Chain {
      blocks,
      data_buffer: vec![],
    }
  }

  pub fn create_genesis(&mut self) {
    self.blocks.push(Block::new_genesis())
  }

  pub fn last(&self) -> &Block {
    &self.blocks[0]
  }

  fn create_block(&mut self) {
    self.prepend_block(Block::new(
      self.data_buffer.to_owned(),
      self.last().summary.clone(),
    ));
  }

  pub fn add_data(&mut self, from: String, mut data: DataPoint) {
    data.from = from;
    self.data_buffer.insert(0, data);
    self.check_buffer();
  }

  fn check_buffer(&mut self) {
    if self.data_buffer.len() == BLOCK_SIZE {
      self.create_block();
    }
  }

  fn prepend_block(&mut self, block: Block) {
    self.blocks.insert(0, block);
    println!("{}", self.verify());
  }

  pub fn verify(&mut self) -> bool {
    for b in &self.blocks {
      if b.genesis && !b.verify() {
        return false;
      }
    }
    true
  }

  pub fn check_balance(&mut self, pubkey: String) -> f64 {
    let mut b = 0.0;
    for i in &self.blocks {
      for d in &i.data {
        if d.from == pubkey {
          b -= d.amount;
        } else if d.to == pubkey {
          b += d.amount;
        }
      }
    }
    b
  }

  pub fn to_string(&mut self) -> Vec<String> {
    let mut out = vec![];
    for i in &self.blocks {
      out.push(to_string(i).unwrap());
    }
    out
  }

  pub fn to_vec(&mut self) -> Vec<Block> {
    self.blocks.clone()
  }
}
