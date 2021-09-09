use chrono::Utc;
use ed25519_dalek::{Keypair, PublicKey, Signature, Signer, Verifier};
use log::{error, info};
use miniz_oxide::{deflate::compress_to_vec, inflate::decompress_to_vec};
use serde::{Deserialize, Serialize};
use serde_bytes::Bytes;
use serde_json::{from_slice, to_string};
use std::convert::TryFrom;
use std::io::stdin;
use std::sync::{Arc, Mutex};
use std::thread;
use std::{
  io::{Read, Write},
  net::{TcpListener, TcpStream},
};
use hex::ToHex;

use crate::block::DataPoint;
use crate::blockchain::Chain;

const BUFFER_SIZE: usize = 65536;
const COMPRESSION_LEVEL: u8 = 9;
const TTL: usize = 3600;
lazy_static! {
  static ref BOOT_NODES: Vec<String> = vec!["127.0.0.1:60129".to_string()];
}

fn send_message(stream: &mut TcpStream, msg: &[u8]) {
  let compressed = compress_to_vec(msg, COMPRESSION_LEVEL);
  stream.write(&compressed).unwrap();
}

fn forward(contact_list: std::slice::Iter<String>, buf: &[u8]) {
  for peer in contact_list {
    match TcpStream::connect(&peer) {
      Ok(mut stream) => {
        send_message(&mut stream, buf);
      }
      Err(e) => {
        error!("couldn't connect to {} with {}", peer, e);
        continue;
      }
    }
  }
}

fn validate_sig(pubkey: &Vec<u8>, msg: String, signed: &Vec<u8>) -> bool {
  let p = PublicKey::from_bytes(&pubkey).unwrap();
  let r = p.verify(msg.as_bytes(), &Signature::try_from(&signed[..]).unwrap());
  r.is_ok()
}

fn strip_trailing(buf: &[u8]) -> &[u8] {
  let mut pos: usize = 0;
  for i in 0..buf.len() {
    if buf[i..i + 3] == [0, 0, 0] {
      pos = i;
      break;
    }
  }
  &buf[0..pos]
}

#[derive(Debug, Serialize, Deserialize)]
struct Message {
  destiny: String,
  #[serde(with = "serde_bytes")]
  pubkey: Vec<u8>,
  #[serde(with = "serde_bytes")]
  signed: Vec<u8>,
  data: Vec<DataPoint>,
  timestamp: i64,
}

pub struct Client {
  contact_list: Arc<Mutex<Vec<String>>>,
  banned_list: Arc<Mutex<Vec<Vec<u8>>>>,
  chain: Arc<Mutex<Chain>>,
  keypair: Keypair,
}

impl Client {
  pub fn new(keypair: Keypair, boot_node: bool) -> Client {
    Client {
      contact_list: Arc::new(Mutex::new(if boot_node {
        vec![]
      } else {
        BOOT_NODES.clone()
      })),
      banned_list: Arc::new(Mutex::new(vec![])),
      keypair,
      chain: Arc::new(Mutex::new(Chain::new()))
    }
  }

  pub fn main(&self) {
    let contacts = Arc::clone(&self.contact_list);
    let banned = Arc::clone(&self.banned_list);
    let chain = Arc::clone(&self.chain);
    thread::spawn(move || {
      Listener::new(contacts, banned, chain);
    });
    loop {
      let mut input = String::new();
      stdin().read_line(&mut input).unwrap();
      let splitted: Vec<&str> = input.split_whitespace().collect();
      match splitted[0] {
        "new-trans" => self.parse_transaction(&splitted[1..splitted.len()]),
        "get-chain" => self.get_chain(),
        _ => eprintln!("invalid command: {}", splitted[0]),
      }
    }
  }

  fn parse_transaction(&self, splitted: &[&str]) {
    let mut amount = "";
    let mut to = "";
    for i in 0..splitted.len() {
      match splitted[i] {
        "amm" => amount = splitted[i + 1],
        "to" => to = splitted[i + 1],
        _ => continue,
      }
    }
    self.create_transaction(DataPoint::new(
      "".to_string(),
      to.to_string(),
      amount.parse().unwrap(),
    ))
  }

