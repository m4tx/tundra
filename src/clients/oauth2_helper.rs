use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use axum::extract::{Query, State};
use axum::response::Html;
use axum::routing::get;
use axum::Router;
use gettextrs::gettext;
use log::info;
use oauth2::basic::{BasicClient, BasicTokenResponse};
use oauth2::reqwest::async_http_client;
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, CsrfToken, PkceCodeChallenge, PkceCodeVerifier,
    RedirectUrl, TokenResponse, TokenUrl,
};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use url::Url;

/// List of random ports to redirect to.
const REDIRECT_PORTS: [u16; 10] = [
    12177, 13326, 16474, 22626, 22823, 29728, 32600, 41100, 45186, 63355,
];

#[derive(Debug)]
pub enum OAuth2FlowError {
    ServerStartFailed(std::io::Error),
    ServerError(std::io::Error),
    OAuth2RequestError(
        oauth2::basic::BasicRequestTokenError<oauth2::reqwest::AsyncHttpClientError>,
    ),
    VerificationFailed,
    WaitingForServerStopFailed(tokio::task::JoinError),
}

impl Display for OAuth2FlowError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            OAuth2FlowError::ServerStartFailed(e) => {
                write!(f, "Could not start OAuth2 code receiver server: {e}")
            }
            OAuth2FlowError::ServerError(e) => {
                write!(f, "OAuth2 code receiver server error: {e}")
            }
            OAuth2FlowError::OAuth2RequestError(e) => {
                write!(f, "OAuth2 request error: {e}")
            }
            OAuth2FlowError::VerificationFailed => {
                write!(f, "OAuth2 code verification failed")
            }
            OAuth2FlowError::WaitingForServerStopFailed(e) => {
                write!(
                    f,
                    "Failed waiting for OAuth2 code receiver server to stop: {e}"
                )
            }
        }
    }
}

impl Error for OAuth2FlowError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            OAuth2FlowError::ServerStartFailed(e) => Some(e),
            OAuth2FlowError::ServerError(e) => Some(e),
            OAuth2FlowError::OAuth2RequestError(e) => Some(e),
            OAuth2FlowError::VerificationFailed => None,
            OAuth2FlowError::WaitingForServerStopFailed(e) => Some(e),
        }
    }
}

type OAuth2Result<T> = Result<T, OAuth2FlowError>;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum PkceCodeChallengeType {
    Plain,
}

#[derive(Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct AccessToken(String);

impl AccessToken {
    #[must_use]
    pub fn new(secret: String) -> Self {
        Self(secret)
    }

    #[must_use]
    pub fn secret(&self) -> &str {
        &self.0
    }
}

impl Display for AccessToken {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "<OAuth2 Access Token>")
    }
}

impl Debug for AccessToken {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "AccessToken(<secret>)")
    }
}

impl From<&oauth2::AccessToken> for AccessToken {
    fn from(value: &oauth2::AccessToken) -> Self {
        Self(value.secret().to_owned())
    }
}

impl From<&AccessToken> for oauth2::AccessToken {
    fn from(value: &AccessToken) -> Self {
        Self::new(value.0.clone())
    }
}

#[derive(Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct RefreshToken(String);

impl RefreshToken {
    #[must_use]
    pub fn new(secret: String) -> Self {
        Self(secret)
    }

    #[must_use]
    pub fn secret(&self) -> &str {
        &self.0
    }
}

impl Display for RefreshToken {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "<OAuth2 Refresh Token>")
    }
}

impl Debug for RefreshToken {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "RefreshToken(<secret>)")
    }
}

impl From<&oauth2::RefreshToken> for RefreshToken {
    fn from(value: &oauth2::RefreshToken) -> Self {
        Self(value.secret().to_owned())
    }
}

impl From<&RefreshToken> for oauth2::RefreshToken {
    fn from(value: &RefreshToken) -> Self {
        Self::new(value.0.clone())
    }
}

#[must_use]
#[derive(Debug, Clone)]
pub struct OAuth2Token {
    pub access_token: AccessToken,
    pub expires_in: Option<Duration>,
    pub refresh_token: Option<RefreshToken>,
}

