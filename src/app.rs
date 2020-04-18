use std::collections::HashSet;
use std::fs;
use std::sync::Arc;
use std::time::Duration;

use directories::ProjectDirs;
use notify_rust::Notification;
use serde::{Deserialize, Serialize};
use tokio::time;

use crate::anime_relations::AnimeRelations;
use crate::clients::mal_client::MalClient;
use crate::clients::AnimeDbClient;
use crate::player_controller::PlayerController;
use crate::title_recognizer::{Title, TitleRecognizer};

#[derive(Deserialize, Serialize)]
pub struct Config {
    pub mal: MALConfig,
}

#[derive(Deserialize, Serialize)]
pub struct MALConfig {
    pub username: String,
    pub password: String,
}

// Check player status every 20 seconds
static REFRESH_INTERVAL: u64 = 20000;

pub struct TundraApp {
    config: Config,
    player_controller: PlayerController,
    title_recognizer: TitleRecognizer,
    mal_client: MalClient,
    scrobbled_titles: HashSet<Title>,
}

impl TundraApp {
    pub fn init() -> Result<Self, Box<dyn std::error::Error>> {
        let config = Self::load_config()?;
        let anime_relations = Arc::new(AnimeRelations::new());
        let player_controller = PlayerController::new()?;
        let title_recognizer = TitleRecognizer::new();
        let mal_client = MalClient::new(anime_relations.clone())?;
        let scrobbled_titles = Default::default();

        Ok(Self {
            config,
            player_controller,
            title_recognizer,
            mal_client,
            scrobbled_titles,
        })
    }

    fn load_config() -> Result<Config, Box<dyn std::error::Error>> {
        let project_dirs =
            ProjectDirs::from("com", "m4tx", "tundra").ok_or("config directory not found")?;
        let config_file = project_dirs.config_dir().join("config.toml");
        let config_file_str = fs::read_to_string(config_file).expect(
            "Config file could not be read. Make sure to execute \
            `tundra authenticate <username> <password>` before using.",
        );

        Ok(toml::from_str(&config_file_str)?)
    }

    pub async fn authenticate_mal(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.mal_client
            .authenticate(&self.config.mal.username, &self.config.mal.password)
            .await?;
        Ok(())
    }

    pub async fn run_daemon(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut interval = time::interval(Duration::from_millis(REFRESH_INTERVAL));

        loop {
            interval.tick().await;
            self.try_scrobble().await?;
        }
    }

    pub async fn try_scrobble(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let players = self.player_controller.get_players()?;
        let mut titles = Vec::new();

        for player in players {
            let filename = player.filename_played();
            if filename.is_ok() {
                let title = self.title_recognizer.recognize(&filename.unwrap());
                match title {
                    None => {}
                    Some(t) => {
                        if player.position()? > 0.5 && player.is_currently_playing()? {
                            titles.push(t);
                        }
                    }
                }
            }
        }

        for title in titles {
            self.scrobble_title(&title).await?;
        }

        Ok(())
    }

    async fn scrobble_title(&mut self, title: &Title) -> Result<(), Box<dyn std::error::Error>> {
        if self.scrobbled_titles.contains(&title) {
            return Ok(());
        }

        let new_title = self.mal_client.set_title_watched(&title).await?;
        self.scrobbled_titles.insert(title.clone());

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

        Ok(())
    }
}
