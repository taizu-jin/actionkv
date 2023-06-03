use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    pub file: String,
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Retrieves the value at `key` from the store
    Get(KArgs),
    /// Adds a key-value pair to the store
    Insert(KVArgs),
    /// Removes a key-value pair from store
    Delete(KArgs),
    /// Replaces an old value with a new one
    Update(KVArgs),
}

#[derive(Args)]
pub struct KArgs {
    pub key: String,
}

#[derive(Args)]
pub struct KVArgs {
    pub key: String,
    pub value: String,
}
