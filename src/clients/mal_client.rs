use crate::clients::AnimeDbClient;
use crate::title_recognizer::Title;
use async_trait::async_trait;
use serde::Deserialize;

static MAL_URL: &str = "https://api.myanimelist.net";
static CLIENT_ID_HEADER: &str = "X-MAL-Client-ID";
static CLIENT_ID: &str = "6114d00ca681b7701d1e15fe11a4987e";
static USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"),);

pub struct MalClient {
    client: reqwest::Client,
    access_token: String,
    refresh_token: String,
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
    pub async fn new(username: &str, password: &str) -> Result<Self, Box<dyn std::error::Error>> {
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

        let mut client = Self {
            client,
            access_token: "".to_owned(),
            refresh_token: "".to_owned(),
        };
        client.authenticate(username, password).await?;

        Ok(client)
    }

    async fn authenticate(
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
            .post(&format!("{}/v2/auth/token", MAL_URL))
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
            .get(&format!("{}/v2/anime", MAL_URL))
            .header("Authorization", format!("Bearer {}", self.access_token))
            .query(&params);

        Ok(req.send().await?.json::<SearchResponse>().await?)
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
            .patch(&format!("{}/v2/anime/{}/my_list_status", MAL_URL, id))
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
    async fn set_title_watched(&self, title: &Title) -> Result<bool, Box<dyn std::error::Error>> {
        let results = self.search(&title.title).await?;
        if !results.data.is_empty() {
            let anime_object = &results.data[0].node;
            self.set_episode_number(anime_object, title.episode_number)
                .await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}
