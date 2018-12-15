use std::env;

use std::net::SocketAddr;
use std::net::ToSocketAddrs;

pub struct Config {
    pub mpd_address: SocketAddr,
    pub discord_token: String,
}

impl Config {
    pub fn from_env() -> Config {
        let discord_token = env::var("DISCORD_TOKEN").expect("DISCORD_TOKEN is not set");
        let mpd_address: SocketAddr = env::var("MPD_ADDRESS")
            .expect("MPD_ADDRESS is not set")
            .to_socket_addrs()
            .expect("parsing MPD_ADDRESS failed")
            .next()
            .unwrap();

        Config {
            mpd_address,
            discord_token,
        }
    }
}
