extern crate byteorder;
extern crate discord;
extern crate dotenv;
extern crate mpd;

mod mpd_client;

use mpd_client::MpdClient;

use std::net::ToSocketAddrs;
use std::fs::File;
use std::env;
use std::time::Duration;

use discord::{Connection, Discord, State};
use discord::model::{ChannelId, Event};
use discord::voice::create_pcm_source;
use dotenv::dotenv;
use mpd::Song;

const COMMAND: &str = "!r";

fn main() {
    dotenv().ok();
    let token = &env::var("DISCORD_TOKEN")
        .expect("DISCORD_TOKEN not set! Did you forget to create a .env file?");

    let discord = Discord::from_bot_token(token).expect("login failed");

    let (mut connection, ready) = discord.connect().expect("connect failed");
    println!(
        "[Ready] \"{}\" is serving {} servers",
        ready.user.username,
        ready.servers.len()
    );
    let mut state = State::new(ready);
    connection.sync_calls(&state.all_private_channels());

    let mut mpd = MpdClient::connect("127.0.0.1:6600").unwrap();

    loop {
        let event = match connection.recv_event() {
            Ok(event) => event,
            Err(err) => {
                println!("[WARN] Received error: {:?}", err);
                match err {
                    discord::Error::WebSocket(..) => {
                        // Handle the websocket connection being dropped
                        let (new_connection, ready) = discord.connect().expect("connect failed");
                        connection = new_connection;
                        state = State::new(ready);
                        println!("[INFO] Reconnected successfully.");
                    }
                    discord::Error::Closed(..) => {
                        println!("[WARN] Connection closed!");
                    }
                    _ => {}
                }
                continue;
            }
        };
        state.update(&event);
        handle_event(event, &state, &mut connection, &discord, &mut mpd);
    }
}
fn handle_event<A: ToSocketAddrs>(
    event: Event,
    state: &State,
    connection: &mut Connection,
    discord: &Discord,
    mpd: &mut MpdClient<A>,
) -> Option<discord::Error> {
    let send_message = |channel: ChannelId, message: &str| {
        return discord.send_message(channel, message, "", false).err();
    };

    match event {
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
                    voice_channel.map(|(sid, _)| connection.drop_voice(sid));
                }
                "info" => {
                    if let Some(song) = mpd.currentsong().unwrap() {
                        send_message(message.channel_id, &format_mpd_singinfo(&song));
                    } else {
                        send_message(message.channel_id, "No song currently playing!");
                    }
                }
                _ => {
                    let output = if let Some((server_id, channel_id)) = voice_channel {
                        let voice = connection.voice(server_id);
                        voice.set_deaf(true);
                        voice.connect(channel_id);
                        voice.play(create_pcm_source(
                            true,
                            File::open("/tmp/mpd_bot.fifo").unwrap(),
                        ));
                        String::new()
                    } else {
                        "You must be in a voice channel to invate Rusty Webradio".to_string()
                    };
                    if !output.is_empty() {
                        send_message(message.channel_id, &output);
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
                            if srv.voice_states
                                .iter()
                                .filter(|vs| vs.channel_id == Some(cur_channel))
                                .count() <= 1
                            {
                                connection.voice(Some(server_id)).disconnect();
                            }
                        }
                    }
                    None => if let Some(call) = state.calls().get(&cur_channel) {
                        if call.voice_states.len() <= 1 {
                            connection.voice(server_id).disconnect();
                        }
                    },
                }
            }
        }
        _ => {} // discard other events
    }
    None
}

fn format_mpd_singinfo(song: &Song) -> String {
    let mut info = "```\n".to_string();
    info += &format!(
        "Title: {}\n",
        if let Some(ref title) = song.title {
            title
        } else {
            &song.file
        }
    );
    if let Some(album) = song.tags.get("Album") {
        info += &format!("Album: {}\n", album)
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

fn format_duration(duration: &Duration) -> String {
    let hours = duration.as_secs() / (60 * 60);
    let minutes = (duration.as_secs() - hours * 60 * 60) / 60;
    let seconds = duration.as_secs() - hours * 60 * 60 - minutes * 60;
    if hours > 0 {
        format!("{}:{:02}:{:02}", hours, minutes, seconds)
    } else {
        format!("{}:{:02}", minutes, seconds)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;
    #[test]
    fn format_duration() {
        use super::format_duration;
        assert_eq!("0:00", format_duration(&Duration::from_secs(0)));
        assert_eq!("0:10", format_duration(&Duration::from_secs(10)));
        assert_eq!("0:59", format_duration(&Duration::from_secs(59)));
        assert_eq!("1:00", format_duration(&Duration::from_secs(60)));
        assert_eq!("1:10", format_duration(&Duration::from_secs(70)));
        assert_eq!("10:42", format_duration(&Duration::from_secs(10 * 60 + 42)));
        assert_eq!("59:59", format_duration(&Duration::from_secs(59 * 60 + 59)));
        assert_eq!("1:00:00", format_duration(&Duration::from_secs(3600)));
        assert_eq!(
            "42:42:42",
            format_duration(&Duration::from_secs(42 * 3600 + 42 * 60 + 42))
        );
    }
}
