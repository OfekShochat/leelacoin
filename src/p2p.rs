use ed25519_dalek::{Keypair, Signature, Signer};
use log::{error, info};
use miniz_oxide::{deflate::compress_to_vec, inflate::decompress_to_vec};
use serde::{Deserialize, Serialize};
use serde_bytes::Bytes;
use serde_json::{from_slice, to_string};
use std::io::stdin;
use std::sync::{Arc, Mutex};
use std::thread;
use std::{
  io::{Read, Write},
  net::{TcpListener, TcpStream},
};

use crate::block::DataPoint;

const BUFFER_SIZE: usize = 65536;
const COMPRESSION_LEVEL: u8 = 9;
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
}

pub struct Client {
  contact_list: Arc<Mutex<Vec<String>>>,
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
      keypair,
    }
  }

  pub fn main(&mut self) {
    let contacts = Arc::clone(&self.contact_list);
    thread::spawn(move || {
      Listener::new(contacts);
    });
    loop {
      let mut input = String::new();
      stdin().read_line(&mut input).unwrap();
      let splitted: Vec<&str> = input.split_whitespace().collect();
      match splitted[0] {
        "new-trans" => self.parse_transaction(&splitted[1..splitted.len()]),
        _ => eprintln!("invalid command: {}", splitted[0]),
      }
    }
  }

  fn parse_transaction(&mut self, splitted: &[&str]) {
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

  fn create_transaction(&mut self, data: DataPoint) {
    let msg = Message {
      destiny: "create-transaction".to_string(),
      pubkey: Bytes::new(&self.keypair.public.to_bytes()).to_vec(),
      signed: Bytes::new(&self.keypair.sign(data.to_string().as_bytes()).to_bytes()).to_vec(),
      data: vec![data],
    };
    self.send_all(to_string(&msg).unwrap().as_bytes());
  }

  fn send_all(&mut self, buf: &[u8]) {
    forward(self.contact_list.lock().unwrap().iter(), buf)
  }

  fn get_chain(&mut self) {}
}

pub struct Listener {
  contact_list: Arc<Mutex<Vec<String>>>,
}

impl Listener {
  pub fn new(contact_list: Arc<Mutex<Vec<String>>>) {
    let mut l = Listener { contact_list };
    l.main()
  }

  fn main(&mut self) {
    let listener = TcpListener::bind("0.0.0.0:0").unwrap();
    info!(
      "Listening on {}",
      listener.local_addr().unwrap().to_string()
    );
    for stream in listener.incoming() {
      match stream {
        Ok(mut stream) => {
          let mut buf = [0; BUFFER_SIZE];
          let stripped = self.get_message(&mut stream, &mut buf);

          let msg: Message = from_slice(&stripped).expect("poooo");
          println!("msg {:?}", &msg);
          self.forward(&buf);
        }
        Err(e) => error!("connection failed with {}", e),
      }
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
