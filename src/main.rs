use clap::{App, Arg, SubCommand};

use crate::app::TundraApp;
use crate::logging::init_logging;

mod anime_relations;
mod app;
mod clients;
mod config;
mod logging;
mod player_controller;
mod title_recognizer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_logging().expect("Could not initialize logging");

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

    let mut app = TundraApp::init()?;

    if let Some(matches) = matches.subcommand_matches("authenticate") {
        let username = matches.value_of("username").unwrap();
        let password = matches.value_of("password").unwrap();

        app.authenticate_mal(username, password).await?;
    } else {
        app.check_mal_authenticated();
        app.run_daemon().await?;
    }

    Ok(())
}
