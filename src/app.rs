use std::fs;
use std::sync::Arc;

use directories::ProjectDirs;
use notify_rust::Notification;
use serde::{Deserialize, Serialize};

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

pub struct TundraApp {
    config: Config,
    player_controller: PlayerController,
    title_recognizer: TitleRecognizer,
    mal_client: MalClient,
}

impl TundraApp {
    pub fn init() -> Result<Self, Box<dyn std::error::Error>> {
        let config = Self::load_config()?;
        let anime_relations = Arc::new(AnimeRelations::new());
        let player_controller = PlayerController::new()?;
        let title_recognizer = TitleRecognizer::new();
        let mal_client = MalClient::new(anime_relations.clone())?;

        Ok(Self {
            config,
            player_controller,
            title_recognizer,
            mal_client,
        })
    }

    fn load_config() -> Result<Config, Box<dyn std::error::Error>> {
        let project_dirs =
            ProjectDirs::from("com", "m4tx", "tundra").ok_or("config directory not found")?;
        let config_file = project_dirs.config_dir().join("config.toml");
        let config_file_str = fs::read_to_string(config_file)?;

        Ok(toml::from_str(&config_file_str)?)
    }

    pub async fn authenticate_mal(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.mal_client
            .authenticate(&self.config.mal.username, &self.config.mal.password)
            .await?;
        Ok(())
    }

    pub async fn try_scrobble(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let players = self.player_controller.get_players()?;

        for player in players {
            let filename = player.filename_played();
            if filename.is_ok() {
                let title = self.title_recognizer.recognize(&filename.unwrap());
                match title {
                    None => {}
                    Some(t) => {
                        if player.position()? > 0.5 && player.is_currently_playing()? {
                            self.scrobble_title(&t).await?;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    async fn scrobble_title(&self, title: &Title) -> Result<(), Box<dyn std::error::Error>> {
        let new_title = self.mal_client.set_title_watched(&title).await?;

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
