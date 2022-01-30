use gettextrs::gettext;
use glib::{Object, ObjectExt};
use gtk::{gdk, glib};

mod imp;

glib::wrapper! {
    pub struct ScrobblePage(ObjectSubclass<imp::ScrobblePage>)
        @extends gtk::Box, gtk::Widget,
        @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

#[allow(clippy::new_without_default)]
impl ScrobblePage {
    pub const STATUS_SUMMARY_PROPERTY: &'static str = "status-summary";
    pub const TITLE_PROPERTY: &'static str = "title";
    pub const EPISODE_PROPERTY: &'static str = "episode";
    pub const PLAYER_PROPERTY: &'static str = "player";
    pub const STATUS_PROPERTY: &'static str = "status";
    pub const WEBSITE_URL_PROPERTY: &'static str = "website-url";
    pub const IMAGE_PROPERTY: &'static str = "image";

    pub fn new() -> Self {
        let scrobble_page: Self = Object::new(&[]).expect("Failed to create `ScrobblePage`.");
        scrobble_page.set_anime_info_none();
        scrobble_page
    }

    pub fn set_anime_info(
        &self,
        title: &str,
        episode_number: &str,
        player_name: &str,
        status: &str,
        website_url: &str,
        picture: Option<gdk::Texture>,
    ) {
        self.set_status_summary(&gettext("Scrobbling now"));
        self.set_title(title);
        self.set_episode(episode_number);
        self.set_player(player_name);
        self.set_status(status);
        self.set_website_url(website_url);
        if picture.is_some() {
            self.set_image(picture);
        }
    }

    pub fn set_anime_info_none(&self) {
        self.set_status_summary(&gettext("Not scrobbling now"));
        self.set_title(&gettext("N/A"));
        self.set_episode(&gettext("N/A"));
        self.set_player(&gettext("N/A"));
        self.set_status(&gettext("N/A"));
        self.set_website_url("");
        self.set_image(None);
    }

    fn set_status_summary(&self, status_summary: &str) {
        self.set_property(Self::STATUS_SUMMARY_PROPERTY, status_summary);
    }

    fn set_title(&self, title: &str) {
        self.set_property(Self::TITLE_PROPERTY, title);
    }

    fn set_episode(&self, episode: &str) {
        self.set_property(Self::EPISODE_PROPERTY, episode);
    }

    fn set_player(&self, player: &str) {
        self.set_property(Self::PLAYER_PROPERTY, player);
    }

    fn set_status(&self, status: &str) {
        self.set_property(Self::STATUS_PROPERTY, status);
    }

    fn set_website_url(&self, website_url: &str) {
        self.set_property(Self::WEBSITE_URL_PROPERTY, website_url);
    }

    fn set_image(&self, image: Option<gdk::Texture>) {
        self.set_property(Self::IMAGE_PROPERTY, image);
    }
}
