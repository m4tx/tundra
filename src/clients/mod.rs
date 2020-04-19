use crate::title_recognizer::Title;
use async_trait::async_trait;
pub mod mal_client;

#[derive(Clone, Eq, Hash, PartialEq)]
pub struct AnimeInfo {
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
        title: &Title,
    ) -> Result<bool, Box<dyn std::error::Error>>;
}
