use clap::Parser;
use log::debug;

use client::run;

#[derive(Parser, Debug)]
#[command(version, about, long_about=None)]
struct Args {
    /// Address of the chat server
    #[arg(short = 'a', long, default_value_t = String::from("127.0.0.1"))]
    chat_address: String,

    /// Port number for the chat server
    #[arg(short = 'p', long, default_value_t = 8080)]
    chat_port: u32,
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let args = Args::parse();

    debug!("Chat server is at address {}", args.chat_address);
    debug!("Chat server port is {}", args.chat_port);

    let connect = format!("{}:{}", args.chat_address, args.chat_port);

    run(connect).await;
}
