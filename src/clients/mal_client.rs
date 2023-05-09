use core::fmt;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::error::Error;
use std::fmt::Formatter;
use std::str::FromStr;
use std::sync::{Arc, RwLock};

use log::info;
use reqwest::StatusCode;
use serde::Deserialize;

use async_trait::async_trait;
use gettextrs::gettext;
use lazy_static::lazy_static;
use tokio::try_join;

use crate::anime_relations::{AnimeDbs, AnimeRelations};
use crate::clients::{AnimeDbClient, AnimeId, AnimeInfo, PictureUrl, WebsiteUrl};
use crate::config::Config;
use crate::constants::{MAL_CLIENT_ID, USER_AGENT};
use crate::title_recognizer::Title;

static MAL_URL: &str = "https://api.myanimelist.net/v2";
static CLIENT_ID_HEADER: &str = "X-MAL-Client-ID";

#[derive(Clone, Debug, Deserialize)]
struct AuthenticationResponse {
    access_token: String,
    // expires_in: i64,
    refresh_token: String,
    // token_type: String,
}

#[derive(Clone, Debug, Deserialize)]
struct SearchResponse {
    data: Vec<SearchResponseObject>,
}

#[derive(Clone, Debug, Deserialize)]
struct SearchResponseObject {
    node: AnimeObject,
}

#[derive(Clone, Debug, Deserialize)]
struct RelatedAnime {
    node: RelatedAnimeObject,
    relation_type: String,
}

#[derive(Clone, Debug, Deserialize)]
struct RelatedAnimeObject {
    id: i64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum MediaType {
    TV,
    Ova,
    Movie,
    Special,
    Ona,
    Music,
    Unknown,
}

#[derive(Clone, Debug, Deserialize)]
struct AnimeObject {
    id: i64,
    title: String,
    num_episodes: i32,
    main_picture: PictureObject,
    my_list_status: Option<MyListStatus>,
    media_type: MediaType,
    popularity: i64,
    #[serde(default)]
    related_anime: Vec<RelatedAnime>,
}

#[derive(Clone, Debug, Deserialize)]
struct PictureObject {
    large: String,
    // medium: String,
}

#[derive(Clone, Debug, Deserialize)]
struct MyListStatus {
    // status: String,
    num_episodes_watched: i32,
}

pub struct MalClient {
    config: Arc<RwLock<Config>>,
    client: reqwest::Client,
    anime_relations: Arc<AnimeRelations>,
    title_cache: HashMap<Title, AnimeInfo>,
}

#[derive(Debug)]
pub enum MalClientError {
    HttpClientError(reqwest::Error),
    AuthenticationError(reqwest::Error),
}

impl From<reqwest::Error> for MalClientError {
    fn from(e: reqwest::Error) -> Self {
        MalClientError::HttpClientError(e)
    }
}

lazy_static! {
    static ref AUTHENTICATION_ERROR_STRING: String = gettext(
        "Could not authenticate to MyAnimeList. \
        Make sure the username and password you entered is correct.",
    );
}

impl fmt::Display for MalClientError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            MalClientError::HttpClientError(error) => {
                write!(
                    f,
                    "{}",
                    gettext!("Could not communicate with MAL: {}", error)
                )
            }
            MalClientError::AuthenticationError(_) => {
                write!(f, "{}", *AUTHENTICATION_ERROR_STRING)
            }
        }
    }
}

impl std::error::Error for MalClientError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            MalClientError::HttpClientError(error) => Some(error),
            MalClientError::AuthenticationError(error) => Some(error),
        }
    }
}

const RELATION_TYPE_SEQUEL: &str = "sequel";

pub type MalClientResult<T> = Result<T, MalClientError>;

