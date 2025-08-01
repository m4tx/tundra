use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

use anyhow::Context;
use log::{error, info};
use notify_rust::Notification;
use tokio::time;

use crate::anime_relations::AnimeRelations;
use crate::clients::mal_client::{MalAuthenticator, MalClient, MalClientResult};
use crate::clients::{AnimeDbClient, AnimeInfo};
use crate::config::Config;
use crate::constants::REFRESH_INTERVAL;
use crate::player_controller::{Player, PlayerController};
use crate::title_recognizer::{Title, TitleRecognizer};

#[derive(Clone)]
pub struct PlayedTitle {
    pub anime_info: AnimeInfo,
    pub player_name: String,
    pub scrobbled: bool,
    pub should_scrobble: bool,
}

pub struct TundraApp {
    config: Arc<RwLock<Config>>,
    player_controller: PlayerController,
    title_recognizer: TitleRecognizer,
    mal_client: MalClient,
    scrobbled_titles: HashSet<AnimeInfo>,
    anime_info_cache: HashMap<Title, Option<AnimeInfo>>,
}

impl TundraApp {
    pub fn init() -> anyhow::Result<Self> {
        let config = Arc::new(RwLock::new(Config::load()));
        let anime_relations = Arc::new(AnimeRelations::new());
        let player_controller = PlayerController::new()?;
        let title_recognizer = TitleRecognizer::new();
        let mal_client = MalClient::new(config.clone(), anime_relations)?;
        let scrobbled_titles = Default::default();

        Ok(Self {
            config,
            player_controller,
            title_recognizer,
            mal_client,
            scrobbled_titles,
            anime_info_cache: HashMap::new(),
        })
    }

    pub async fn authenticate_mal_cli(&mut self) -> anyhow::Result<()> {
        let authenticator = self.start_mal_authentication().await?;
        info!(
            "Visit this website to authenticate: {}",
            authenticator.get_auth_url()
        );
        authenticator.wait_for_auth().await?;
        info!("Successfully authenticated!");

        Ok(())
    }

    pub async fn start_mal_authentication(&mut self) -> MalClientResult<MalAuthenticator> {
        self.scrobbled_titles.clear();
        self.mal_client.start_authentication().await
    }

    pub fn is_mal_authenticated(&self) -> bool {
        self.config.read().unwrap().is_mal_authenticated()
    }

    pub fn check_mal_authenticated(&self) {
        assert!(
            self.is_mal_authenticated(),
            "You are not authenticated to MyAnimeList. \
            Please execute `tundra authenticate <username> <password>` first."
        );
    }

    pub async fn run_daemon(&mut self) {
        let mut interval = time::interval(REFRESH_INTERVAL);

        loop {
            interval.tick().await;
            let scrobble_result = self.try_scrobble().await;
            if let Err(err) = scrobble_result {
                error!("{err:?}");
            }
        }
    }

    async fn get_scrobblable_title(&mut self) -> anyhow::Result<Option<(Title, String, bool)>> {
        info!("Checking active players");

        let players = self.player_controller.get_players()?;
        for player in players {
            if let Some(title) = Self::check_player(&mut self.title_recognizer, &player)? {
                let player_name = player.player_name()?;
                let should_scrobble = player.position()? > 0.5;
                info!(
                    "Found an active player: {}, playing {} season {} episode {}",
                    player_name, title.title, title.season_number, title.episode_number
                );
                return Ok(Some((title, player_name, should_scrobble)));
            }
        }

        Ok(None)
    }

    fn check_player(
        title_recognizer: &mut TitleRecognizer,
        player: &Player,
    ) -> anyhow::Result<Option<Title>> {
        if player.is_currently_playing()? {
            let title = player.title_played().ok();
            let filename = player.filename_played().ok();
            let title = title_recognizer.recognize(title.as_deref(), filename.as_deref());

            Ok(title)
        } else {
            Ok(None)
        }
    }

    pub async fn get_played_title(&mut self) -> anyhow::Result<Option<PlayedTitle>> {
        let result = self.get_scrobblable_title().await?;

        if let Some((title, player_name, should_scrobble)) = result {
            let anime_info = self.anime_info_for_title(title).await?;

            if let Some(anime_info) = anime_info {
                let scrobbled = self.scrobbled_titles.contains(&anime_info);
                Ok(Some(PlayedTitle {
                    anime_info,
                    player_name,
                    scrobbled,
                    should_scrobble,
                }))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    async fn anime_info_for_title(&mut self, title: Title) -> anyhow::Result<Option<AnimeInfo>> {
        if self.anime_info_cache.contains_key(&title) {
            return Ok(self.anime_info_cache[&title].clone());
        }

        let anime_info = self.mal_client.get_anime_info(&title).await?;
        self.anime_info_cache.insert(title, anime_info.clone());

        Ok(anime_info)
    }

    pub async fn try_scrobble(&mut self) -> anyhow::Result<()> {
        let title = self.get_played_title().await?;

        if let Some(title) = title {
            if title.should_scrobble {
                if title.scrobbled {
                    info!("Already scrobbled, skipping...");
                } else {
                    self.scrobble_title(&title.anime_info).await?;
                }
            }
        }

        Ok(())
    }

    async fn scrobble_title(&mut self, anime_info: &AnimeInfo) -> anyhow::Result<()> {
        info!(
            "Scrobbling {} episode {} / {}",
            anime_info.title, anime_info.episode_watched, anime_info.total_episodes
        );

        let scrobbled = self.mal_client.set_title_watched(anime_info).await?;
        self.scrobbled_titles.insert(anime_info.clone());

        if scrobbled {
            Notification::new()
                .summary("Tundra")
                .body(&format!(
                    "Scrobbled anime: {}, episode {} / {}",
                    anime_info.title, anime_info.episode_watched, anime_info.total_episodes
                ))
                .icon("dialog-information-symbolic")
                .timeout(6000)
                .show()
                .with_context(
                    || "Failed to send the notification. Is the notification daemon running?",
                )?;
        }

        Ok(())
    }
}