  fn create_transaction(&self, data: DataPoint) {
    let current_time = Utc::now().timestamp();
    let msg = Message {
      destiny: "create-transaction".to_string(),
      pubkey: Bytes::new(&self.keypair.public.to_bytes()).to_vec(),
      signed: Bytes::new(
        &self
          .keypair
          .sign((data.to_string() + &current_time.to_string()).as_bytes())
          .to_bytes(),
      )
      .to_vec(),
      data: vec![data],
      timestamp: current_time,
    };
    self.send_all(to_string(&msg).unwrap().as_bytes());
  }

  fn send_all(&self, buf: &[u8]) {
    forward(self.contact_list.lock().unwrap().iter(), buf)
  }

  fn get_chain(&self) {
    let current_time = Utc::now().timestamp();
    let msg = Message {
      destiny: "get-chain".to_string(),
      pubkey: Bytes::new(&self.keypair.public.to_bytes()).to_vec(),
      signed: Bytes::new(b"NONE").to_vec(),
      data: vec![],
      timestamp: current_time,
    };
    self.send_all(to_string(&msg).unwrap().as_bytes())
  }
}

pub struct Listener {
  contact_list: Arc<Mutex<Vec<String>>>,
  banned_list: Arc<Mutex<Vec<Vec<u8>>>>,
  chain: Arc<Mutex<Chain>>,
  processed: Vec<Vec<u8>>,
}

impl Listener {
  pub fn new(contact_list: Arc<Mutex<Vec<String>>>, banned_list: Arc<Mutex<Vec<Vec<u8>>>>, chain: Arc<Mutex<Chain>>) {
    let mut l = Listener {
      contact_list,
      banned_list,
      processed: vec![],
      chain
    };
    l.main()
  }

  fn main(&mut self) {
    use crate::config as cfg_reader;
    let cfg = cfg_reader::get_config();
    let listener = TcpListener::bind(format!("0.0.0.0:{}", cfg.port)).unwrap();
    info!(
      "Listening on {}",
      listener.local_addr().unwrap().to_string()
    );
    for stream in listener.incoming() {
      match stream {
        Ok(mut stream) => {
          let mut buf = [0; BUFFER_SIZE];
          let stripped = self.get_message(&mut stream, &mut buf);

          let msg: Message = from_slice(&stripped).unwrap();
          println!("{:?}", &msg);
          if self.banned_list.lock().unwrap().contains(&msg.pubkey) {
            continue;
          } else if msg.timestamp + (TTL as i64) < Utc::now().timestamp() || self.processed.contains(&msg.signed)
          {
            info!(
              "node {}... - {} has provided an expired/already used timestamp.",
              hex::encode(&msg.pubkey)[0..10].to_string(),
              stream.peer_addr().unwrap()
            )
          } else if !validate_sig(
            &msg.pubkey,
            msg.data[0].to_string() + &msg.timestamp.to_string(),
            &msg.signed,
          ) {
            info!(
              "node {}... - {} has provided an invalid signature.",
              hex::encode(&msg.pubkey)[0..10].to_string(),
              stream.peer_addr().unwrap()
            )
          } else {
            self.process_ok(&msg)
          }
          self.processed.push(msg.signed);
          self.cleanup();
          self.forward(&buf);
        }
        Err(e) => error!("connection failed with {}", e),
      }
    }
  }

  fn process_ok(&mut self, msg: &Message) {
    match msg.destiny.as_str() {
      "create-transaction" => {
        self.chain.lock().unwrap().check_balance(msg.pubkey.encode_hex());
      }
      "get-chain" => {
        // self.chain.lock().unwrap().blocks
      }
      _ => {}
    }
  }

  fn ban(&mut self, pubkey: Vec<u8>) {
    self.banned_list.lock().unwrap().push(pubkey)
  }

  fn cleanup(&mut self) {
    while self.processed.len() > TTL {
      self.processed.remove(0);
    }
  }

  fn get_message(&mut self, stream: &mut TcpStream, buf: &mut [u8; BUFFER_SIZE]) -> Vec<u8> {
    stream.read(&mut buf[..]).unwrap();
    let stripped = strip_trailing(buf); // removing trailing zeros
    decompress_to_vec(stripped).unwrap()
  }

  fn forward(&mut self, buf: &[u8]) {
    forward(self.contact_list.lock().unwrap().iter(), buf)
  }
}
