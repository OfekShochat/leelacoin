use async_std::task;
use futures::{future, prelude::*};
use libp2p::{
  floodsub::{self, Floodsub, FloodsubEvent},
  identity,
  mdns::{Mdns, MdnsConfig, MdnsEvent},
  swarm::{NetworkBehaviourEventProcess, SwarmEvent},
  Multiaddr, NetworkBehaviour, PeerId, Swarm,
};

use serde_json::json;

use crate::blockchain;

use std::{
  error::Error,
  task::{Context, Poll},
};

#[derive(NetworkBehaviour)]
struct MyBehaviour {
  floodsub: Floodsub,
  mdns: Mdns,
}

impl NetworkBehaviourEventProcess<FloodsubEvent> for MyBehaviour {
  // Called when `floodsub` produces an event.
  fn inject_event(&mut self, message: FloodsubEvent) {
    if let FloodsubEvent::Message(message) = message {
      println!(
        "Received: '{:?}' from {:?}",
        String::from_utf8_lossy(&message.data),
        message.source
      );
    }
  }
}

impl NetworkBehaviourEventProcess<MdnsEvent> for MyBehaviour {
  // Called when `mdns` produces an event.
  fn inject_event(&mut self, event: MdnsEvent) {
    match event {
      MdnsEvent::Discovered(list) => {
        for (peer, _) in list {
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

#[async_std::main]
pub async fn main() -> Result<(), Box<dyn Error>> {
  let local_key = identity::Keypair::generate_ed25519();
  let local_peer_id = PeerId::from(local_key.public());
  println!("Local peer id: {:?}", local_peer_id);

  // Set up a an encrypted DNS-enabled TCP Transport over the Mplex and Yamux protocols
  let transport = libp2p::development_transport(local_key).await?;

  // Create a Floodsub topic
  let floodsub_topic = floodsub::Topic::new("chat");
  // Create a Swarm to manage peers and events
  let mut swarm = {
    let mdns = task::block_on(Mdns::new(MdnsConfig::default()))?;
    let mut behaviour = MyBehaviour {
      floodsub: Floodsub::new(local_peer_id),
      mdns,
    };

    behaviour.floodsub.subscribe(floodsub_topic.clone());
    Swarm::new(transport, behaviour, local_peer_id)
  };

  swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

  let mut bc = blockchain::Chain::new();

  task::block_on(future::poll_fn(move |cx: &mut Context<'_>| {
    loop {
      match swarm.poll_next_unpin(cx) {
        Poll::Ready(Some(event)) => {
          if let SwarmEvent::NewListenAddr { address, .. } = event {
            println!("Listening on {:?}", address);
          }
        }
        Poll::Ready(None) => return Poll::Ready(Ok(())),
        Poll::Pending => break,
      }
    }
    bc.add_block("poop".to_string(), "poopoo".to_string(), 5);
    let l = bc.last();
    swarm.behaviour_mut().floodsub.publish(
      floodsub_topic.clone(),
      json!({
        "poop": l.summary
      })
      .to_string()
      .as_bytes(),
    );
    Poll::Pending
  }))
}
