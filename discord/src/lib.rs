use std::fs::File;

use serenity::async_trait;
use serenity::client::ClientBuilder;
use serenity::model::gateway::GatewayIntents;
use serenity::model::prelude::*;
use serenity::prelude::*;
use songbird::input::RawAdapter;
use songbird::SerenityInit;

use shared::config::Config;
use shared::helper::*;
use shared::log::*;
use shared::mpd::Song;
use shared::mpd_client::MpdClient;
use shared::romanize::Romanizer;

const COMMAND_PREFIX: &str = "!r";

pub async fn launch(config: Config) {
    let handler = Handler {
        mpd: Mutex::new(MpdClient::connect(config.mpd_address).unwrap()),
        romanizer: Romanizer::new().unwrap(),
    };
    let mut client = ClientBuilder::new(
        &config.discord_token,
        GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT,
    )
    .event_handler(handler)
    .register_songbird()
    .await
    .unwrap();

    if let Err(err) = client.start().await {
        error!("[discord] client error: {:?}", err);
    }
}

struct Handler {
    mpd: Mutex<MpdClient>,
    romanizer: Romanizer,
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.content.starts_with(COMMAND_PREFIX) {
            let arguments: Vec<&str> = msg.content.split(' ').collect();
            if arguments.first() != Some(&COMMAND_PREFIX) {
                return;
            }

            let mut mpd = self.mpd.lock().await;
            // TODO: Probably migrate to serenity proc-macro based commands...
            let response = 'response: {
                match arguments.get(1) {
                    // TODO: help
                    Some(&"pause") => {
                        mpd.toggle_pause().unwrap();
                        None
                    }
                    Some(&"next") => {
                        mpd.next().unwrap();
                        None
                    }
                    Some(&"prev") => {
                        mpd.prev().unwrap();
                        None
                    }
                    Some(&"play") => {
                        match arguments.get(2).and_then(|queue_id| queue_id.parse().ok()) {
                            Some(queue_id) => match mpd.switch(queue_id) {
                                Ok(_) => Some(format!(
                                    "Playing \"{}\" now.",
                                    mpd.current_song()
                                        .unwrap()
                                        .unwrap()
                                        .title
                                        .unwrap_or_else(|| "???".to_string())
                                )),
                                Err(e) => {
                                    error!("{:?}", e);
                                    Some(format!("Failed to play {}!", queue_id))
                                }
                            },
                            None => Some("Missing or failed to parse song id".to_string()),
                        }
                    }
                    Some(&"vol") => {
                        // TODO: rework using a proxy struct wrapping the PCM stream so we can adjust the volume
                        Some("TODO".to_string())
                    }
                    Some(&"info") => Some(match mpd.current_song().unwrap() {
                        Some(song) => format_mpd_songinfo(&song, &self.romanizer),
                        None => "Currently no song is playing!".to_string(),
                    }),
                    Some(&"quit") => match msg.guild(&ctx.cache).map(|g| g.id) {
                        None => Some("Groups and DMs not supported".to_string()),
                        Some(guild_id) => {
                            let manager = songbird::get(&ctx).await.unwrap().clone();
                            match manager.get(guild_id) {
                                Some(_) => {
                                    manager.remove(guild_id).await.unwrap();
                                    None
                                }
                                None => Some("Currently not in any channel!".to_string()),
                            }
                        }
                    },
                    None | Some(&"join") => {
                        let manager = songbird::get(&ctx).await.unwrap().clone();
                        let (guild_id, channel_id) = match msg.guild(&ctx.cache) {
                            None => {
                                break 'response Some("Groups and DMs not supported".to_string())
                            }
                            Some(guild) => {
                                let channel_id = guild
                                    .voice_states
                                    .get(&msg.author.id)
                                    .and_then(|voice_state| voice_state.channel_id);
                                let guild_id = guild.id;
                                match channel_id {
                                    None => {
                                        break 'response Some("Not in a voice channel".to_string())
                                    }
                                    Some(channel_id) => (guild_id, channel_id),
                                }
                            }
                        };

                        match manager.join(guild_id, channel_id).await {
                            Err(err) => Some(format!("Error joining the channel: {}", err)),
                            Ok(call) => {
                                let fifo = File::open("/tmp/mpd_bot.fifo").unwrap();
                                // Discord uses 48 kHz sample rate so we avoid resampling here
                                let raw_stream = RawAdapter::new(fifo, 48000, 2);
                                call.lock().await.play_only_input(raw_stream.into());
                                None
                            }
                        }
                    }
                    _ => Some("Unrecognized command... TODO: reference".to_string()),
                }
            };
            if let Some(response) = response {
                if let Err(err) = msg.channel_id.say(&ctx.http, response).await {
                    error!("[discord] error sending message: {:?}", err);
                }
            }
        }
    }

    async fn ready(&self, _: Context, ready: Ready) {
        info!("[discord] {} is connected!", ready.user.name);
    }
}

fn format_mpd_songinfo(song: &Song, romanizer: &Romanizer) -> String {
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
        .artist
        .as_ref()
        .map(|a| add_romanization(a.clone(), romanizer));
    let album = song.tags.iter().find_map(|(tag, value)| {
        if tag == "Album" {
            Some(add_romanization(value.clone(), romanizer))
        } else {
            None
        }
    });

    info += &format!("Title:    {}\n", title,);
    if let Some(artist) = artist {
        info += &format!("Artist:   {}\n", artist)
    }
    if let Some(album) = album {
        info += &format!("Album:    {}\n", album)
    }
    if let Some(duration) = song.duration {
        info += &format!("Duration: {}\n", format_duration(&duration));
    }

    info += "```";
    info
}