impl From<BasicTokenResponse> for OAuth2Token {
    fn from(token_result: BasicTokenResponse) -> Self {
        OAuth2Token {
            access_token: token_result.access_token().into(),
            expires_in: token_result.expires_in(),
            refresh_token: token_result.refresh_token().map(std::convert::Into::into),
        }
    }
}

#[derive(Debug)]
pub struct OAuth2Helper {
    client_id: Option<ClientId>,
    auth_url: Option<AuthUrl>,
    token_url: Option<TokenUrl>,
    pkce_code_challenge_type: Option<PkceCodeChallengeType>,
}

impl OAuth2Helper {
    #[must_use]
    pub fn new() -> Self {
        Self {
            client_id: None,
            auth_url: None,
            token_url: None,
            pkce_code_challenge_type: None,
        }
    }

    #[must_use]
    pub fn set_client_id(mut self, client_id: String) -> Self {
        self.client_id = Some(ClientId::new(client_id));
        self
    }

    #[must_use]
    pub fn set_auth_url(mut self, auth_url: String) -> Self {
        self.auth_url = Some(AuthUrl::new(auth_url).expect("invalid auth_url provided"));
        self
    }

    #[must_use]
    pub fn set_token_url(mut self, token_url: String) -> Self {
        self.token_url = Some(TokenUrl::new(token_url).expect("invalid token_url provided"));
        self
    }

    #[must_use]
    pub fn set_pkce_code_challenge_type(
        mut self,
        pkce_code_challenge_type: PkceCodeChallengeType,
    ) -> Self {
        self.pkce_code_challenge_type = Some(pkce_code_challenge_type);
        self
    }

    pub async fn start_auth(mut self) -> OAuth2Result<OAuth2CodeReceiver> {
        let server_builder = CodeReceiverServerBuilder::start().await?;

        let client = self.construct_client().set_redirect_uri(
            RedirectUrl::new(format!("http://127.0.0.1:{}", server_builder.get_port()))
                .expect("Redirect URL not valid"),
        );

        let challenge_type = self
            .pkce_code_challenge_type
            .expect("PKCE Code Challenge Type not set");
        let (pkce_challenge, pkce_verifier) = match challenge_type {
            PkceCodeChallengeType::Plain => PkceCodeChallenge::new_random_plain(),
        };

        let (auth_url, csrf_token) = client
            .authorize_url(CsrfToken::new_random)
            .set_pkce_challenge(pkce_challenge)
            .url();

        let code_receiver_server = server_builder.run().await?;

        Ok(OAuth2CodeReceiver {
            client,
            pkce_verifier,
            auth_url,
            csrf_token,
            code_receiver_server,
        })
    }

    pub async fn refresh_token(
        mut self,
        refresh_token: &RefreshToken,
    ) -> OAuth2Result<OAuth2Token> {
        let client = self.construct_client();

        let token_result = client
            .exchange_refresh_token(&refresh_token.into())
            .request_async(async_http_client)
            .await
            .map_err(OAuth2FlowError::OAuth2RequestError)?;

        Ok(token_result.into())
    }

    #[must_use]
    fn construct_client(&mut self) -> BasicClient {
        BasicClient::new(
            self.client_id.take().expect("Client ID not set"),
            None,
            self.auth_url.take().expect("Auth URL not set"),
            Some(self.token_url.take().expect("Token URL not set")),
        )
    }
}

#[must_use]
#[derive(Debug)]
pub struct OAuth2CodeReceiver {
    client: BasicClient,
    pkce_verifier: PkceCodeVerifier,
    auth_url: Url,
    csrf_token: CsrfToken,
    code_receiver_server: CodeReceiverServer,
}

impl OAuth2CodeReceiver {
    #[must_use]
    pub fn auth_url(&self) -> &Url {
        &self.auth_url
    }

    pub async fn wait_for_code(self) -> OAuth2Result<OAuth2Token> {
        let (authorization_code, csrf_state) = self.code_receiver_server.wait_for_code().await?;
        if csrf_state.secret() != self.csrf_token.secret() {
            return Err(OAuth2FlowError::VerificationFailed);
        }

        let token_result = self
            .client
            .exchange_code(authorization_code)
            .set_pkce_verifier(self.pkce_verifier)
            .request_async(async_http_client)
            .await
            .map_err(OAuth2FlowError::OAuth2RequestError)?;

        Ok(token_result.into())
    }
}

