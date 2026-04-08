use clap::{Parser, ValueEnum};

#[derive(Clone, Debug, ValueEnum)]
pub enum ExecutionMode {
    Api,
    Worker,
}

#[derive(Parser)]
#[command(name = "auris")]
pub struct Mode {
    #[arg(long, value_enum, default_value_t = ExecutionMode::Api)]
    pub execution_mode: ExecutionMode, // "api" or "worker"
}
