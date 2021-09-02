use std::{io::Read, net::{TcpStream, TcpListener}};
// use rand::rngs::OsRng;
use ed25519_dalek::{Signature, Signer, Keypair};
use serde_json::Value;
use log::{info, error};
use miniz_oxide::{deflate::compress_to_vec, inflate::decompress_to_vec};

pub struct Listener {
  keypair: Keypair,
}

const BUFFER_SIZE: usize = 65536;

fn get_message(mut stream: TcpStream, buf: &mut [u8; BUFFER_SIZE]) -> Vec<u8> {
  stream.read(&mut buf[..]).unwrap();
  let stripped = buf.strip_suffix(b"\0").unwrap(); // removing trailing zeros
  decompress_to_vec(stripped).unwrap()
}

impl Listener {
  pub fn start(keypair: Keypair) {
    let mut l = Listener { keypair };
    l.main()
  }

  fn main(&mut self) {
    let listener = TcpListener::bind("0.0.0.0:0").unwrap();
    info!("Listening on {}", listener.local_addr().unwrap().to_string());
    for stream in listener.incoming() {
      match stream {
        Ok(stream) => {
          let mut buf = [0; BUFFER_SIZE];
          let stripped = get_message(stream, &mut buf);
          let cmd: Value = serde_json::from_slice(&stripped).unwrap();
          println!("{}", cmd);
        }
        Err(e) => error!("connection failed with {}", e)
      }
    }
  }
}