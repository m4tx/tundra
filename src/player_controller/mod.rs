use anyhow::anyhow;
use std::path::Path;
use std::time::Duration;

use dbus::blocking::Connection;
use dbus::blocking::Proxy;
use mpris::OrgMprisMediaPlayer2;
use mpris::OrgMprisMediaPlayer2Player;

mod mpris;

type PlayerControllerResult<T> = anyhow::Result<T>;

pub struct PlayerController {
    connection: Connection,
}

impl PlayerController {
    pub fn new() -> PlayerControllerResult<Self> {
        let connection = Connection::new_session()?;
        Ok(Self { connection })
    }

    pub fn get_players(&self) -> anyhow::Result<Vec<Player>> {
        let proxy =
            self.connection
                .with_proxy("org.freedesktop.DBus", "/", Duration::from_millis(5000));
        let (names,): (Vec<String>,) =
            proxy.method_call("org.freedesktop.DBus", "ListNames", ())?;

        Ok(names
            .iter()
            .filter(|x| x.starts_with("org.mpris.MediaPlayer2"))
            .map(|x| Player::new(self, x.to_owned()))
            .collect())
    }
}

pub struct Player<'a> {
    dbus_proxy: Proxy<'a, &'a Connection>,
}

impl<'a> Player<'a> {
    fn new(player_controller: &'a PlayerController, name: String) -> Self {
        Self {
            dbus_proxy: player_controller.connection.with_proxy(
                name,
                "/org/mpris/MediaPlayer2",
                Duration::from_millis(5000),
            ),
        }
    }

    pub fn player_name(&self) -> PlayerControllerResult<String> {
        Ok(self.dbus_proxy.identity()?)
    }

    pub fn is_currently_playing(&self) -> PlayerControllerResult<bool> {
        Ok(self.dbus_proxy.playback_status()? == "Playing")
    }

    pub fn title_played(&self) -> PlayerControllerResult<String> {
        let metadata = self.dbus_proxy.metadata()?;
        let title_value = metadata
            .get("xesam:title")
            .ok_or(anyhow!("Title was not found"))?;
        let title: &str = title_value
            .0
            .as_str()
            .ok_or(anyhow!("Title is not string"))?;

        Ok(title.to_owned())
    }

    pub fn filename_played(&self) -> PlayerControllerResult<String> {
        let metadata = self.dbus_proxy.metadata()?;
        let url_value = metadata
            .get("xesam:url")
            .ok_or(anyhow!("URL was not found"))?;
        let url: &str = url_value.0.as_str().ok_or(anyhow!("URL is not string"))?;
        let url: String = if url.starts_with("file://") {
            percent_encoding::percent_decode_str(url)
                .decode_utf8()?
                .to_string()
        } else {
            url.to_owned()
        };
        let url = url.replace("file://", "");
        let path = Path::new(&url);

        Ok(path
            .file_name()
            .ok_or(anyhow!("URL does not have a filename"))?
            .to_str()
            .ok_or(anyhow!("filename is not string"))?
            .to_owned())
    }

    pub fn duration(&self) -> PlayerControllerResult<Duration> {
        let metadata = self.dbus_proxy.metadata()?;
        let x = metadata
            .get("mpris:length")
            .ok_or(anyhow!("duration was not found"))?;
        let duration_any = x.0.as_any();

        if let Some(duration) = duration_any.downcast_ref::<i64>() {
            Ok(Duration::from_micros(*duration as u64))
        } else if let Some(duration) = duration_any.downcast_ref::<u64>() {
            Ok(Duration::from_micros(*duration))
        } else if let Some(duration) = duration_any.downcast_ref::<f64>() {
            Ok(Duration::from_micros(*duration as u64))
        } else {
            Err(anyhow!("duration is not integer"))
        }
    }

    pub fn position(&self) -> PlayerControllerResult<f32> {
        let position = Duration::from_micros(self.dbus_proxy.position()? as u64);
        let duration = self.duration()?;

        Ok(position.as_secs_f32() / duration.as_secs_f32())
    }
}
