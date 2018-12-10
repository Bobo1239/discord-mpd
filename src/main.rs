#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;

mod discord;
mod helper;
mod mpd_client;
mod web;

use std::net::SocketAddr;
use std::net::ToSocketAddrs;
use std::{env, thread};

use dotenv::dotenv;

// TODO: error handling...

fn main() {
    dotenv().unwrap();

    let discord_token = env::var("DISCORD_TOKEN")
        .expect("DISCORD_TOKEN not set! Did you forget to create a .env file?");
    let mpd_address: SocketAddr = env::var("MPD_ADDRESS")
        .unwrap_or_else(|_| "127.0.0.1:6600".to_string())
        .to_socket_addrs()
        .expect("Failed to parse mpd address!")
        .next()
        .unwrap();

    let web = thread::spawn(move || {
        web::launch(&mpd_address);
    });
    let discord = thread::spawn(move || {
        discord::launch(&mpd_address, &discord_token);
    });

    web.join().unwrap();
    discord.join().unwrap();
}
