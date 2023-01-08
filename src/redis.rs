

use redis::{IntoConnectionInfo, Msg, RedisResult, ConnectionInfo, aio::Connection, AsyncCommands};
use crate::{error::ApplicationResult, messages::{SelectionMessage, SimpleControllerInput, Message, StatisticsMessage}, sync::MessageProducer};
use serde::Deserialize;
use tokio::select;
use futures_util::StreamExt;

#[derive(Clone)]
pub struct CreationInfo {
    pub redis_addr: String,
    pub redis_password: Option<String>,
    pub redis_db: i64,
    pub redis_channel: String,
    pub redis_stats_prefix: String,
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

fn stat_key_name(prefix: &str, name: &str) -> String {
    // name, to_lowercase, replace spaces with underscores, prepend prefix
    format!("{}_{}", prefix, name.to_lowercase().replace(" ", "_"))
}

async fn open_connection(conn_info: impl IntoConnectionInfo) -> ApplicationResult<Connection> {
    let client = redis::Client::open(conn_info)?;
    Ok(client.get_async_connection().await?)
}

async fn update_stat(stat_prefix: &str, selection_message: &SelectionMessage, connection: &mut Connection) -> ApplicationResult<StatisticsMessage> {
    let stat_name = selection_message.model.to_string();
    let stat_key = stat_key_name(stat_prefix, &stat_name);
    let stat: u64 = connection.incr(stat_key, 1).await?;
    Ok(StatisticsMessage {
        model: stat_name,
        counts: stat,
    })
}

pub async fn create_redis_stream(conn_info: CreationInfo, tx: MessageProducer) -> ApplicationResult {
    let make_connection = || open_connection(conn_info.clone());
    let mut stat_writer = make_connection().await?;
    let mut pubsub = make_connection().await?.into_pubsub();
    pubsub.subscribe(conn_info.redis_channel.to_string()).await?;
    let message_stream = pubsub.into_on_message();

    tokio::pin!(message_stream);

    loop {
        select! {
            Some(msg) = message_stream.next() => {
                let _ = match parse_message(msg) {
                    Some(msg) => {
                        let stat = update_stat(&conn_info.redis_stats_prefix, &msg, &mut stat_writer).await?;
                        // Notes from broadcast::channel
                        // A return value of Ok does not mean that the sent value will be observed by all or any of the active [Receiver] handles.
                        // [Receiver] handles may be dropped before receiving the sent message.
                        // A return value of Err does not mean that future calls to send will fail.
                        // New [Receiver] handles may be created by calling [subscribe].
                        // Thus: We don't care about the result of the send.
                        let _ = tx.send(Message::Selection(msg));
                        let _ = tx.send(Message::Statistics(stat));
                    }
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