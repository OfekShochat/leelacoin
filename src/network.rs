use std::sync::mpsc::{Receiver, Sender};

use miniz_oxide::{deflate::compress_to_vec, inflate::decompress_to_vec};

use serde_json::{from_slice, json, Value};

use crate::block::{Block, DataPoint};
use crate::blockchain;

fn process(recv: Receiver<String>, sender: Sender<Block>) {
  let mut bc = blockchain::Chain::new();
  loop {
    bc.add_block("poop".to_string(), "poopoo".to_string(), 5.0);
    let l = bc.last();
    sender.send(l.to_owned()).unwrap();
    if !bc.verify() {
      return;
    }
  }
}