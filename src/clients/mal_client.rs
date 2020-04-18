use core::fmt;
use std::sync::Arc;

use serde::Deserialize;

use async_trait::async_trait;

use crate::anime_relations::{AnimeDbs, AnimeRelations};
use crate::clients::AnimeDbClient;
use crate::title_recognizer::Title;
use reqwest::StatusCode;

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
struct AuthenticationResponse {
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

#[derive(Debug)]
struct AuthenticationFailedError;

impl AuthenticationFailedError {
    fn new() -> Self {
        Self {}
    }
}

static AUTHENTICATION_ERROR_STRING: &str = "Could not authenticate to MyAnimeList. \
Make sure the username and password you entered is correct.";

impl fmt::Display for AuthenticationFailedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", AUTHENTICATION_ERROR_STRING)
    }
}

impl std::error::Error for AuthenticationFailedError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
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
        let params = vec![
            ("client_id", CLIENT_ID),
            ("grant_type", "password"),
            ("username", username),
            ("password", password),
        ];

        Ok(self.make_auth_request(&params).await?)
    }

    async fn refresh_auth(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let refresh_token = self.refresh_token.to_owned();
        let params = vec![
            ("client_id", CLIENT_ID),
            ("grant_type", "refresh_token"),
            ("refresh_token", &refresh_token),
        ];

        Ok(self.make_auth_request(&params).await?)
    }

    pub async fn make_auth_request(
        &mut self,
        params: &Vec<(&str, &str)>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let req = self
            .client
            .post(&format!("{}/auth/token", MAL_URL))
            .form(&params);
        let response = req.send().await?;
        if response.status().is_client_error() {
            return Err(Box::new(AuthenticationFailedError::new()));
        }

        let response_data: AuthenticationResponse = response
            .error_for_status()?
            .json::<AuthenticationResponse>()
            .await?;
        self.access_token = response_data.access_token;
        self.refresh_token = response_data.refresh_token;

        Ok(())
    }

    async fn make_request(
        &mut self,
        request: reqwest::RequestBuilder,
    ) -> Result<reqwest::Response, Box<dyn std::error::Error>> {
        let request = request.header("Authorization", format!("Bearer {}", self.access_token));
        let request_copy = request.try_clone().expect("Request could not be cloned");
        let response = request.send().await?;

        if response.status() == StatusCode::UNAUTHORIZED {
            self.refresh_auth();
            return Ok(request_copy.send().await?.error_for_status()?);
        } else {
            return Ok(response.error_for_status()?);
        }
    }

    async fn search(&mut self, query: &str) -> Result<SearchResponse, Box<dyn std::error::Error>> {
        let params = [
            ("q", query),
            (
                "fields",
                "title,alternative_titles,average_episode_duration,num_episodes,my_list_status",
            ),
        ];

        let req = self
            .make_request(
                self.client
                    .get(&format!("{}/anime", MAL_URL))
                    .query(&params),
            )
            .await?;

        Ok(req.json::<SearchResponse>().await?)
    }

    async fn get_by_id(&mut self, id: i64) -> Result<AnimeObject, Box<dyn std::error::Error>> {
        let params = [(
            "fields",
            "title,alternative_titles,average_episode_duration,num_episodes,my_list_status",
        )];

        let req = self
            .make_request(
                self.client
                    .get(&format!("{}/anime/{}", MAL_URL, id))
                    .query(&params),
            )
            .await?;

        Ok(req.json::<AnimeObject>().await?)
    }

    async fn set_status(
        &mut self,
        id: i64,
        status: &str,
        num_episodes_watched: i32,
    ) -> Result<MyListStatus, Box<dyn std::error::Error>> {
        let params = [
            ("status", status),
            ("num_watched_episodes", &num_episodes_watched.to_string()),
        ];

        let req = self
            .make_request(
                self.client
                    .patch(&format!("{}/anime/{}/my_list_status", MAL_URL, id))
                    .form(&params),
            )
            .await?;

        Ok(req.json::<MyListStatus>().await?)
    }

    async fn set_episode_number(
        &mut self,
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
        &mut self,
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
