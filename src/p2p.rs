use std::{io::{Error, Read, Write}, net::{TcpListener, TcpStream}};
// use rand::rngs::OsRng;
use ed25519_dalek::{Keypair, Signature, Signer};
use log::{error, info};
use miniz_oxide::{deflate::compress_to_vec, inflate::decompress_to_vec};
use serde::{Deserialize, Serialize};

use crate::block::DataPoint;

pub struct Listener {
  keypair: Keypair,
}

const BUFFER_SIZE: usize = 65536;

fn get_message(stream: &mut TcpStream, buf: &mut [u8; BUFFER_SIZE]) -> Vec<u8> {
  stream.read(&mut buf[..]).unwrap();
  let stripped = buf.strip_suffix(b"\0").unwrap(); // removing trailing zeros
  decompress_to_vec(stripped).unwrap()
}

fn send_message(stream: &mut TcpStream, msg: &[u8]) -> Result<(), Error> {
  let compressed = compress_to_vec(msg, 9);
  let res = stream.write(&compressed);
  if res.is_err() {
    Err(res.unwrap_err())
  } else {
    Ok(())
  }
}

#[derive(Debug, Serialize, Deserialize)]
struct Message {
  destiny: String,
  pubkey: Vec<u8>,
  signed: Vec<u8>,
  data: Vec<DataPoint>,
}

impl Listener {
  pub fn start(keypair: Keypair) {
    let mut l = Listener { keypair };
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
          let stripped = get_message(&mut stream, &mut buf);
          let msg: Message = serde_json::from_slice(&stripped).unwrap();
          send_message(&mut stream, b"poop").unwrap();
        }
        Err(e) => error!("connection failed with {}", e),
      }
    }
  }
}
