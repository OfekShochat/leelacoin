extern crate hex;

use self::hex::ToHex;
use serde_json::json;
use sha3::{Digest, Sha3_256};

const COST: u32 = 2;

#[derive(Clone)]
pub struct DataPoint {
  from: String,
  to: String,
  amount: u64,
}

impl DataPoint {
  pub fn get_string(&self) -> String {
    json!({
      "from": self.from,
      "to": self.to,
      "amount": self.amount
    }).to_string()
  }
}

#[derive(Clone)]
pub struct Block {
  pub summary: String,
  pub data: DataPoint,
  pub previous_summary: String,
  pub nonce: u64,
  pub genesis: bool,
}

fn hash(data: &String) -> String {
  let mut hasher = Sha3_256::new();
  hasher.update(data);

  hasher.finalize().as_slice().encode_hex()
}

fn hash_with_cost(data: String) -> (String, u64) {
  let mut h = "".to_string();
  let mut nonce = 0;
  while !h.starts_with(&"0".repeat(COST as usize)) {
    nonce += 1;
    h = hash(&(data.to_string() + &nonce.to_string()));
  }
  (h, nonce)
}

impl Block {
  pub fn new(from: String, to: String, amount: u64, previous_hash: String, genesis: bool) -> Block {
    let (summary, nonce) =
      hash_with_cost(from.clone() + &to + &previous_hash + amount.to_string().as_str());

    Block {
      summary,
      data: DataPoint { from, to, amount },
      previous_summary: previous_hash,
      nonce,
      genesis,
    }
  }

  pub fn verify(&self) -> bool {
    if !self.genesis {
      hash(
        &(self.data.from.clone() +
          &self.data.to +
          &self.previous_summary +
          self.data.amount.to_string().as_str() +
          self.nonce.to_string().as_str()),
      ) == self.summary
    } else {
      true
    }
  }
}
