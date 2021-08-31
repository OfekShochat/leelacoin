use async_std::task;
use futures::{future, prelude::*};
use libp2p::{
  floodsub::{self, Floodsub, FloodsubEvent, Topic},
  identity,
  mdns::{Mdns, MdnsConfig, MdnsEvent},
  swarm::{NetworkBehaviourEventProcess, SwarmEvent},
  NetworkBehaviour, PeerId, Swarm,
};

use miniz_oxide::{deflate::compress_to_vec, inflate::decompress_to_vec};

use serde_json::json;

use crate::block::Block;
use crate::blockchain;

use std::{
  error::Error,
  str::FromStr,
  sync::mpsc::{self, Receiver, Sender},
  task::{Context, Poll},
};

#[derive(NetworkBehaviour)]
struct Client {
  floodsub: Floodsub,
  mdns: Mdns,

  #[behaviour(ignore)]
  sender: Sender<String>,
}

impl Client {
  pub fn report_mine(&mut self, topic: Topic, block: &Block) {
    let compressed = compress_to_vec(
      json!({
        "report": "mined",
        "hash": block.summary,
        "data": block.data.to_string(),
        "previous": block.previous_summary,
        "nonce": block.nonce
      })
      .to_string()
      .as_bytes(),
      9, // best compression
    );
    self.floodsub.publish(topic, compressed);
  }
}

impl NetworkBehaviourEventProcess<FloodsubEvent> for Client {
  // Called when `floodsub` produces an event.
  fn inject_event(&mut self, message: FloodsubEvent) {
    if let FloodsubEvent::Message(message) = message {
      let decompressed = decompress_to_vec(&message.data).unwrap();
      println!(
        "Received: '{:?}' from {:?}",
        String::from_utf8_lossy(decompressed.as_slice()),
        message.source
      );
    }
  }
}

impl NetworkBehaviourEventProcess<MdnsEvent> for Client {
  // Called when `mdns` produces an event.
  fn inject_event(&mut self, event: MdnsEvent) {
    match event {
      MdnsEvent::Discovered(list) => {
        for (peer, _) in list {
          println!("discovered {:?}", peer);
          self.floodsub.add_node_to_partial_view(peer);
        }
      }
      MdnsEvent::Expired(list) => {
        for (peer, _) in list {
          if !self.mdns.has_node(&peer) {
            self.floodsub.remove_node_from_partial_view(&peer);
          }
        }
      }
    }
  }
}

fn process(recv: Receiver<String>, sender: Sender<Block>) {
  let mut bc = blockchain::Chain::new();
  loop {
    bc.add_block("poop".to_string(), "poopoo".to_string(), 5);
    let l = bc.last();
    sender.send(l.to_owned()).unwrap();
    if !bc.verify() {
      return;
    }
  }
}

#[async_std::main]
pub async fn main() -> Result<(), Box<dyn Error>> {
  let local_key = identity::Keypair::generate_ed25519();
  let local_peer_id = PeerId::from(local_key.public());
  println!("Local peer id: {:?}", local_peer_id);

  // Set up a an encrypted DNS-enabled TCP Transport over the Mplex and Yamux protocols
  let transport = libp2p::development_transport(local_key).await?;

  let (event_sender, event_receiver) = mpsc::channel();

  // Create a Floodsub topic
  let floodsub_topic = floodsub::Topic::new("leelacoin");
  // Create a Swarm to manage peers and events
  let mut swarm = {
    let mdns = task::block_on(Mdns::new(MdnsConfig::default()))?;
    let mut behavior = Client {
      floodsub: Floodsub::new(local_peer_id),
      mdns,
      sender: event_sender,
    };

    behavior.floodsub.subscribe(floodsub_topic.clone());
    Swarm::new(transport, behavior, local_peer_id)
  };

  let (_miner_to_main_sender, miner_receiver_to_main) = mpsc::channel();
  let (miner_sender, miner_receiver) = mpsc::channel();
  std::thread::spawn(move || process(miner_receiver_to_main, miner_sender));

  swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

  let mut can_make = false;
  task::block_on(future::poll_fn(move |cx: &mut Context<'_>| {
    loop {
      match swarm.poll_next_unpin(cx) {
        Poll::Ready(Some(event)) => {
          if let SwarmEvent::NewListenAddr { address, .. } = event {
            println!("Listening on {}:{}", address, floodsub_topic.id());
            can_make = true;
          }
        }
        Poll::Ready(None) => return Poll::Ready(Ok(())),
        Poll::Pending => {
          let r = event_receiver.try_recv();
          if r.is_ok() {
            let r = r.unwrap();
            let mut splited = r.split(" ");
            match splited.nth(0).unwrap() {
              "ban" => {
                // Messenger found fraudulent peer. ban this peer's ID.
                swarm.ban_peer_id(PeerId::from_str(splited.nth(1).unwrap()).unwrap())
              }
              _ => {}
            }
          }
          if can_make {
            let recieved = miner_receiver.try_recv();
            if recieved.is_ok() {
              swarm
                .behaviour_mut()
                .report_mine(floodsub_topic.clone(), &recieved.unwrap())
            }
          }
          break;
        }
      }
    }
    Poll::Pending
  }))
}
