use clap::Parser;

#[derive(Parser)]
#[command(name = "auris")]
pub struct Mode {
    #[arg(long, default_value = "api")]
    pub execution_mode: String, // "api" or "worker"
}
