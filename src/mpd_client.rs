use std::net::ToSocketAddrs;

use mpd::{Client, Song};
use mpd::error::Error;

pub struct MpdClient<A: ToSocketAddrs>(Client, A);

impl<A: ToSocketAddrs> MpdClient<A> {
    pub fn connect(address: A) -> Result<MpdClient<A>, Error> {
        Ok(MpdClient(Client::connect(&address)?, address))
    }

    fn do_op<T, F: Fn(&mut Client) -> Result<T, Error>>(&mut self, f: F) -> Result<T, Error> {
        if self.0.status().is_err() || self.0.status().is_err() {
            self.0 = Client::connect(&self.1).unwrap();
        }
        f(&mut self.0)
    }

    pub fn toggle_pause(&mut self) -> Result<(), Error> {
        self.do_op(Client::toggle_pause)
    }

    pub fn next(&mut self) -> Result<(), Error> {
        self.do_op(Client::next)
    }

    pub fn prev(&mut self) -> Result<(), Error> {
        self.do_op(Client::prev)
    }

    pub fn currentsong(&mut self) -> Result<Option<Song>, Error> {
        self.do_op(Client::currentsong)
    }
}
