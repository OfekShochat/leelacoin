use std::{
  io::{Read, Write},
  net::{TcpListener, TcpStream},
};
use ed25519_dalek::{Keypair, Signature, Signer};
use log::{error, info};
use miniz_oxide::{deflate::compress_to_vec, inflate::decompress_to_vec};
use serde::{Deserialize, Serialize};
use std::sync::{Mutex, Arc};
use std::thread;

use crate::block::DataPoint;

const BUFFER_SIZE: usize = 65536;
const COMPRESSION_LEVEL: u8 = 9;
lazy_static! {
  static ref BOOT_NODES: Vec<String> = vec![
    "127.0.0.1:8000".to_string()
  ];
}

fn send_message(stream: &mut TcpStream, msg: &[u8]) {
  let compressed = compress_to_vec(msg, COMPRESSION_LEVEL);
  stream.write(&compressed).unwrap();
}

#[derive(Debug, Serialize, Deserialize)]
struct Message {
  destiny: String,
  source: String,
  pubkey: Vec<u8>,
  signed: Vec<u8>,
  data: Vec<DataPoint>,
}

pub struct Client {
  contact_list: Arc<Mutex<Vec<String>>>,
  keypair: Keypair,
}


impl Client {
  pub fn new(keypair: Keypair) -> Client {
    Client {
      contact_list: Arc::new(Mutex::new(BOOT_NODES.clone())),
      keypair
    }
  }

  pub fn main(&mut self) {
    let contacts = Arc::clone(&self.contact_list);
    thread::spawn(move || {
      Listener::new(contacts);
    });
  }

  fn get_chain(&mut self) {

  }
}

pub struct Listener {
  contact_list: Arc<Mutex<Vec<String>>>,
}

impl Listener {
  pub fn new(contact_list: Arc<Mutex<Vec<String>>>) {
    let mut l = Listener {
      contact_list,
    };
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

          let msg: Message = serde_json::from_slice(&stripped).unwrap();
          println!("{:?}", msg);
          self.forward(&buf);
        }
        Err(e) => error!("connection failed with {}", e),
      }
    }
  }

  fn get_message(&mut self, stream: &mut TcpStream, buf: &mut [u8; BUFFER_SIZE]) -> Vec<u8> {
    stream.read(&mut buf[..]).unwrap();
    let stripped = buf.strip_suffix(b"\0").unwrap(); // removing trailing zeros
    decompress_to_vec(stripped).unwrap()
  }

  fn forward(&mut self, buf: &[u8]) {
    for peer in self.contact_list.lock().unwrap().iter() {
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
}
