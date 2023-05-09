use std::fmt::{Display, Formatter};

use async_trait::async_trait;

use crate::title_recognizer::Title;

pub mod mal_client;

#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct AnimeId(pub String);

impl Display for AnimeId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct WebsiteUrl(pub String);

impl Display for WebsiteUrl {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct PictureUrl(pub String);

impl Display for PictureUrl {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct AnimeInfo {
    pub id: AnimeId,
    pub picture: PictureUrl,
    pub website_url: WebsiteUrl,
    pub title: String,
    pub episode_watched: i32,
    pub total_episodes: i32,
}

#[async_trait]
pub trait AnimeDbClient {
    async fn get_anime_info(&mut self, title: &Title) -> anyhow::Result<Option<AnimeInfo>>;

    async fn set_title_watched(&mut self, anime_info: &AnimeInfo) -> anyhow::Result<bool>;
}
