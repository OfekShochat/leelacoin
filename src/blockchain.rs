use crate::block::Block;

pub struct Chain {
  pub blocks: Vec<Block>,
}

impl Chain {
  pub fn new() -> Chain {
    let mut chain = Chain { blocks: vec![] };
    chain.create_genesis();
    chain
  }

  pub fn create_genesis(&mut self) {
    self.blocks.push(Block::new(
      "NOONE".to_string(),
      "NOONE".to_string(),
      0,
      "NONE".to_string(),
      true,
    ))
  }

  pub fn last(&self) -> &Block {
    &self.blocks[0]
  }

  pub fn add_block(&mut self, from: String, to: String, amount: u64) {
    self.prepend_block(Block::new(
      from,
      to,
      amount,
      self.last().summary.clone(),
      false,
    ));
  }

  fn prepend_block(&mut self, block: Block) {
    self.blocks.insert(0, block)
  }

  pub fn verify(&mut self) -> bool {
    for b in &self.blocks {
      if !b.verify() {
        return false;
      }
    }
    true
  }
}
