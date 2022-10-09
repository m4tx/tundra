use gtk::gdk;
use gtk::gdk::Texture;
use gtk::prelude::*;
use libadwaita::ApplicationWindow;

use crate::constants::{APP_COPYRIGHT, APP_HOMEPAGE, APP_HOMEPAGE_NAME, APP_TITLE};
use crate::{APP_AUTHORS, APP_VERSION};

const LOGO_BYTES: &[u8] = include_bytes!("../../data/moe.tundra.Tundra.svg");

pub struct AboutDialog {
    dialog: gtk::AboutDialog,
}

impl AboutDialog {
    pub fn new(application: &gtk::Application, window: &ApplicationWindow) -> Self {
        let logo = Self::get_logo_texture();

        let dialog = gtk::AboutDialog::builder()
            .program_name(APP_TITLE)
            .version(APP_VERSION)
            .website(APP_HOMEPAGE)
            .website_label(APP_HOMEPAGE_NAME)
            .authors(vec![APP_AUTHORS.to_owned()])
            .copyright(APP_COPYRIGHT)
            .license_type(gtk::License::Gpl30)
            .logo(&logo)
            .modal(true)
            .application(application)
            .transient_for(window)
            .build();

        Self { dialog }
    }

    fn get_logo_texture() -> Texture {
        let bytes = glib::Bytes::from(LOGO_BYTES);
        let stream = gio::MemoryInputStream::from_bytes(&bytes);
        let pixbuf = gdk_pixbuf::Pixbuf::from_stream(&stream, gio::Cancellable::NONE).unwrap();
        gdk::Texture::for_pixbuf(&pixbuf)
    }

    pub fn run(&self) {
        self.dialog.show();
    }
}
