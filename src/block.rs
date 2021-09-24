extern crate chrono;
extern crate hex;

use self::hex::ToHex;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha3::{Digest, Sha3_256};

const COST: usize = 4;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DataPoint {
  pub from: String,
  pub to: String,
  pub amount: f64,
}

impl DataPoint {
  pub fn new(from: String, to: String, amount: f64) -> DataPoint {
    DataPoint { from, to, amount }
  }

  pub fn get(&self) -> Value {
    json!({
      "from": self.from,
      "to": self.to,
      "amount": self.amount,
    })
  }

  pub fn to_string(&self) -> String {
    self.get().to_string()
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
  pub summary: String,
  pub data: Vec<DataPoint>,
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
  while !h.starts_with(&"0".repeat(COST)) {
    nonce += 1;
    h = hash(&(data.to_string() + &nonce.to_string()));
  }
  (h, nonce)
}

fn data_to_string(data: &Vec<DataPoint>) -> String {
  let mut tobe_hashed = "".to_string();
  for d in data {
    tobe_hashed += &(d.from.clone() + &d.to + d.amount.to_string().as_str());
  }
  tobe_hashed
}


impl Block {
  pub fn new(data: Vec<DataPoint>, previous_hash: String) -> Block {
    let tobe_hashed = data_to_string(&data);
    let (summary, nonce) =
      hash_with_cost(tobe_hashed + &previous_hash);

    Block {
      summary,
      data,
      previous_summary: previous_hash,
      nonce,
      timestamp: Utc::now().timestamp(),
      genesis: false,
    }
  }

  pub fn new_genesis() -> Block {
    Block {
      summary: "NONE".to_string(),
      data: vec![DataPoint {
        from: "NOONE".to_string(),
        to: "NOONE".to_string(),
        amount: 0.0,
      }],
      previous_summary: "NONE".to_string(),
      nonce: 0,
      timestamp: 0,
      genesis: false,
    }
  }

  pub fn verify(&self) -> bool {
    if !self.genesis {
      hash(
        &(data_to_string(&self.data) +
          &self.previous_summary +
          self.nonce.to_string().as_str()),
      ) == self.summary
    } else {
      true
    }
  }
}
