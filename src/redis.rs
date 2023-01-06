

use redis::{IntoConnectionInfo, Msg, RedisResult, ConnectionInfo};
use crate::{error::ApplicationResult, messages::{SelectionMessage, SimpleControllerInput}, sync::MessageProducer};
use serde::Deserialize;
use tokio::select;
use futures_util::StreamExt;

#[derive(Clone)]
pub struct CreationInfo {
    pub redis_addr: String,
    pub redis_password: Option<String>,
    pub redis_db: i64,
    pub redis_channel: String,
}

impl IntoConnectionInfo for CreationInfo {
    fn into_connection_info(self) -> RedisResult<ConnectionInfo> {
        let mut info: redis::ConnectionInfo = self.redis_addr.as_str().into_connection_info()?;
        info.redis.db = self.redis_db;
        info.redis.password = self.redis_password;
        Ok(info)
    }
}


#[derive(Deserialize, Debug)]
struct SelectionMessageWire {
    model: String,
    actions: Option<Vec<f32>>,
}

fn parse_message(msg: Msg) -> Option<SelectionMessage> {
    // Catches errors and reports them to tracing.
    let msg = match msg.get_payload::<String>() {
        Ok(msg) => msg,
        Err(err) => {
            tracing::error!("Failed to get payload from message, error occurred: {}", err);
            return None;
        }
    };

    let msg = match serde_json::from_str::<SelectionMessageWire>(&msg) {
        Ok(msg) => msg,
        Err(err) => {
            tracing::error!("Failed to parse message: {:?}, error occurred: {}", msg, err);
            return None;
        }
    };

    let actions = match msg.actions {
        Some(actions) if actions.len() == 8 => Some(SimpleControllerInput {
            throttle: actions[0],
            steer: actions[1],
            pitch: actions[2],
            yaw: actions[3],
            roll: actions[4],
            jump: actions[5] > 0.5,
            boost: actions[6] > 0.5,
            handbrake: actions[7] > 0.5,
        }),
        None => None,
        _ => {
            tracing::error!("Failed to parse message: {:?}, actions is not 8 elements long", msg);
            return None
        }
    };

    Some(SelectionMessage {
        model: msg.model,
        actions,
    })
}

pub async fn create_redis_stream(conn_info: CreationInfo, tx: MessageProducer) -> ApplicationResult {
    let client = redis::Client::open(conn_info.clone())?;
    let redis = client.get_async_connection().await?;
    let mut pubsub = redis.into_pubsub();
    pubsub.subscribe(conn_info.redis_channel).await?;
    let message_stream = pubsub.into_on_message();

    tokio::pin!(message_stream);

    loop {
        select! {
            Some(msg) = message_stream.next() => {
                let _ = match parse_message(msg) {
                    Some(msg) => tx.send(msg),
                    None => continue,
                };
            }
            else => {
                break;
            }
        }
    }

    Ok(())
}