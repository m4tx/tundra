use crate::clients::mal_client::MalClient;

mod clients;
mod player_controller;
mod title_recognizer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _client = MalClient::new("test", "test").await?;

    Ok(())
}
