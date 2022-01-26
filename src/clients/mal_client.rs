use core::fmt;
use std::collections::HashMap;
use std::error::Error;
use std::str::FromStr;
use std::sync::{Arc, RwLock};

use log::info;
use reqwest::StatusCode;
use serde::Deserialize;

use async_trait::async_trait;

use crate::anime_relations::{AnimeDbs, AnimeRelations};
use crate::clients::{AnimeDbClient, AnimeId, AnimeInfo, PictureUrl, WebsiteUrl};
use crate::config::Config;
use crate::constants::{MAL_CLIENT_ID, USER_AGENT};
use crate::title_recognizer::Title;

static MAL_URL: &str = "https://api.myanimelist.net/v2";
static CLIENT_ID_HEADER: &str = "X-MAL-Client-ID";

#[derive(Debug, Deserialize)]
struct AuthenticationResponse {
    access_token: String,
    // expires_in: i64,
    refresh_token: String,
    // token_type: String,
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
    // average_episode_duration: i64,
    num_episodes: i32,
    main_picture: PictureObject,
    my_list_status: Option<MyListStatus>,
}

#[derive(Debug, Deserialize)]
struct PictureObject {
    // large: String,
    medium: String,
}

#[derive(Debug, Deserialize)]
struct MyListStatus {
    // status: String,
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

pub struct MalClient {
    config: Arc<RwLock<Config>>,
    client: reqwest::Client,
    anime_relations: Arc<AnimeRelations>,
    title_cache: HashMap<Title, AnimeInfo>,
}

impl MalClient {
    pub fn new(
        config: Arc<RwLock<Config>>,
        anime_relations: Arc<AnimeRelations>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        use reqwest::header;
        let mut headers = header::HeaderMap::new();
        headers.insert(
            CLIENT_ID_HEADER,
            header::HeaderValue::from_static(MAL_CLIENT_ID),
        );

        let client: reqwest::Client = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .default_headers(headers)
            .build()?;

        Ok(Self {
            config,
            client,
            anime_relations,
            title_cache: HashMap::new(),
        })
    }

    pub async fn authenticate(
        &mut self,
        username: &str,
        password: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        info!("Authenticating with MAL");

        let params = vec![
            ("client_id", MAL_CLIENT_ID),
            ("grant_type", "password"),
            ("username", username),
            ("password", password),
        ];

        Ok(self
            .make_auth_request(&format!("{}/auth/token", MAL_URL), &params)
            .await?)
    }

    fn access_token(&self) -> String {
        return self.config.read().unwrap().mal.access_token.clone();
    }

    fn refresh_token(&self) -> String {
        return self.config.read().unwrap().mal.refresh_token.clone();
    }

    async fn refresh_auth(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Refreshing MAL authentication token");

        let refresh_token = self.refresh_token();
        let params = vec![
            ("client_id", MAL_CLIENT_ID),
            ("grant_type", "refresh_token"),
            ("refresh_token", &refresh_token),
        ];

        Ok(self
            .make_auth_request("https://myanimelist.net/v1/oauth2/token", &params)
            .await?)
    }

    pub async fn make_auth_request(
        &mut self,
        url: &str,
        params: &[(&str, &str)],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let req = self.client.post(url).form(&params);
        let response = req.send().await?;
        if response.status().is_client_error() {
            return Err(Box::new(AuthenticationFailedError::new()));
        }

        let response_data: AuthenticationResponse = response
            .error_for_status()?
            .json::<AuthenticationResponse>()
            .await?;

        let mut config = self.config.write().unwrap();
        config.mal.access_token = response_data.access_token;
        config.mal.refresh_token = response_data.refresh_token;
        config.save();

        Ok(())
    }

    async fn make_request(
        &mut self,
        request: reqwest::RequestBuilder,
    ) -> Result<reqwest::Response, Box<dyn std::error::Error>> {
        let request_copy = request.try_clone().expect("Request could not be cloned");
        let response = request
            .header("Authorization", format!("Bearer {}", self.access_token()))
            .send()
            .await?;

        if response.status() == StatusCode::UNAUTHORIZED {
            self.refresh_auth().await?;
            return Ok(request_copy
                .header("Authorization", format!("Bearer {}", self.access_token()))
                .send()
                .await?
                .error_for_status()?);
        } else {
            Ok(response.error_for_status()?)
        }
    }

    async fn search(&mut self, query: &str) -> Result<SearchResponse, Box<dyn std::error::Error>> {
        let params = [
            ("q", query),
            (
                "fields",
                "title,main_picture,alternative_titles,num_episodes,my_list_status",
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
            "title,main_picture,alternative_titles,average_episode_duration,num_episodes,my_list_status",
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

    async fn get_anime_object(
        &mut self,
        title: &Title,
    ) -> Result<Option<(AnimeObject, i32)>, Box<dyn std::error::Error>> {
        let results = self.search(&title.title).await?;
        if !results.data.is_empty() {
            let anime_object = results.data.into_iter().next().unwrap().node;
            let relation_rule = self
                .anime_relations
                .get_rule(&AnimeDbs::Mal, anime_object.id);

            if let Some(rule) = relation_rule {
                let (new_id, new_ep) = rule.convert_episode_number(
                    &AnimeDbs::Mal,
                    anime_object.id,
                    title.episode_number,
                );
                let new_anime_object = self.get_by_id(new_id).await?;

                if new_anime_object.my_list_status.is_some() {
                    Ok(Some((new_anime_object, new_ep)))
                } else {
                    Ok(None)
                }
            } else if anime_object.my_list_status.is_some() {
                Ok(Some((anime_object, title.episode_number)))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }
}

#[async_trait]
impl AnimeDbClient for MalClient {
    async fn get_anime_info(&mut self, title: &Title) -> Result<Option<AnimeInfo>, Box<dyn Error>> {
        if self.title_cache.contains_key(title) {
            return Ok(Some(self.title_cache[title].clone()));
        }

        let anime_object = self.get_anime_object(title).await?;
        let anime_info = anime_object.map(|(anime_object, episode_number)| {
            let id = AnimeId(anime_object.id.to_string());
            let website_url = WebsiteUrl(format!("https://myanimelist.net/anime/{}", &id));
            let picture_url = PictureUrl(anime_object.main_picture.medium);
            AnimeInfo {
                id,
                title: anime_object.title,
                website_url,
                picture: picture_url,
                episode_watched: episode_number,
                total_episodes: anime_object.num_episodes,
            }
        });

        if anime_info.is_some() {
            self.title_cache
                .insert(title.clone(), anime_info.as_ref().unwrap().clone());
        }

        Ok(anime_info)
    }

    async fn set_title_watched(
        &mut self,
        anime_info: &AnimeInfo,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let anime_object = self.get_by_id(i64::from_str(&anime_info.id.0)?).await?;

        let episodes_watched = anime_object
            .my_list_status
            .as_ref()
            .unwrap()
            .num_episodes_watched;
        let episode_number = anime_info.episode_watched;
        if episodes_watched < episode_number {
            self.set_episode_number(&anime_object, episode_number)
                .await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}
