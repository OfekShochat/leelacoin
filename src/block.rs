extern crate chrono;
extern crate hex;

use self::hex::ToHex;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha3::{Digest, Sha3_256};

const COST: u32 = 7;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DataPoint {
  from: String,
  to: String,
  amount: f64,
  pub timestamp: i64
}

impl DataPoint {
  pub fn new(from: String, to: String, amount: f64) -> DataPoint {
    DataPoint { from, to, amount, timestamp: Utc::now().timestamp() }
  }

  pub fn get(&self) -> Value {
    json!({
      "from": self.from,
      "to": self.to,
      "amount": self.amount,
      "timestamp": self.timestamp,
    })
  }

  pub fn to_string(&self) -> String {
    self.get().to_string()
  }
}

#[derive(Clone)]
pub struct Block {
  pub summary: String,
  pub data: DataPoint,
  pub previous_summary: String,
  pub nonce: u64,
  pub timestamp: i64,
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
  pub fn new(from: String, to: String, amount: f64, previous_hash: String) -> Block {
    let (summary, nonce) =
      hash_with_cost(from.clone() + &to + &previous_hash + amount.to_string().as_str());

    Block {
      summary,
      data: DataPoint::new(from, to, amount),
      previous_summary: previous_hash,
      nonce,
      timestamp: Utc::now().timestamp(),
      genesis: false,
    }
  }

  pub fn new_genesis() -> Block {
    Block {
      summary: "NONE".to_string(),
      data: DataPoint {
        from: "NOONE".to_string(),
        to: "NOONE".to_string(),
        amount: 0.0,
        timestamp: 0,
      },
      previous_summary: "NONE".to_string(),
      nonce: 0,
      timestamp: 0,
      genesis: false,
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
