use std::net::SocketAddr;
use std::sync::Mutex;

use askama::Template;
use itertools::izip;
use rocket::State;
use rocket_contrib::serve::StaticFiles;
use romanize::Romanizer;

use crate::mpd_client::MpdClient;

pub fn launch(mpd_address: &SocketAddr) {
    rocket::ignite()
        .manage(Mutex::new(MpdClient::connect(*mpd_address).unwrap()))
        .manage(Romanizer::new().unwrap())
        .mount("/", routes![index, next, test])
        .mount("/", StaticFiles::from("static"))
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

#[get("/test")]
fn test() -> HelloTemplate<'static> {
    HelloTemplate { name: "testing" }
}

#[derive(Template)]
#[template(path = "hello.html")]
struct HelloTemplate<'a> {
    name: &'a str,
}
