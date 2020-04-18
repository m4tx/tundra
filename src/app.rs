use std::collections::HashSet;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use log::info;
use notify_rust::Notification;
use tokio::time;

use crate::anime_relations::AnimeRelations;
use crate::clients::mal_client::MalClient;
use crate::clients::AnimeDbClient;
use crate::config::Config;
use crate::player_controller::{Player, PlayerController};
use crate::title_recognizer::{Title, TitleRecognizer};

// Check player status every 20 seconds
static REFRESH_INTERVAL: u64 = 20000;

pub struct TundraApp {
    config: Arc<RwLock<Config>>,
    player_controller: PlayerController,
    title_recognizer: TitleRecognizer,
    mal_client: MalClient,
    scrobbled_titles: HashSet<Title>,
}

impl TundraApp {
    pub fn init() -> Result<Self, Box<dyn std::error::Error>> {
        let config = Arc::new(RwLock::new(Config::load()));
        let anime_relations = Arc::new(AnimeRelations::new());
        let player_controller = PlayerController::new()?;
        let title_recognizer = TitleRecognizer::new();
        let mal_client = MalClient::new(config.clone(), anime_relations.clone())?;
        let scrobbled_titles = Default::default();

        Ok(Self {
            config,
            player_controller,
            title_recognizer,
            mal_client,
            scrobbled_titles,
        })
    }

    pub async fn authenticate_mal(
        &mut self,
        username: &str,
        password: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.mal_client.authenticate(username, password).await?;
        Ok(())
    }

    pub fn check_mal_authenticated(&self) {
        if !self.config.read().unwrap().is_mal_authenticated() {
            panic!(
                "You are not authenticated to MyAnimeList. \
            Please execute `tundra authenticate <username> <password>` first."
            );
        }
    }

    pub async fn run_daemon(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut interval = time::interval(Duration::from_millis(REFRESH_INTERVAL));

        loop {
            interval.tick().await;
            self.try_scrobble().await?;
        }
    }

    pub async fn try_scrobble(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Checking active players");

        let players = self.player_controller.get_players()?;
        let mut titles = Vec::new();

        for player in players {
            if let Some(title) = Self::check_player(&mut self.title_recognizer, &player)? {
                info!(
                    "Found an active player: {}, playing {} episode {}",
                    player.player_name()?,
                    title.title,
                    title.episode_number
                );

                if self.scrobbled_titles.contains(&title) {
                    info!("Already scrobbled, skipping...");
                } else {
                    titles.push(title);
                }
            }
        }

        for title in titles {
            self.scrobble_title(&title).await?;
        }

        Ok(())
    }

    fn check_player(
        title_recognizer: &mut TitleRecognizer,
        player: &Player,
    ) -> Result<Option<Title>, Box<dyn std::error::Error>> {
        let filename = player.filename_played();
        if filename.is_ok() && player.position()? > 0.5 && player.is_currently_playing()? {
            Ok(title_recognizer.recognize(&filename.unwrap()))
        } else {
            Ok(None)
        }
    }

    async fn scrobble_title(&mut self, title: &Title) -> Result<(), Box<dyn std::error::Error>> {
        info!(
            "Scrobbling {} episode {}",
            title.title, title.episode_number
        );

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
