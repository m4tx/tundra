use crate::player_controller::PlayerController;
use crate::title_recognizer::{Title, TitleRecognizer};

mod player_controller;
mod title_recognizer;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let controller = PlayerController::new()?;
    let mut title_recognizer = TitleRecognizer::new();
    let players = controller.get_players()?;

    for player in players {
        println!(
            "Player \"{}\", currently playing? {}",
            player.player_name()?,
            player.is_currently_playing()?
        );

        let filename = player.filename_played();
        if filename.is_ok() {
            let title = title_recognizer.recognize(&filename.unwrap());
            match title {
                None => {}
                Some(t) => println!(
                    "Currently playing {}, episode {}",
                    t.title, t.episode_number
                ),
            }
        }
    }

    Ok(())
}
