use std::sync::Arc;

use serde::Deserialize;

use async_trait::async_trait;

use crate::anime_relations::{AnimeDbs, AnimeRelations};
use crate::clients::AnimeDbClient;
use crate::title_recognizer::Title;

static MAL_URL: &str = "https://api.myanimelist.net/v2";
static CLIENT_ID_HEADER: &str = "X-MAL-Client-ID";
static CLIENT_ID: &str = "6114d00ca681b7701d1e15fe11a4987e";
static USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

pub struct MalClient {
    client: reqwest::Client,
    access_token: String,
    refresh_token: String,
    anime_relations: Arc<AnimeRelations>,
}

#[derive(Debug, Deserialize)]
struct PasswordGrantResponse {
    access_token: String,
    expires_in: i64,
    refresh_token: String,
    token_type: String,
}

#[derive(Debug, Deserialize)]
struct SearchResponse {
    data: Vec<SearchResponseObject>,
}

#[derive(Debug, Deserialize)]
struct SearchResponseObject {
    node: AnimeObject,
}

#[derive(Debug, Deserialize)]
struct AnimeObject {
    id: i64,
    title: String,
    average_episode_duration: i64,
    num_episodes: i32,
    my_list_status: Option<MyListStatus>,
}

#[derive(Debug, Deserialize)]
struct MyListStatus {
    status: String,
    num_episodes_watched: i32,
}

impl MalClient {
    pub fn new(anime_relations: Arc<AnimeRelations>) -> Result<Self, Box<dyn std::error::Error>> {
        use reqwest::header;
        let mut headers = header::HeaderMap::new();
        headers.insert(
            CLIENT_ID_HEADER,
            header::HeaderValue::from_static(CLIENT_ID),
        );

        let client: reqwest::Client = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .default_headers(headers)
            .build()?;

        Ok(Self {
            client,
            access_token: "".to_owned(),
            refresh_token: "".to_owned(),
            anime_relations,
        })
    }

    pub async fn authenticate(
        &mut self,
        username: &str,
        password: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let params = [
            ("client_id", CLIENT_ID),
            ("grant_type", "password"),
            ("username", username),
            ("password", password),
        ];

        let req = self
            .client
            .post(&format!("{}/auth/token", MAL_URL))
            .form(&params);
        let response: PasswordGrantResponse =
            req.send().await?.json::<PasswordGrantResponse>().await?;

        self.access_token = response.access_token;
        self.refresh_token = response.refresh_token;

        Ok(())
    }

    async fn search(&self, query: &str) -> Result<SearchResponse, Box<dyn std::error::Error>> {
        let params = [
            ("q", query),
            (
                "fields",
                "title,alternative_titles,average_episode_duration,num_episodes,my_list_status",
            ),
        ];

        let req = self
            .client
            .get(&format!("{}/anime", MAL_URL))
            .header("Authorization", format!("Bearer {}", self.access_token))
            .query(&params);

        Ok(req.send().await?.json::<SearchResponse>().await?)
    }

    async fn get_by_id(&self, id: i64) -> Result<AnimeObject, Box<dyn std::error::Error>> {
        let params = [(
            "fields",
            "title,alternative_titles,average_episode_duration,num_episodes,my_list_status",
        )];

        let req = self
            .client
            .get(&format!("{}/anime/{}", MAL_URL, id))
            .header("Authorization", format!("Bearer {}", self.access_token))
            .query(&params);

        Ok(req.send().await?.json::<AnimeObject>().await?)
    }

    async fn set_status(
        &self,
        id: i64,
        status: &str,
        num_episodes_watched: i32,
    ) -> Result<MyListStatus, Box<dyn std::error::Error>> {
        let params = [
            ("status", status),
            ("num_watched_episodes", &num_episodes_watched.to_string()),
        ];

        let req = self
            .client
            .patch(&format!("{}/anime/{}/my_list_status", MAL_URL, id))
            .header("Authorization", format!("Bearer {}", self.access_token))
            .form(&params);

        Ok(req.send().await?.json::<MyListStatus>().await?)
    }

    async fn set_episode_number(
        &self,
        anime_object: &AnimeObject,
        num_episodes_watched: i32,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        if anime_object.my_list_status.is_none() {
            return Ok(false);
        }

        let new_status = if num_episodes_watched == anime_object.num_episodes {
            "completed"
        } else {
            "watching"
        };
        self.set_status(anime_object.id, new_status, num_episodes_watched)
            .await?;
        Ok(true)
    }
}

#[async_trait]
impl AnimeDbClient for MalClient {
    async fn set_title_watched(
        &self,
        title: &Title,
    ) -> Result<Option<Title>, Box<dyn std::error::Error>> {
        let results = self.search(&title.title).await?;
        if !results.data.is_empty() {
            let anime_object = &results.data[0].node;
            let relation_rule = self
                .anime_relations
                .get_rule(&AnimeDbs::Mal, anime_object.id);

            if let Some(rule) = relation_rule {
                let (new_id, new_ep) = rule.convert_episode_number(
                    &AnimeDbs::Mal,
                    anime_object.id,
                    title.episode_number,
                );
                let anime_object = self.get_by_id(new_id).await?;
                self.set_episode_number(&anime_object, new_ep).await?;

                let new_title = Title::new(anime_object.title, new_ep);
                Ok(Some(new_title))
            } else {
                self.set_episode_number(anime_object, title.episode_number)
                    .await?;

                let new_title = Title::new(anime_object.title.clone(), title.episode_number);
                Ok(Some(new_title))
            }
        } else {
            Ok(None)
        }
    }
}
