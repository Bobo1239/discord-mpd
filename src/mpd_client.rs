use std::net::ToSocketAddrs;

use mpd::error::Error;
use mpd::{Client, Song};

pub struct MpdClient<A: ToSocketAddrs = String>(Client, A);

impl<A: ToSocketAddrs> MpdClient<A> {
    pub fn connect(address: A) -> Result<MpdClient<A>, Error> {
        Ok(MpdClient(Client::connect(&address)?, address))
    }

    fn do_op<T, F: Fn(&mut Client) -> Result<T, Error>>(&mut self, f: F) -> Result<T, Error> {
        if self.0.ping().is_err() || self.0.ping().is_err() {
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

    pub fn queue(&mut self) -> Result<Vec<Song>, Error> {
        self.do_op(Client::queue)
    }
}
