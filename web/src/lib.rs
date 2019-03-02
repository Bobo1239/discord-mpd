#![feature(proc_macro_hygiene, decl_macro)]

use std::sync::Mutex;

use askama::Template;
use itertools::izip;
use rocket::{get, routes, State};
use rocket_contrib::serve::StaticFiles;

use shared::config::Config;
use shared::mpd_client::MpdClient;
use shared::romanize::Romanizer;

pub fn launch(config: &Config) {
    rocket::ignite()
        .manage(Mutex::new(MpdClient::connect(config.mpd_address).unwrap()))
        .manage(Romanizer::new().unwrap())
        .mount("/", routes![index, next, switch])
        .mount(
            "/",
            StaticFiles::from(concat!(env!("CARGO_MANIFEST_DIR"), "/static")),
        )
        .launch();
}

#[get("/next")]
fn next(mpd: State<Mutex<MpdClient>>) -> &str {
    let mut mpd = mpd.lock().unwrap();
    mpd.next().unwrap();
    "Skipped"
}

#[get("/switch/<place>")]
fn switch(mpd: State<Mutex<MpdClient>>, place: u32) -> &str {
    let mut mpd = mpd.lock().unwrap();
    mpd.switch(place).unwrap();
    "Switched"
}

#[get("/")]
fn index(mpd: State<Mutex<MpdClient>>, romanizer: State<Romanizer>) -> IndexTemplate {
    let songs = mpd.lock().unwrap().queue().unwrap();

    // TODO: Cache romanization somehow...
    let titles: Vec<_> = songs
        .iter()
        .map(|s| {
            s.title
                .as_ref()
                .cloned()
                .unwrap_or_else(|| "[missing title]".to_string())
        })
        .collect();
    let titles_romanized: Vec<_> = romanizer
        .romanize(&titles.join("\n"))
        .split('\n')
        .map(|s| s.to_string())
        .collect();

    assert_eq!(songs.len(), titles.len());
    assert_eq!(songs.len(), titles_romanized.len());

    // TODO: Use a map or something... (also see TODO above)
    let albums: Vec<_> = songs
        .iter()
        .map(|s| {
            s.tags
                .get("Album")
                .cloned()
                .unwrap_or_else(|| "[missing album]".to_string())
        })
        .collect();
    let albums_romanized: Vec<_> = romanizer
        .romanize(&albums.join("\n"))
        .split('\n')
        .map(|s| s.to_string())
        .collect();

    assert_eq!(songs.len(), albums.len());
    assert_eq!(songs.len(), albums_romanized.len());

    let song_infos: Vec<SongInfo> =
        izip!(songs, titles, titles_romanized, albums, albums_romanized)
            .map(
                |(song, title, title_romanized, album, album_romanized)| SongInfo {
                    place: song.place.unwrap().id.0 - 1,
                    title,
                    title_romanized,
                    album,
                    album_romanized,
                },
            )
            .collect();

    IndexTemplate {
        song_infos: song_infos,
    }
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    song_infos: Vec<SongInfo>,
}

struct SongInfo {
    place: u32,
    title: String,
    title_romanized: String,
    album: String,
    album_romanized: String,
}
