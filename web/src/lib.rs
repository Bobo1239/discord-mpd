#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;

use std::sync::Mutex;

use askama::Template;
use itertools::izip;
use rocket::State;
use rocket_contrib::serve::StaticFiles;

use shared::config::Config;
use shared::mpd_client::MpdClient;
use shared::romanize::Romanizer;

pub fn launch(config: &Config) {
    rocket::ignite()
        .manage(Mutex::new(MpdClient::connect(config.mpd_address).unwrap()))
        .manage(Romanizer::new().unwrap())
        .mount("/", routes![index, next, test])
        .mount("/", StaticFiles::from(concat!(env!("CARGO_MANIFEST_DIR"), "/static")))
        .launch();
}

#[get("/")]
fn index(mpd: State<Mutex<MpdClient>>, romanizer: State<Romanizer>) -> String {
    let songs = mpd.lock().unwrap().queue().unwrap();

    let titles: Vec<_> = songs
        .iter()
        .map(|s| {
            s.title
                .as_ref()
                .cloned()
                .unwrap_or_else(|| "[missing title]".to_string())
        })
        .collect();
    let romanized_titles = romanizer.romanize(&titles.join("\n"));
    let romanized_titles: Vec<_> = romanized_titles.split('\n').collect();

    assert_eq!(songs.len(), titles.len());
    assert_eq!(songs.len(), romanized_titles.len());
    let mut output = String::new();
    for (song, title, romanized) in izip!(songs, titles, romanized_titles) {
        output += &format!("{} {}   |  {}\n", song.place.unwrap().id, title, romanized,);
    }
    output
}

#[get("/next")]
fn next(mpd: State<Mutex<MpdClient>>) -> &str {
    let mut mpd = mpd.lock().unwrap();
    mpd.next().unwrap();
    "Skipped"
}

#[get("/play")]
fn test() -> PlayTemplate<'static> {
    PlayTemplate { name: "testing" }
}

#[derive(Template)]
#[template(path = "index.html")]
struct PlayTemplate<'a> {
    name: &'a str,
}
