use async_std::task;
use futures::{future, prelude::*};
use libp2p::{
  floodsub::{self, Floodsub, FloodsubEvent, Topic},
  identity,
  mdns::{Mdns, MdnsConfig, MdnsEvent},
  swarm::{NetworkBehaviourEventProcess, SwarmEvent},
  NetworkBehaviour, PeerId, Swarm
};

use serde_json::json;

use crate::blockchain;
use crate::block::Block;

use std::{error::Error, sync::mpsc::{self, Receiver, Sender}, task::{Context, Poll}, time::Duration};

#[derive(NetworkBehaviour)]
struct Client {
  floodsub: Floodsub,
  mdns: Mdns,
}

impl Client {
  pub fn report_mine(&mut self, topic: Topic, block: &Block) {
    self.floodsub.publish(topic, json!({
      "report": "mined",
      "hash": block.summary,
      "data": block.data.get_string(),
      "previous": block.previous_summary,
      "nonce": block.nonce
    }).to_string());
  }
}

impl NetworkBehaviourEventProcess<FloodsubEvent> for Client {
  // Called when `floodsub` produces an event.
  fn inject_event(&mut self, message: FloodsubEvent) {
    if let FloodsubEvent::Message(message) = message {
      println!(
        "Received: '{:?}' from {:?}",
        String::from_utf8_lossy(&message.data),
        message.source
      );
      // if its a new block, validate it yourself and if you agree then send it to all peers.
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
  }
}

#[async_std::main]
pub async fn main() -> Result<(), Box<dyn Error>> {
  let local_key = identity::Keypair::generate_ed25519();
  let local_peer_id = PeerId::from(local_key.public());
  println!("Local peer id: {:?}", local_peer_id);

  // Set up a an encrypted DNS-enabled TCP Transport over the Mplex and Yamux protocols
  let transport = libp2p::development_transport(local_key).await?;

  // Create a Floodsub topic
  let floodsub_topic = floodsub::Topic::new("leelacoin");
  // Create a Swarm to manage peers and events
  let mut swarm = {
    let mdns = task::block_on(Mdns::new(MdnsConfig::default()))?;
    let mut behavior = Client {
      floodsub: Floodsub::new(local_peer_id),
      mdns,
    };

    behavior.floodsub.subscribe(floodsub_topic.clone());
    Swarm::new(transport, behavior, local_peer_id)
  };

  let (_sender, receiver) = mpsc::channel();
  let (sender2, receiver2) = mpsc::channel();
  std::thread::spawn(move || {
    process(receiver, sender2)
  });

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
        },
        Poll::Ready(None) => return Poll::Ready(Ok(())),
        Poll::Pending => {
          if can_make {
            let recieved = receiver2.recv_timeout(Duration::from_secs_f32(0.1));
            if recieved.is_ok() {
              swarm.behaviour_mut().report_mine(floodsub_topic.clone(), &recieved.unwrap())
            }
          }
          break
        }
      }
    }
    Poll::Pending
  }))
}
