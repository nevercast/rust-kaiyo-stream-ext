use clap::Parser;

#[derive(Parser, Clone)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Bind address, default: 127.0.0.1:3000
    #[arg(short, long)]
    pub bind: Option<String>,

    /// Static file path, default: static
    #[arg(short, long)]
    pub static_path: Option<String>,

    /// "--redis", default: redis://127.0.0.1:6379
    #[arg(short, long)]
    pub redis: Option<String>,

    /// "--pass", default: None
    /// If you want to use password, you must set this value
    #[arg(long="pass")]
    pub redis_password: Option<String>,

    /// Redis database, "--db", default: 0
    #[arg(long="db")]
    pub redis_db: Option<i64>,

    /// Redis pubsub "--chan", default: on_model_selection
    #[arg(long="chan")]
    pub redis_channel: Option<String>,

    /// Redis stats prefix, "--stats-prefix", default: selector_stat
    #[arg(long="stats-prefix")]
    pub redis_stats_prefix: Option<String>,
}