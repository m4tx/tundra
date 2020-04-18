use std::fs;
use std::sync::Arc;

use directories::ProjectDirs;
use notify_rust::Notification;
use serde::{Deserialize, Serialize};

use crate::anime_relations::AnimeRelations;
use crate::clients::mal_client::MalClient;
use crate::clients::AnimeDbClient;
use crate::player_controller::PlayerController;
use crate::title_recognizer::TitleRecognizer;
use clap::{App, Arg, SubCommand};

mod anime_relations;
mod clients;
mod player_controller;
mod title_recognizer;

#[derive(Deserialize, Serialize)]
struct Config {
    mal: MALConfig,
}

#[derive(Deserialize, Serialize)]
struct MALConfig {
    username: String,
    password: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about("MyAnimeList scrobbler")
        .subcommand(
            SubCommand::with_name("authenticate")
                .about("sign in to MyAnimeList")
                .version(env!("CARGO_PKG_VERSION"))
                .author(env!("CARGO_PKG_AUTHORS"))
                .arg(
                    Arg::with_name("username")
                        .required(true)
                        .help("MyAnimeList username"),
                )
                .arg(
                    Arg::with_name("password")
                        .required(true)
                        .help("MyAnimeList password"),
                ),
        )
        .get_matches();

    if let Some(matches) = matches.subcommand_matches("authenticate") {
        let username = matches.value_of("username").unwrap();
        let password = matches.value_of("password").unwrap();

        save_config(username, password)?;
    } else {
        scrobble().await?;
    }

    Ok(())
}

fn save_config(username: &str, password: &str) -> Result<(), Box<dyn std::error::Error>> {
    let project_dirs =
        ProjectDirs::from("com", "m4tx", "tundra").ok_or("config directory not found")?;
    fs::create_dir_all(project_dirs.config_dir())?;
    let config_file = project_dirs.config_dir().join("config.toml");

    let config = Config {
        mal: MALConfig {
            username: username.to_owned(),
            password: password.to_owned(),
        },
    };
    let toml = toml::to_string(&config).unwrap();

    fs::write(config_file, toml)?;

    Ok(())
}

async fn scrobble() -> Result<(), Box<dyn std::error::Error>> {
    let anime_relations = Arc::new(AnimeRelations::new());

    let project_dirs =
        ProjectDirs::from("com", "m4tx", "tundra").ok_or("config directory not found")?;
    let config_file = project_dirs.config_dir().join("config.toml");
    let config_file_str = fs::read_to_string(config_file)?;
    let config: Config = toml::from_str(&config_file_str)?;

    let client =
        MalClient::new(&config.mal.username, &config.mal.password, anime_relations).await?;

    let controller = PlayerController::new()?;
    let mut title_recognizer = TitleRecognizer::new();
    let players = controller.get_players()?;

    for player in players {
        println!(
            "Player \"{}\", currently playing? {}",
            player.player_name()?,
            player.is_currently_playing()?
        );

        let filename = player.filename_played();
        if filename.is_ok() {
            let title = title_recognizer.recognize(&filename.unwrap());
            match title {
                None => {}
                Some(t) => {
                    println!(
                        "Currently playing {}, episode {}, at {}",
                        t.title,
                        t.episode_number,
                        player.position()?
                    );
                    if player.position()? > 0.5 && player.is_currently_playing()? {
                        let new_title = client.set_title_watched(&t).await?;

                        if let Some(title) = new_title {
                            Notification::new()
                                .summary("Tundra")
                                .body(&format!(
                                    "Scrobbled anime: {}, episode {}",
                                    title.title, title.episode_number
                                ))
                                .icon("dialog-information-symbolic")
                                .timeout(6000)
                                .show();
                        }
                    } else {
                        println!("Not scrobbling");
                    }
                }
            }
        }
    }

    Ok(())
}
