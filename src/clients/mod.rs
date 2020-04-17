use crate::title_recognizer::Title;
use async_trait::async_trait;
pub mod mal_client;

#[async_trait]
pub trait AnimeDbClient {
    async fn set_title_watched(title: Title) -> Result<(), Box<dyn std::error::Error>>;
}
