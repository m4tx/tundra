use std::fs;

use directories::ProjectDirs;

use crate::app::{Config, MALConfig, TundraApp};
use clap::{App, Arg, SubCommand};

mod anime_relations;
mod app;
mod clients;
mod player_controller;
mod title_recognizer;

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
        let mut app = TundraApp::init()?;
        app.authenticate_mal().await?;
        app.run_daemon().await?;
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
