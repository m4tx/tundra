use std::env;

use clap::Command;
use gettextrs::TextDomain;
use log::info;

use crate::app::TundraApp;
use crate::constants::{APP_AUTHORS, APP_NAME, APP_VERSION, GETTEXT_PACKAGE};
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
async fn main() -> anyhow::Result<()> {
    init_logging().expect("Could not initialize logging");
    init_i18n()?;

    let matches = Command::new(APP_NAME)
        .version(APP_VERSION)
        .author(APP_AUTHORS)
        .about("MyAnimeList scrobbler")
        .subcommand(
            Command::new("authenticate")
                .about("Sign in to MyAnimeList")
                .version(APP_VERSION)
                .author(APP_AUTHORS),
        )
        .subcommand(
            Command::new("daemon")
                .about("Start Tundra daemon")
                .version(APP_VERSION)
                .author(APP_AUTHORS),
        )
        .get_matches();

    let mut app = TundraApp::init()?;

    if matches.subcommand_matches("authenticate").is_some() {
        app.authenticate_mal_cli().await?;
    } else if matches.subcommand_matches("daemon").is_some() {
        app.check_mal_authenticated();
        app.run_daemon().await;
    } else {
        gtk_gui::GtkApp::start(app);
    }

    Ok(())
}

fn init_i18n() -> anyhow::Result<()> {
    let text_domain = TextDomain::new(GETTEXT_PACKAGE);
    let text_domain = if cfg!(debug_assertions) {
        let exe_path = env::current_exe()?;
        let path = exe_path.parent().unwrap().parent().unwrap();
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
