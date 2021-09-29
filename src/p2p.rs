use chrono::Utc;
use ed25519_dalek::{Keypair, PublicKey, Signature, Signer, Verifier};
use hex::ToHex;
use log::{error, info};
use miniz_oxide::{deflate::compress_to_vec, inflate::decompress_to_vec};
use serde::{Deserialize, Serialize};
use serde_bytes::Bytes;
use serde_json::{from_slice, to_string};
use std::convert::TryFrom;
use std::io::stdin;
use std::ops::AddAssign;
use std::sync::{Arc, Mutex};
use std::thread;
use std::{
  io::{Read, Write},
  net::{TcpListener, TcpStream},
};

use crate::block::{Block, DataPoint};
use crate::blockchain::Chain;

const BUFFER_SIZE: usize = 65536 / 8;
const COMPRESSION_LEVEL: u8 = 9;
const TTL: usize = 3600;
const VALIDATOR: bool = true;
lazy_static! {
  static ref BOOT_NODES: Vec<String> = vec!["127.0.0.1:60000".to_string()];
}

fn forward(contact_list: std::slice::Iter<String>, buf: &[u8]) {
  let compressed = compress_to_vec(buf, COMPRESSION_LEVEL);
  for peer in contact_list {
    match TcpStream::connect(&peer) {
      Ok(mut stream) => {
        stream.write(&compressed).unwrap();
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

fn strip_trailing(buf: &[u8], trail: usize) -> &[u8] {
  for i in 0..buf.len() {
    if buf[i..i + 3] == [0, 0, 0] {
      return &buf[0..i + trail];
    }
  }
  unreachable!();
}

#[derive(Debug, Serialize, Deserialize)]
struct Message {
  destiny: String,
  #[serde(with = "serde_bytes")]
  pubkey: Vec<u8>,
  #[serde(with = "serde_bytes")]
  signed: Vec<u8>,
  data: Vec<DataPoint>,
  blocks: Vec<Block>,
  contacts: Vec<String>,
  timestamp: i64,
  contact: String,
}

pub struct Client {
  contact_list: Arc<Mutex<Vec<String>>>,
  banned_list: Arc<Mutex<Vec<Vec<u8>>>>,
  chain: Arc<Mutex<Chain>>,
  keypair: Keypair,
  contact: Arc<Mutex<String>>,
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
      chain: Arc::new(Mutex::new(Chain::new())),
      contact: Arc::new(Mutex::new(String::new())),
    }
  }

  pub fn main(&self) {
    let contacts = Arc::clone(&self.contact_list);
    let banned = Arc::clone(&self.banned_list);
    let chain = Arc::clone(&self.chain);
    let contact = Arc::clone(&self.contact);
    thread::spawn(move || {
      Listener::new(contacts, banned, chain, contact);
    });
    println!(
      "Starting client with ID={}",
      self.keypair.public.encode_hex::<String>()
    );
    loop {
      let mut input = String::new();
      stdin().read_line(&mut input).unwrap();
      let splitted: Vec<&str> = input.split_whitespace().collect();
      match splitted[0] {
        "new-trans" => self.parse_transaction(&splitted[1..splitted.len()]),
        "get-chain" => self.get_chain(),
        "get-contacts" => self.get_contacts(),
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
    let contact = self.contact.lock().unwrap().to_string();
    let current_time = Utc::now().timestamp();
    let msg = Message {
      destiny: "create-transaction".to_string(),
      pubkey: Bytes::new(&self.keypair.public.to_bytes()).to_vec(),
      signed: Bytes::new(
        &self
          .keypair
          .sign((data.to_string() + &current_time.to_string() + &contact).as_bytes())
          .to_bytes(),
      )
      .to_vec(),
      data: vec![data],
      blocks: vec![],
      contacts: vec![],
      timestamp: current_time,
      contact,
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
      blocks: vec![],
      contacts: vec![],
      timestamp: current_time,
      contact: self.contact.lock().unwrap().to_string(),
    };
    self.send_all(to_string(&msg).unwrap().as_bytes());
  }

  fn get_contacts(&self) {
    let current_time = Utc::now().timestamp();
    let msg = Message {
      destiny: "get-contacts".to_string(),
      pubkey: Bytes::new(&self.keypair.public.to_bytes()).to_vec(),
      signed: Bytes::new(b"NONE").to_vec(),
      data: vec![],
      blocks: vec![],
      contacts: vec![],
      timestamp: current_time,
      contact: self.contact.lock().unwrap().to_string(),
    };
    self.send_all(to_string(&msg).unwrap().as_bytes());
  }
}

pub struct Listener {
  contact_list: Arc<Mutex<Vec<String>>>,
  banned_list: Arc<Mutex<Vec<Vec<u8>>>>,
  chain: Arc<Mutex<Chain>>,
  processed: Vec<Vec<u8>>,
  contact: Arc<Mutex<String>>,
}

impl Listener {
  pub fn new(
    contact_list: Arc<Mutex<Vec<String>>>,
    banned_list: Arc<Mutex<Vec<Vec<u8>>>>,
    chain: Arc<Mutex<Chain>>,
    contact: Arc<Mutex<String>>,
  ) {
    let mut l = Listener {
      contact_list,
      banned_list,
      processed: vec![],
      chain,
      contact,
    };
    l.main()
  }

  fn main(&mut self) {
    use crate::config as cfg_reader;
    let cfg = cfg_reader::get_config();
    let listener = TcpListener::bind(format!("127.0.0.1:{}", cfg.port)).unwrap();
    let contact = listener.local_addr().unwrap().to_string();
    self.contact.lock().unwrap().add_assign(contact.as_str());
    info!("Listening on {}", contact);
    for stream in listener.incoming() {
      match stream {
        Ok(mut stream) => {
          let mut buf = [0; BUFFER_SIZE];
          let stripped = self.get_message(&mut stream, &mut buf);

          let msg: Message = from_slice(&stripped).unwrap();
          println!("{:?}", &msg);
          if self.banned(&msg.pubkey) || self.invalid_timestamp(msg.timestamp) {
            continue;
          }

          match msg.destiny.as_str() {
            "create-transaction" => {
              if !validate_sig(
                &msg.pubkey,
                msg.data[0].to_string() + &msg.timestamp.to_string() + &msg.contact,
                &msg.signed,
              ) || self.processed.contains(&msg.signed)
              {
                continue;
              }

              self.add_contact(msg.contact.to_string());

              if !VALIDATOR {
                continue;
              }

              self
                .chain
                .lock()
                .unwrap()
                .add_data(msg.pubkey.encode_hex(), msg.data[0].to_owned());
              self.forward(&stripped, msg.contact);
            }
            "get-chain" => {
              self.add_contact(msg.contact.to_string());
              let blocks = self.chain.lock().unwrap().to_vec();
              self.give_chain(blocks, contact.to_string());
            }
            "give-chain" => {
              if !self.contact_list.lock().unwrap().contains(&contact) {
                continue;
              }
              self.add_contact(msg.contact.to_string());

              let new_chain = Chain::from_vec(msg.blocks);
              if new_chain.verify() {
                self.chain = Arc::new(Mutex::new(new_chain));
                println!("chian chian {:?}", self.chain.lock().unwrap().to_string());
              } else {
                self.ban(msg.pubkey);
              }
            }
            "get-contacts" => {
              self.add_contact(msg.contact.to_string());
              let contacts = self.contact_list.lock().unwrap().to_vec();
              self.give_contacts(contacts, contact.to_string());
            }
            "give-contacts" => {
              self.add_contacts(msg.contacts);
            }
            _ => continue,
          }
          self.processed.push(msg.signed);
          self.cleanup();
        }
        Err(e) => error!("connection failed with {}", e),
      }
    }
  }

  fn give_chain(&mut self, blocks: Vec<Block>, contact: String) {
    self.forward(
      to_string(&Message {
        destiny: "give-chain".to_string(),
        pubkey: "NONE".as_bytes().to_vec(),
        data: vec![],
        blocks,
        contacts: vec![],
        signed: "NONE".as_bytes().to_vec(),
        timestamp: Utc::now().timestamp(),
        contact: contact.clone(),
      })
      .unwrap()
      .as_bytes(),
      contact,
    );
  }

  fn give_contacts(&mut self, contacts: Vec<String>, contact: String) {
    self.forward(
      to_string(&Message {
        destiny: "give-contacts".to_string(),
        pubkey: "NONE".as_bytes().to_vec(),
        data: vec![],
        blocks: vec![],
        contacts,
        signed: "NONE".as_bytes().to_vec(),
        timestamp: Utc::now().timestamp(),
        contact: contact.clone(),
      })
      .unwrap()
      .as_bytes(),
      contact,
    );
  }

  fn add_contact(&mut self, contact: String) {
    if !self.contact_list.lock().unwrap().contains(&contact) {
      self.contact_list.lock().unwrap().push(contact);
    }
  }

  fn add_contacts(&mut self, contacts: Vec<String>) {
    let mycontact = self.contact.lock().unwrap().to_owned();
    for c in contacts {
      if c != mycontact {
        self.add_contact(c);
      }
    }
  }

  fn banned(&mut self, pubkey: &Vec<u8>) -> bool {
    self.banned_list.lock().unwrap().contains(pubkey)
  }

  fn invalid_timestamp(&mut self, timestamp: i64) -> bool {
    timestamp + (TTL as i64) < Utc::now().timestamp()
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
    let stripped = strip_trailing(buf, 0); // removing trailing zeros
    let mut d = decompress_to_vec(stripped);
    let mut trail: usize = 1;
    while d.is_err() {
      let stripped = strip_trailing(buf, trail); // removing trailing zeros
      d = decompress_to_vec(stripped);
      trail += 1;
    }
    d.unwrap()
  }

  fn forward(&mut self, buf: &[u8], contact: String) {
    let mut contacts = self.contact_list.lock().unwrap().clone();
    if contacts.contains(&contact) {
      let index = contacts.iter().position(|x| *x == contact).unwrap();
      contacts.remove(index);
    }
    forward(contacts.iter(), buf)
  }
}
