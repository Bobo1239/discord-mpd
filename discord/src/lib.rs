use std::fs::File;
use std::io::BufReader;
use std::sync::Arc;

use serenity::client::bridge::voice::ClientVoiceManager;
use serenity::model::prelude::*;
use serenity::prelude::*;
use serenity::voice;

use shared::config::Config;
use shared::helper::*;
use shared::log::*;
use shared::mpd::Song;
use shared::mpd_client::MpdClient;
use shared::romanize::Romanizer;

const COMMAND_PREFIX: &str = "!r";

pub fn launch(config: &Config) {
    let handler = Handler {
        mpd: Mutex::new(MpdClient::connect(config.mpd_address).unwrap()),
        romanizer: Romanizer::new().unwrap(),
    };
    let mut client = Client::new(&config.discord_token, handler).unwrap();
    {
        let mut data = client.data.write();
        data.insert::<VoiceManager>(Arc::clone(&client.voice_manager));
    }
    if let Err(err) = client.start() {
        error!("[discord] client error: {:?}", err);
    }
}

struct VoiceManager;

impl TypeMapKey for VoiceManager {
    type Value = Arc<Mutex<ClientVoiceManager>>;
}

struct Handler {
    mpd: Mutex<MpdClient>,
    romanizer: Romanizer,
}

impl EventHandler for Handler {
    fn message(&self, ctx: Context, msg: Message) {
        if msg.content.starts_with(COMMAND_PREFIX) {
            let arguments: Vec<&str> = msg.content.split(' ').collect();
            if arguments.get(0) != Some(&COMMAND_PREFIX) {
                return;
            }

            let mut mpd = self.mpd.lock();
            let response = match arguments.get(1) {
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
                                    .unwrap_or("???".to_string())
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
                Some(&"quit") => {
                    let result = msg
                        .guild(&ctx.cache)
                        .ok_or_else(|| "Groups and DMs not supported".to_string())
                        .and_then(|guild| {
                            let guild = guild.read();
                            let manager_lock =
                                ctx.data.read().get::<VoiceManager>().cloned().unwrap();
                            let mut manager = manager_lock.lock();

                            match manager.get(guild.id) {
                                Some(_) => {
                                    manager.remove(guild.id);
                                    Ok(())
                                }
                                None => Err("Currently not in any channel!".to_string()),
                            }
                        });
                    if let Err(msg) = result {
                        Some(msg)
                    } else {
                        None
                    }
                }
                None | Some(&"join") => {
                    let result = msg
                        .guild(&ctx.cache)
                        .ok_or_else(|| "Groups and DMs not supported".to_string())
                        .and_then(|guild| {
                            let guild = guild.read();
                            guild
                                .voice_states
                                .get(&msg.author.id)
                                .and_then(|voice_state| voice_state.channel_id)
                                .ok_or_else(|| "Not in a voice channel".to_string())
                                .map(|channel_id| (guild.id, channel_id))
                        })
                        .and_then(|(guild_id, channel_id)| {
                            let manager_lock =
                                ctx.data.read().get::<VoiceManager>().cloned().unwrap();
                            let mut manager = manager_lock.lock();
                            // TODO: Make this configurable and also note that the sample
                            //       rate should be 48kHz
                            let fifo = File::open("/tmp/mpd_bot.fifo").unwrap();
                            let reader = BufReader::new(fifo);

                            if manager.join(guild_id, channel_id).is_none() {
                                Err("Error joining the channel".to_string())
                            } else {
                                manager
                                    .get_mut(guild_id)
                                    .unwrap()
                                    .play(voice::pcm(true, reader));
                                Ok(())
                            }
                        });
                    if let Err(msg) = result {
                        Some(msg)
                    } else {
                        None
                    }
                }
                _ => Some("Unrecognized command... TODO: reference".to_string()),
            };
            if let Some(response) = response {
                if let Err(err) = msg.channel_id.say(&ctx.http, response) {
                    error!("[discord] error sending message: {:?}", err);
                }
            }
        }
    }

    fn ready(&self, _: Context, ready: Ready) {
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
