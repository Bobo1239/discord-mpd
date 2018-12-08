#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;

mod helper;
mod mpd_client;

use std::env;
use std::fs::File;
use std::net::SocketAddr;
use std::sync::Mutex;

use discord::model::{ChannelId, Event};
use discord::voice::create_pcm_source;
use discord::{Connection, Discord, State};
use dotenv::dotenv;
use failure::Error;
use itertools::izip;
use log::*;
use mpd::Song;
use rocket_contrib::serve::StaticFiles;
use romanize::Romanizer;

use crate::helper::*;
use crate::mpd_client::MpdClient;

// TODO: error handling...

const COMMAND: &str = "!r";

#[get("/")]
#[allow(clippy::needless_pass_by_value)]
fn index(mpd: rocket::State<Mutex<MpdClient>>, romanizer: rocket::State<Romanizer>) -> String {
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
#[allow(clippy::needless_pass_by_value)]
fn next(mpd: rocket::State<Mutex<MpdClient>>) -> &str {
    let mut mpd = mpd.lock().unwrap();
    mpd.next().unwrap();
    "Skipped"
}

fn launch_rocket(mpd_address: SocketAddr) {
    std::thread::spawn(move || {
        rocket::ignite()
            .manage(Mutex::new(MpdClient::connect(mpd_address).unwrap()))
            .manage(Romanizer::new().unwrap())
            .mount("/", routes![index, next])
            .mount("/", StaticFiles::from("static"))
            .launch();
    });
}

fn main() {
    dotenv().ok();
    let token = &env::var("DISCORD_TOKEN")
        .expect("DISCORD_TOKEN not set! Did you forget to create a .env file?");
    let mpd_address = env::var("MPD_ADDRESS")
        .unwrap_or_else(|_| "127.0.0.1:6600".to_string())
        .parse()
        .expect("Failed to parse mpd address!");

    let mut mpd = MpdClient::connect(mpd_address).unwrap();

    launch_rocket(mpd_address);

    let discord = Discord::from_bot_token(token).expect("login failed");
    let romanizer = Romanizer::new().unwrap();

    let (mut connection, ready) = discord.connect().expect("connect failed");
    info!(
        "\"{}\" is serving {} servers",
        ready.user.username,
        ready.servers.len()
    );
    let mut state = State::new(ready);
    connection.sync_calls(&state.all_private_channels());

    loop {
        let event = match connection.recv_event() {
            Ok(event) => event,
            Err(err) => {
                warn!("Received error: {:?}", err);
                match err {
                    discord::Error::WebSocket(..) => {
                        // Handle the websocket connection being dropped
                        let (new_connection, ready) = discord.connect().expect("connect failed");
                        connection = new_connection;
                        state = State::new(ready);
                        info!("Discord reconnected successfully.");
                    }
                    discord::Error::Closed(..) => {
                        warn!("Discord connection closed!");
                        return;
                    }
                    _ => {}
                }
                continue;
            }
        };
        state.update(&event);
        handle_event(
            &event,
            &state,
            &mut connection,
            &discord,
            &mut mpd,
            &romanizer,
        );
    }
}

fn handle_event(
    event: &Event,
    state: &State,
    connection: &mut Connection,
    discord: &Discord,
    mpd: &mut MpdClient,
    romanizer: &Romanizer,
) -> Option<Error> {
    let send_message =
        |channel: ChannelId, message: &str| discord.send_message(channel, message, "", false).err();

    match *event {
        Event::MessageCreate(ref message)
            if message.author.id != state.user().id && message.content.starts_with(COMMAND) =>
        {
            // safeguard: stop if the message is from us

            let voice_channel = state.find_voice_user(message.author.id);
            let arguments = message.content.split(' ').skip(1).collect::<Vec<_>>();

            match *arguments.get(0).unwrap_or(&"") {
                "pause" => mpd.toggle_pause().unwrap(),
                "next" => mpd.next().unwrap(),
                "prev" => mpd.prev().unwrap(),
                // TODO: rework using a proxy struct wrapping the PCM stream so we can adjust the volume
                // "vol" => {
                //     if let Some(vol) = arguments.get(1) {
                //         if let Ok(mut vol) = vol.parse() {
                //             if vol > 100 {
                //                 vol = 100;
                //             }
                //             if vol < 0 {
                //                 vol = 0;
                //             }
                //             mpd.volume(vol).unwrap();
                //         }
                //     } else {
                //         send_message(
                //             message.channel_id,
                //             &format!("Current volume is {}.", mpd.status().unwrap().volume),
                //         );
                //     }
                // }
                "quit" => {
                    if let Some((sid, _)) = voice_channel {
                        connection.drop_voice(sid)
                    }
                }
                "info" => {
                    if let Some(song) = mpd.currentsong().unwrap() {
                        send_message(message.channel_id, &format_mpd_singinfo(&song, romanizer));
                    } else {
                        send_message(message.channel_id, "No song currently playing!");
                    }
                }
                _ => {
                    if let Some((server_id, channel_id)) = voice_channel {
                        let voice = connection.voice(server_id);
                        voice.set_deaf(true);
                        voice.connect(channel_id);
                        voice.play(create_pcm_source(
                            true,
                            File::open("/tmp/mpd_bot.fifo").unwrap(),
                        ));
                    } else {
                        send_message(
                            message.channel_id,
                            "You must be in a voice channel to invite me.",
                        );
                    }
                }
            }
        }
        Event::VoiceStateUpdate(server_id, _) => {
            // If someone moves/hangs up, and we are in a voice channel,
            if let Some(cur_channel) = connection.voice(server_id).current_channel() {
                // and our current voice channel is empty, disconnect from voice
                match server_id {
                    Some(server_id) => {
                        if let Some(srv) = state.servers().iter().find(|srv| srv.id == server_id) {
                            if srv
                                .voice_states
                                .iter()
                                .filter(|vs| vs.channel_id == Some(cur_channel))
                                .count()
                                <= 1
                            {
                                connection.voice(Some(server_id)).disconnect();
                            }
                        }
                    }
                    None => {
                        if let Some(call) = state.calls().get(&cur_channel) {
                            if call.voice_states.len() <= 1 {
                                connection.voice(server_id).disconnect();
                            }
                        }
                    }
                }
            }
        }
        _ => {} // discard other events
    }
    None
}

fn format_mpd_singinfo(song: &Song, romanizer: &Romanizer) -> String {
    fn add_romanization(mut input: String, romanizer: &Romanizer) -> String {
        let romanized = romanizer.romanize(&input);
        if romanized != input {
            input.push_str(&format!(" ({})", romanized));
        }
        input
    }

    let mut info = "```\n".to_string();

    let title = if let Some(ref title) = song.title {
        add_romanization(title.clone(), romanizer)
    } else {
        song.file.clone()
    };
    let artist = song
        .tags
        .get("Artist")
        .map(|s| add_romanization(s.clone(), romanizer));
    let album = song
        .tags
        .get("Album")
        .map(|s| add_romanization(s.clone(), romanizer));

    info += &format!("Title:    {}\n", title,);
    if let Some(artist) = artist {
        info += &format!("Artist:   {}\n", artist)
    }
    if let Some(album) = album {
        info += &format!("Album:    {}\n", album)
    }
    if let Some(duration) = song.duration {
        info += &format!(
            "Duration: {}\n",
            format_duration(&duration.to_std().unwrap())
        );
    }

    info += "```";
    info
}
