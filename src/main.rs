#![allow(dead_code)]
#![allow(mutable_transmutes)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(unused_assignments)]
#![allow(unused_mut)]
#![allow(clippy::missing_safety_doc)]
#![feature(c_variadic)]
#![feature(extern_types)]

use clap::{Parser, Subcommand};

extern crate libc;
pub mod client;
pub mod common;
pub mod datagram;
pub mod extc;
pub mod server;
pub mod util;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Client(client::Parameter),
    Server(server::Parameter),
}

pub fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Client(parameter) => {
            client::main::interactive(parameter);
        }
        Commands::Server(parameter) => unsafe {
            server::main::serve(parameter);
        },
    }
}
