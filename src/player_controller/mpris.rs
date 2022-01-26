use dbus::arg;
use dbus::blocking;

pub trait OrgMprisMediaPlayer2 {
    fn identity(&self) -> Result<String, dbus::Error>;
    fn desktop_entry(&self) -> Result<String, dbus::Error>;
    fn supported_uri_schemes(&self) -> Result<Vec<String>, dbus::Error>;
    fn supported_mime_types(&self) -> Result<Vec<String>, dbus::Error>;
}

impl<'a, C: ::std::ops::Deref<Target = blocking::Connection>> OrgMprisMediaPlayer2
    for blocking::Proxy<'a, C>
{
    fn identity(&self) -> Result<String, dbus::Error> {
        <Self as blocking::stdintf::org_freedesktop_dbus::Properties>::get(
            self,
            "org.mpris.MediaPlayer2",
            "Identity",
        )
    }

    fn desktop_entry(&self) -> Result<String, dbus::Error> {
        <Self as blocking::stdintf::org_freedesktop_dbus::Properties>::get(
            self,
            "org.mpris.MediaPlayer2",
            "DesktopEntry",
        )
    }

    fn supported_uri_schemes(&self) -> Result<Vec<String>, dbus::Error> {
        <Self as blocking::stdintf::org_freedesktop_dbus::Properties>::get(
            self,
            "org.mpris.MediaPlayer2",
            "SupportedUriSchemes",
        )
    }

    fn supported_mime_types(&self) -> Result<Vec<String>, dbus::Error> {
        <Self as blocking::stdintf::org_freedesktop_dbus::Properties>::get(
            self,
            "org.mpris.MediaPlayer2",
            "SupportedMimeTypes",
        )
    }
}

pub trait OrgMprisMediaPlayer2Player {
    fn playback_status(&self) -> Result<String, dbus::Error>;
    fn metadata(
        &self,
    ) -> Result<
        ::std::collections::HashMap<String, arg::Variant<Box<dyn arg::RefArg + 'static>>>,
        dbus::Error,
    >;
    fn position(&self) -> Result<i64, dbus::Error>;
}

impl<'a, C: ::std::ops::Deref<Target = blocking::Connection>> OrgMprisMediaPlayer2Player
    for blocking::Proxy<'a, C>
{
    fn playback_status(&self) -> Result<String, dbus::Error> {
        <Self as blocking::stdintf::org_freedesktop_dbus::Properties>::get(
            self,
            "org.mpris.MediaPlayer2.Player",
            "PlaybackStatus",
        )
    }

    fn metadata(
        &self,
    ) -> Result<
        ::std::collections::HashMap<String, arg::Variant<Box<dyn arg::RefArg + 'static>>>,
        dbus::Error,
    > {
        <Self as blocking::stdintf::org_freedesktop_dbus::Properties>::get(
            self,
            "org.mpris.MediaPlayer2.Player",
            "Metadata",
        )
    }

    fn position(&self) -> Result<i64, dbus::Error> {
        <Self as blocking::stdintf::org_freedesktop_dbus::Properties>::get(
            self,
            "org.mpris.MediaPlayer2.Player",
            "Position",
        )
    }
}
