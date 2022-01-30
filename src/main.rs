use std::env;

use clap::{App, Arg};
use gettextrs::TextDomain;
use log::info;

use crate::app::TundraApp;
use crate::constants::{APP_AUTHORS, APP_NAME, APP_VERSION};
use crate::logging::init_logging;

mod anime_relations;
mod app;
mod clients;
mod config;
mod constants;
mod gtk_gui;
mod logging;
mod player_controller;
mod title_recognizer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_logging().expect("Could not initialize logging");
    init_i18n()?;

    let matches = App::new(APP_NAME)
        .version(APP_VERSION)
        .author(APP_AUTHORS)
        .about("MyAnimeList scrobbler")
        .subcommand(
            App::new("authenticate")
                .about("sign in to MyAnimeList")
                .version(APP_VERSION)
                .author(APP_AUTHORS)
                .arg(
                    Arg::new("username")
                        .required(true)
                        .help("MyAnimeList username"),
                )
                .arg(
                    Arg::new("password")
                        .required(true)
                        .help("MyAnimeList password"),
                ),
        )
        .subcommand(
            App::new("daemon")
                .about("start Tundra daemon")
                .version(APP_VERSION)
                .author(APP_AUTHORS),
        )
        .get_matches();

    let mut app = TundraApp::init()?;

    if let Some(matches) = matches.subcommand_matches("authenticate") {
        let username = matches.value_of("username").unwrap();
        let password = matches.value_of("password").unwrap();

        app.authenticate_mal(username, password).await?;
    } else if matches.subcommand_matches("daemon").is_some() {
        app.check_mal_authenticated();
        app.run_daemon().await?;
    } else {
        gtk_gui::GtkApp::start(app);
    }

    Ok(())
}

fn init_i18n() -> Result<(), Box<dyn std::error::Error>> {
    let text_domain = TextDomain::new("tundra");
    let text_domain = if cfg!(debug_assertions) {
        let exe_path = env::current_exe()?;
        let path = exe_path.parent().unwrap().parent().unwrap();
        println!("{}", path.display());
        text_domain.push(path)
    } else {
        text_domain
    };
    text_domain.init().unwrap_or_else(|e| {
        info!("Did not initialize i18n: {}", e);
        None
    });

    Ok(())
}