#[derive(Debug)]
enum StateData {
    WaitingForCode {
        tx: tokio::sync::oneshot::Sender<()>,
    },
    CodeReceived {
        code: String,
        state: String,
    },
}

impl StateData {
    #[must_use]
    fn waiting_for_code(tx: tokio::sync::oneshot::Sender<()>) -> Self {
        Self::WaitingForCode { tx }
    }

    #[must_use]
    fn code_received(code: String, state: String) -> Self {
        Self::CodeReceived { code, state }
    }

    #[must_use]
    fn is_waiting_for_code(&self) -> bool {
        matches!(self, Self::WaitingForCode { .. })
    }
}

#[derive(Debug)]
struct CodeReceiverServerBuilder {
    port: u16,
    listener: tokio::net::TcpListener,
}

impl CodeReceiverServerBuilder {
    pub async fn start() -> OAuth2Result<Self> {
        let addresses: Vec<SocketAddr> = REDIRECT_PORTS
            .iter()
            .map(|port| SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), *port))
            .collect();

        info!("Trying to start listening on addresses: {:?}", addresses);
        let listener = tokio::net::TcpListener::bind(addresses.as_slice())
            .await
            .map_err(OAuth2FlowError::ServerStartFailed)?;

        let port = listener
            .local_addr()
            .expect("Could not retrieve the listener port")
            .port();

        info!("Server running on port {}", port);

        Ok(Self { port, listener })
    }

    #[must_use]
    pub fn get_port(&self) -> u16 {
        self.port
    }

    async fn run(self) -> OAuth2Result<CodeReceiverServer> {
        CodeReceiverServer::new(self).await
    }
}

#[derive(Debug)]
struct CodeReceiverServer {
    state: Arc<Mutex<StateData>>,
    join_handle: JoinHandle<Result<(), std::io::Error>>,
}

impl CodeReceiverServer {
    async fn new(builder: CodeReceiverServerBuilder) -> OAuth2Result<Self> {
        let (tx, rx) = tokio::sync::oneshot::channel::<()>();
        let state = Arc::new(Mutex::new(StateData::waiting_for_code(tx)));

        let app = Router::new()
            .route("/", get(root))
            .with_state(state.clone());

        let join_handle = tokio::spawn(async {
            axum::serve(builder.listener, app.into_make_service())
                .with_graceful_shutdown(async {
                    rx.await.ok();
                    info!("OAuth2 code receiver server has been stopped");
                })
                .await
        });

        info!("OAuth2 code receiver server is running");

        Ok(Self { state, join_handle })
    }

    pub async fn wait_for_code(self) -> OAuth2Result<(AuthorizationCode, CsrfToken)> {
        self.join_handle
            .await
            .map_err(OAuth2FlowError::WaitingForServerStopFailed)?
            .map_err(OAuth2FlowError::ServerError)?;

        let state_mutex = Arc::into_inner(self.state)
            .expect("State data structure could not be extracted from Arc");
        let state = state_mutex.into_inner();
        Ok(match state {
            StateData::CodeReceived { code, state } => {
                (AuthorizationCode::new(code), CsrfToken::new(state))
            }
            _ => unreachable!(),
        })
    }
}

#[derive(Debug, Deserialize)]
struct OAuth2Response {
    code: String,
    state: String,
}

async fn root(
    Query(oauth2_response): Query<OAuth2Response>,
    State(state): State<Arc<Mutex<StateData>>>,
) -> Html<String> {
    let mut state = state.lock().await;

    if state.is_waiting_for_code() {
        match std::mem::replace(
            &mut *state,
            StateData::code_received(oauth2_response.code, oauth2_response.state),
        ) {
            StateData::WaitingForCode { tx } => {
                tx.send(())
                    .expect("Could not send shutdown signal to OAuth2 code receiver server");
            }
            _ => unreachable!(),
        }
    }

    Html(build_html_response())
}

#[must_use]
fn build_html_response() -> String {
    include_str!("oauth2_response.html")
        .replace("{{ title }}", &gettext("Tundra authentication"))
        .replace(
            "{{ body }}",
            &gettext("The authentication has succeeded; you may now close this tab."),
        )
}
