use async_trait::async_trait;

use crate::title_recognizer::Title;

pub mod mal_client;

#[derive(Clone, Eq, Hash, PartialEq)]
pub struct AnimeInfo {
    pub id: String,
    pub picture: String,
    pub title: String,
    pub episode_watched: i32,
    pub total_episodes: i32,
}

#[async_trait]
pub trait AnimeDbClient {
    async fn get_anime_info(
        &mut self,
        title: &Title,
    ) -> Result<Option<AnimeInfo>, Box<dyn std::error::Error>>;

    async fn set_title_watched(
        &mut self,
        anime_info: &AnimeInfo,
    ) -> Result<bool, Box<dyn std::error::Error>>;
}
