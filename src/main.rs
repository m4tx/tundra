mod player_controller;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let controller = player_controller::PlayerController::new()?;
    let players = controller.get_players()?;

    for player in players {
        println!(
            "Player \"{}\", currently playing? {}",
            player.player_name()?,
            player.is_currently_playing()?
        );

        match player.filename_played() {
            Ok(f) => println!("{}", f),
            Err(_) => {}
        }
    }

    Ok(())
}
