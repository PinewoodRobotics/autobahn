use std::sync::Arc;

use bytes::Bytes;
use log::{debug, error};
use prost::Message;

use crate::{
  message::UnsubscribeMessage,
  server::{
    topics_map::Websock,
    Server,
  },
};

impl Server {
  pub async fn handle_unsubscribe(self: Arc<Self>, bytes: Bytes, ws_write: Arc<Websock>) {
    let unsubscribe_message = match UnsubscribeMessage::decode(bytes.clone()) {
      Ok(msg) => {
        debug!("Client unsubscribing from topic: {}", msg.topic);
        msg
      }
      Err(e) => {
        error!("Failed to decode unsubscribe message: {}", e);
        return;
      }
    };
    self
      .topics_map
      .remove_subscriber_from_topic(&unsubscribe_message.topic, &ws_write)
      .await;

    let message = self.topics_map.to_proto(self.uuid.clone()).await;

    self.peers_map.update_peers_self_state(message).await;
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{
    message::{MessageType, TopicMessage, UnsubscribeMessage},
    server::topics_map::Websock,
    util::{proto::build_proto_message, Address},
  };
  use futures_util::StreamExt;
  use tokio::net::TcpListener;

  async fn create_ws() -> Arc<Websock> {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
      let (socket, _) = listener.accept().await.unwrap();
      let ws_stream = tokio_tungstenite::accept_async(socket).await.unwrap();
      let (_write, mut read) = ws_stream.split();
      while let Some(_msg) = read.next().await {}
    });

    let socket = tokio::net::TcpStream::connect(addr).await.unwrap();
    let ws_stream = tokio_tungstenite::client_async("ws://localhost", socket)
      .await
      .unwrap()
      .0;
    let (write, _read) = ws_stream.split();
    Arc::new(Websock::new(write))
  }

  #[tokio::test]
  async fn test_handle_unsubscribe_removes_subscriber_and_topic() {
    let server = Server::new(Vec::new(), Address::from_str("127.0.0.1:0").unwrap());
    let ws = create_ws().await;

    // Subscribe first.
    let sub = build_proto_message(&TopicMessage {
      message_type: MessageType::Subscribe as i32,
      topic: "topic".to_string(),
    });
    server.clone().handle_subscribe(sub, ws.clone()).await;

    // Unsubscribe.
    let unsub = build_proto_message(&UnsubscribeMessage {
      message_type: MessageType::Unsubscribe as i32,
      topic: "topic".to_string(),
    });
    server.clone().handle_unsubscribe(unsub, ws.clone()).await;

    let topic_map = server.topics_map.get().await;
    assert!(!topic_map.contains_key("topic"));
  }

  #[tokio::test]
  async fn test_handle_unsubscribe_bad_bytes_is_ignored() {
    let server = Server::new(Vec::new(), Address::from_str("127.0.0.1:0").unwrap());
    let ws = create_ws().await;
    server
      .clone()
      .handle_unsubscribe(Bytes::from_static(b"bad"), ws)
      .await;
  }
}
