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
    self.blocks.push(Block::new_genesis())
  }

  pub fn last(&self) -> &Block {
    &self.blocks[0]
  }

  pub fn add_block(&mut self, from: String, to: String, amount: f64) {
    self.prepend_block(Block::new(from, to, amount, self.last().summary.clone()));
  }

  fn prepend_block(&mut self, block: Block) {
    self.blocks.insert(0, block)
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
      if i.data.from == pubkey {
        b -= i.data.amount;
      } else if i.data.to == pubkey {
        b += i.data.amount;
      }
    }
    b
  }
}
