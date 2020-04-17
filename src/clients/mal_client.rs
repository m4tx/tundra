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
}

#[async_trait]
impl AnimeDbClient for MalClient {
    async fn set_title_watched(_title: Title) -> Result<(), Box<dyn std::error::Error>> {
        unimplemented!()
    }
}
