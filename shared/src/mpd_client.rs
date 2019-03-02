use std::net::SocketAddr;

use mpd::error::Error;
use mpd::{Client, Song};

pub struct MpdClient(Client, SocketAddr);

impl MpdClient {
    pub fn connect(address: SocketAddr) -> Result<MpdClient, Error> {
        Ok(MpdClient(Client::connect(&address)?, address))
    }

    fn client(&mut self) -> &mut Client {
        // TODO: Not sure why we're checking twice...
        if self.0.ping().is_err() || self.0.ping().is_err() {
            self.0 = Client::connect(&self.1).unwrap();
        }
        &mut self.0
    }

    pub fn toggle_pause(&mut self) -> Result<(), Error> {
        self.client().toggle_pause()
    }

    pub fn next(&mut self) -> Result<(), Error> {
        self.client().next()
    }

    pub fn prev(&mut self) -> Result<(), Error> {
        self.client().prev()
    }

    pub fn switch(&mut self, queue_place: u32) -> Result<(), Error> {
        self.client().switch(queue_place)
    }

    pub fn current_song(&mut self) -> Result<Option<Song>, Error> {
        self.client().currentsong()
    }

    pub fn queue(&mut self) -> Result<Vec<Song>, Error> {
        self.client().queue()
    }
}