impl MalClient {
    pub fn new(
        config: Arc<RwLock<Config>>,
        anime_relations: Arc<AnimeRelations>,
    ) -> MalClientResult<Self> {
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

    pub async fn authenticate(&self, username: &str, password: &str) -> MalClientResult<()> {
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

    async fn refresh_auth(&self) -> MalClientResult<()> {
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
        &self,
        url: &str,
        params: &[(&str, &str)],
    ) -> MalClientResult<()> {
        let req = self.client.post(url).form(&params);
        let response = req
            .send()
            .await?
            .error_for_status()
            .map_err(MalClientError::AuthenticationError)?;

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
        &self,
        request: reqwest::RequestBuilder,
    ) -> MalClientResult<reqwest::Response> {
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

    async fn search(&self, query: &str) -> MalClientResult<SearchResponse> {
        let params = [
            ("q", query),
            (
                "fields",
                "title,main_picture,alternative_titles,num_episodes,my_list_status,media_type,popularity",
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

    async fn get_by_id(&self, id: i64) -> MalClientResult<AnimeObject> {
        let params = [(
            "fields",
            "title,main_picture,alternative_titles,average_episode_duration,num_episodes,my_list_status,media_type,related_anime,popularity",
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
        &self,
        id: i64,
        status: &str,
        num_episodes_watched: i32,
    ) -> MalClientResult<MyListStatus> {
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
        &self,
        anime_object: &AnimeObject,
        num_episodes_watched: i32,
    ) -> MalClientResult<bool> {
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

    async fn get_anime_object(&self, title: &Title) -> MalClientResult<Option<(AnimeObject, i32)>> {
        let (anime_1, anime_2) =
            try_join!(self.find_anime_with_season(title), self.find_anime(title))?;
        let anime_objects: Vec<AnimeObject> = [anime_1, anime_2].into_iter().flatten().collect();

        for anime_object in anime_objects {
            let (anime_object, episode_number) = self
                .apply_anime_relation(&title.clone(), anime_object)
                .await?;
            if Self::is_in_my_list(&anime_object) {
                return Ok(Some((anime_object, episode_number)));
            }
        }
        Ok(None)
    }

    async fn find_anime(&self, title: &Title) -> MalClientResult<Option<AnimeObject>> {
        let results = self.search(&title.title).await?;

        let first_result = results.data.into_iter().next();
        if let Some(search_response_object) = first_result {
            Ok(Some(search_response_object.node))
        } else {
            Ok(None)
        }
    }

    async fn find_anime_with_season(&self, title: &Title) -> MalClientResult<Option<AnimeObject>> {
        let results = self.search(&title.title).await?;
        let mut data = results.data;
        data.sort_by(|a, b| {
            let a_rel = Self::search_relevance(&title.title, &a.node);
            let b_rel = Self::search_relevance(&title.title, &b.node);
            b_rel.partial_cmp(&a_rel).unwrap_or(Ordering::Equal)
        });

        let first_result = data.into_iter().next();
        if let Some(search_response_object) = first_result {
            let anime_object = search_response_object.node;

            Ok(self
                .get_nth_season(anime_object.id, title.season_number)
                .await?)
        } else {
            Ok(None)
        }
    }

    fn search_relevance(query: &str, anime_object: &AnimeObject) -> f32 {
        let dist =
            edit_distance::edit_distance(&anime_object.title.to_lowercase(), &query.to_lowercase());
        let edit_distance_relevance = 1.0 / (dist + 1) as f32;
        let popularity_relevance = 1.0 / anime_object.popularity as f32;

        edit_distance_relevance * 0.5 + popularity_relevance * 0.5
    }

    async fn get_nth_season(
        &self,
        anime_id: i64,
        season_number: i32,
    ) -> MalClientResult<Option<AnimeObject>> {
        let mut current_season = 0;
        let mut current_id = anime_id;

        while current_season <= season_number {
            let anime_object = self.get_by_id(current_id).await?;
            if anime_object.media_type != MediaType::Ova
                && anime_object.media_type != MediaType::Music
                && anime_object.media_type != MediaType::Special
            {
                current_season += 1;
            }
            if current_season == season_number {
                return Ok(Some(anime_object));
            }

            let sequel = anime_object
                .related_anime
                .iter()
                .find(|x| x.relation_type == RELATION_TYPE_SEQUEL);

            if let Some(sequel) = sequel {
                current_id = sequel.node.id;
            } else {
                return Ok(None);
            }
        }

        Ok(None)
    }

    fn is_in_my_list(anime_object: &AnimeObject) -> bool {
        anime_object.my_list_status.is_some()
    }

    async fn apply_anime_relation(
        &self,
        title: &Title,
        anime_object: AnimeObject,
    ) -> MalClientResult<(AnimeObject, i32)> {
        let relation_rule = self
            .anime_relations
            .get_rule(&AnimeDbs::Mal, anime_object.id);

        if let Some(rule) = relation_rule {
            let (new_id, new_ep) =
                rule.convert_episode_number(&AnimeDbs::Mal, anime_object.id, title.episode_number);
            let new_anime_object = self.get_by_id(new_id).await?;

            Ok((new_anime_object, new_ep))
        } else {
            Ok((anime_object, title.episode_number))
        }
    }
}

#[async_trait]
impl AnimeDbClient for MalClient {
    async fn get_anime_info(&mut self, title: &Title) -> anyhow::Result<Option<AnimeInfo>> {
        if self.title_cache.contains_key(title) {
            return Ok(Some(self.title_cache[title].clone()));
        }

        let anime_object = self.get_anime_object(title).await?;
        let anime_info = anime_object.map(|(anime_object, episode_number)| {
            let id = AnimeId(anime_object.id.to_string());
            let website_url = WebsiteUrl(format!("https://myanimelist.net/anime/{}", &id));
            let picture_url = PictureUrl(anime_object.main_picture.large);
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

    async fn set_title_watched(&mut self, anime_info: &AnimeInfo) -> anyhow::Result<bool> {
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
