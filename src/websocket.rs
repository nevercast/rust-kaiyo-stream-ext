use axum::{
    extract::ws::{WebSocketUpgrade, WebSocket, Message},
    response::Response
};
use tokio::sync::broadcast::error::RecvError;

use crate::sync::MessageConsumer;

pub async fn ws_upgrade(rx: MessageConsumer, ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(|socket| handle_socket(rx, socket))
}

async fn handle_socket(mut rx: MessageConsumer, mut socket: WebSocket) {
    loop {
        tokio::select! {
            ws_msg = socket.recv() => match ws_msg {
                Some(msg) => match msg {
                    Ok(Message::Close(_)) => {
                        tracing::info!("Websocket closing...");
                    }
                    Ok(Message::Text(_)) => {
                        tracing::info!("WebSocket client sent us a message, but we don't care about it");
                    }
                    Ok(Message::Binary(_)) => {
                        tracing::info!("WebSocket client sent us a binary message, but we don't care about it");
                    }
                    Ok(_) => {}
                    Err(err) => {
                        tracing::error!("Error occurred in websocket connection: {}", err);
                        match socket.close().await {
                            Ok(_) => {}
                            Err(err) => {
                                tracing::error!("Error closing websocket: {}. You can probably ignore this?", err);
                            }
                        }
                        break;
                    }
                },
                None => {
                    tracing::info!("Websocket closed");
                    break;
                }
            },
            redis_msg = rx.recv() => match redis_msg {
                Ok(msg) => {
                    let msg = serde_json::to_string(&msg).unwrap();
                    tracing::info!("Sending message to websocket: {}", msg);
                    socket.send(Message::Text(msg)).await.unwrap();
                }
                Err(RecvError::Lagged(_)) => {
                    tracing::info!("Redis message service is lagged, this is caused by the websocket being slow");
                }
                Err(RecvError::Closed) => {
                    tracing::info!("Redis message service closed, all the websockets should be closed now");
                    break;
                }
            }
        }
    }
}