use std::{env::VarError::{self}};

#[derive(Debug, thiserror::Error)]
pub enum ApplicationError {
    #[error("Tried to read an environment variable, but it wasn't unicode: {0}")]
    EnvVarNotUnicode(#[from] VarError),
    #[error("Can't parse environment variable '{0}'")]
    EnvParseError(String),
    #[error("An error occured while parsing the web server bind address: {0}")]
    BindParseError(#[from] std::net::AddrParseError),
    #[error("An error occurred in the webserver runtime (hyper): {0}")]
    HyperServerError(#[from] hyper::Error),
    #[error("An error occurred in the redis communication channel: {0}")]
    RedisError(#[from] redis::RedisError),
    #[error("An error occurred while attempting to parse a JSON message: {0}")]
    SerdeDeserializeError(#[from] serde_json::Error),
    #[error("An error occurred while transporting messages: {0}")]
    BroadcastChannelSendError(#[from] tokio::sync::broadcast::error::SendError<crate::messages::SelectionMessage>),
}

pub type ApplicationResult<T = ()> = Result<T, ApplicationError>;