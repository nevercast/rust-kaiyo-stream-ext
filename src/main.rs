use std::{net::SocketAddr, str::FromStr};
use std::env::VarError;
use axum::extract::WebSocketUpgrade;

use error::{ApplicationError, ApplicationResult};

use axum::{
    routing::get,
    Router
};
use clap::Parser;

use tokio::task::JoinError;
use crate::redis::CreationInfo;
use sync::{MessageProducer, MessageConsumer, MessageConsumerFactory};

use crate::websocket::ws_upgrade;
use crate::{static_service::create_static_service, args::Args};

mod args;
mod sync;
mod error;
mod redis;
mod messages;
mod websocket;
mod static_service;

fn get_arg_from_env<T: FromStr>(arg: &str) -> ApplicationResult<Option<T>>
{
    match std::env::var(arg) {
        Ok(val) => val.parse().map(Some).map_err(|_| ApplicationError::EnvParseError(arg.to_string())),
        Err(err) => match err {
            VarError::NotPresent => Ok(None),
            VarError::NotUnicode(_) => Err(ApplicationError::EnvVarNotUnicode(err)),
        }
    }
}

async fn spawn_redis(args: Args, tx: MessageProducer) -> ApplicationResult {
    let redis_addr = match args.redis {
        Some(redis_addr) => redis_addr,
        None => get_arg_from_env("RKSE_REDIS_ADDR")?.unwrap_or_else(|| "redis://127.0.0.1:6379".to_string()),
    };

    let redis_password = match args.redis_password {
        Some(redis_password) => redis_password,
        None => get_arg_from_env("RKSE_REDIS_PASSWORD")?.unwrap_or_default(),
    };

    let redis_db = match args.redis_db {
        Some(redis_db) => redis_db,
        None => get_arg_from_env("RKSE_REDIS_DB")?.unwrap_or(0),
    };

    let redis_channel = match args.redis_channel {
        Some(redis_channel) => redis_channel,
        None => get_arg_from_env("RKSE_REDIS_CHANNEL")?.unwrap_or_else(|| "on_model_selection".to_string()),
    };

    let redis_stats_prefix = match args.redis_stats_prefix {
        Some(redis_stats_prefix) => redis_stats_prefix,
        None => get_arg_from_env("RKSE_REDIS_STATS_PREFIX")?.unwrap_or_else(|| "selector_stat".to_string()),
    };

    let creation_info = CreationInfo {
        redis_addr,
        redis_db,
        redis_channel,
        redis_password: match redis_password.as_str() {
            "" => None,
            _ => Some(redis_password),
        },
        redis_stats_prefix
    };

    redis::create_redis_stream(creation_info, tx).await?;

    Ok(())
}

async fn spawn_web(args: Args, rx_factory: MessageConsumerFactory) -> ApplicationResult {
    // parse bind address
    let addr: SocketAddr = match args.bind {
        Some(bind) => bind,
        None => get_arg_from_env("RKSE_BIND")?.unwrap_or_else(|| "127.0.0.1:3000".to_string()),
    }.parse()?;

    let static_path = match args.static_path {
        Some(static_path) => static_path,
        None => get_arg_from_env("RKSE_STATIC_PATH")?.unwrap_or_else(|| "static".to_string()),
    };

    // create static service
    let static_service = create_static_service(static_path);

    let app = Router::new()
        .route("/ws", get(move |ws: WebSocketUpgrade| ws_upgrade(rx_factory.create(), ws)))
        .fallback_service(static_service);

    tracing::info!("Listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

fn print_end_of_service_summary(result: Result<ApplicationResult, JoinError>) {
    match result {
        Ok(Ok(_)) => tracing::info!("Service ended successfully"),
        Ok(Err(err)) => tracing::error!("Service ended with error: {}", err),
        Err(err) => tracing::error!("Service ended with error: {}", err),
    }
}


// args:
// -b, --bind <bind>    [default: 127.0.0.1:3000]
// -s, --static <static>    [default: static]
#[tokio::main]
async fn main() -> ApplicationResult {
    // initialize tracing
    tracing_subscriber::fmt::init();

    // On Windows we need to enable ansi codes for colored output
    #[cfg(windows)]
    ansi_term::enable_ansi_support().unwrap();

    // parse args
    let args = Args::parse();

    // Broadcast channel for redis <-> web service communication
    let (tx, _): (MessageProducer, MessageConsumer) = tokio::sync::broadcast::channel(5);
    let rx_factory = MessageConsumerFactory::new(&tx);

    // spawn web server
    let web_service = spawn_web(args.clone(), rx_factory);
    let web_service = tokio::spawn(web_service);

    // spawn redis client
    let redis_client = spawn_redis(args, tx);
    let redis_client = tokio::spawn(redis_client);

    // Wait for either the web service or the redis client to exit
    tokio::select! {
        web_result = web_service => {
            tracing::info!("Web service exited, killing process.");
            print_end_of_service_summary(web_result);
            Ok(())
        },
        redis_result = redis_client => {
            tracing::info!("Redis client exited, killing process.");
            print_end_of_service_summary(redis_result);
            Ok(())
        }
    }
}