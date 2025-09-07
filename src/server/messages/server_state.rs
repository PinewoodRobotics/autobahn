use std::sync::Arc;

use log::debug;

use crate::{
  message::ServerStateMessage,
  server::{peer::Peer, Server},
};

impl Server {
  pub async fn handle_server_state(self: Arc<Self>, server_state_message: ServerStateMessage) {
    debug!(
      "Handling server state message from UUID: {}",
      server_state_message.uuid
    );

    let peer_arc = self
      .peers_map
      .get_by_id(&server_state_message.uuid)
      .await
      .unwrap();
    let mut peer_gotten_from = peer_arc.write().await;

    peer_gotten_from.update_topics(server_state_message.topics);
  }
}
