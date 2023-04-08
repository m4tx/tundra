use gettextrs::gettext;
use std::cell::RefCell;
use std::rc::Rc;

use glib::{clone, ParamSpec, ParamSpecObject, ParamSpecString, Value};
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gdk, glib};
use once_cell::sync::Lazy;

#[derive(Default)]
pub struct ScrobblePage {
    status_summary_label: Rc<RefCell<gtk::Label>>,
    title_label: Rc<RefCell<gtk::Label>>,
    episode_label: Rc<RefCell<gtk::Label>>,
    player_label: Rc<RefCell<gtk::Label>>,
    status_label: Rc<RefCell<gtk::Label>>,
    website_url: Rc<RefCell<String>>,
    picture: Rc<RefCell<gtk::Picture>>,
}

impl ScrobblePage {
    fn make_label(text: &str) -> gtk::Label {
        let label = gtk::Label::new(None);
        label.set_markup(&format!("<i>{}</i>", text));
        label.set_halign(gtk::Align::End);
        label.set_valign(gtk::Align::Start);
        label
    }

    fn make_status_summary_label() -> gtk::Label {
        let label = gtk::Label::new(None);
        label.set_halign(gtk::Align::Center);
        label
    }

    fn make_property_label() -> gtk::Label {
        let label = gtk::Label::new(None);
        label.set_halign(gtk::Align::Start);
        label.set_wrap(true);
        label
    }

    fn get_status_summary_label_markup(value: &Value) -> String {
        format!("<b>{}</b>", value.get::<&str>().unwrap())
    }

    fn make_picture(&self) -> gtk::Picture {
        let picture = gtk::Picture::new();
        picture.set_hexpand(true);
        picture.set_vexpand(true);

        let gesture = gtk::GestureClick::new();
        let website_url = self.website_url.clone();
        gesture.connect_released(clone!(@strong website_url => move |gesture, _, _, _| {
            gesture.set_state(gtk::EventSequenceState::Claimed);
            let url = website_url.borrow();
            if !url.is_empty() {
                gtk::show_uri(gtk::Window::NONE, &url, gdk::CURRENT_TIME);
            }
        }));
        picture.add_controller(gesture);

        *self.picture.borrow_mut() = picture.clone();

        picture
    }
}

#[glib::object_subclass]
impl ObjectSubclass for ScrobblePage {
    const NAME: &'static str = "TundraScrobblePage";
    type Type = super::ScrobblePage;
    type ParentType = gtk::Box;
}

impl ObjectImpl for ScrobblePage {
    fn constructed(&self) {
        self.parent_constructed();

        let obj = self.obj();
        obj.set_orientation(gtk::Orientation::Horizontal);
        obj.set_homogeneous(true);
        obj.set_spacing(15);
        obj.set_margin_start(10);
        obj.set_margin_end(10);
        obj.set_margin_top(10);
        obj.set_margin_bottom(10);

        let grid = gtk::Grid::new();
        grid.set_column_spacing(10);
        grid.set_row_spacing(3);

        let status_summary_label = Self::make_status_summary_label();
        grid.attach(&status_summary_label, 0, 0, 2, 1);
        *self.status_summary_label.borrow_mut() = status_summary_label;

        grid.attach(&Self::make_label(&gettext("Title:")), 0, 1, 1, 1);
        let title_label = Self::make_property_label();
        grid.attach(&title_label, 1, 1, 1, 1);
        *self.title_label.borrow_mut() = title_label;

        grid.attach(&Self::make_label(&gettext("Episode:")), 0, 2, 1, 1);
        let episode_label = Self::make_property_label();
        grid.attach(&episode_label, 1, 2, 1, 1);
        *self.episode_label.borrow_mut() = episode_label;

        grid.attach(&Self::make_label(&gettext("Player:")), 0, 3, 1, 1);
        let player_label = Self::make_property_label();
        grid.attach(&player_label, 1, 3, 1, 1);
        *self.player_label.borrow_mut() = player_label;

        grid.attach(&Self::make_label(&gettext("Status:")), 0, 4, 1, 1);
        let status_label = Self::make_property_label();
        grid.attach(&status_label, 1, 4, 1, 1);
        *self.status_label.borrow_mut() = status_label;

        grid.set_halign(gtk::Align::Center);
        grid.set_valign(gtk::Align::Center);
        grid.set_vexpand(true);

        obj.append(&self.make_picture());
        obj.append(&grid);
    }

    fn properties() -> &'static [ParamSpec] {
        static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
            vec![
                ParamSpecString::builder(super::ScrobblePage::STATUS_SUMMARY_PROPERTY)
                    .blurb("Status summary")
                    .default_value(Some(""))
                    .flags(glib::ParamFlags::READWRITE)
                    .build(),
                ParamSpecString::builder(super::ScrobblePage::TITLE_PROPERTY)
                    .blurb("Anime title")
                    .default_value(Some(""))
                    .flags(glib::ParamFlags::READWRITE)
                    .build(),
                ParamSpecString::builder(super::ScrobblePage::EPISODE_PROPERTY)
                    .blurb("Anime episode")
                    .default_value(Some(""))
                    .flags(glib::ParamFlags::READWRITE)
                    .build(),
                ParamSpecString::builder(super::ScrobblePage::PLAYER_PROPERTY)
                    .blurb("Player being used")
                    .default_value(Some(""))
                    .flags(glib::ParamFlags::READWRITE)
                    .build(),
                ParamSpecString::builder(super::ScrobblePage::STATUS_PROPERTY)
                    .blurb("Scrobble status")
                    .default_value(Some(""))
                    .flags(glib::ParamFlags::READWRITE)
                    .build(),
                ParamSpecString::builder(super::ScrobblePage::WEBSITE_URL_PROPERTY)
                    .blurb("Anime website URL")
                    .default_value(Some(""))
                    .flags(glib::ParamFlags::READWRITE)
                    .build(),
                ParamSpecObject::builder::<gdk::Paintable>(super::ScrobblePage::IMAGE_PROPERTY)
                    .blurb("Image paintable")
                    .flags(glib::ParamFlags::READWRITE)
                    .build(),
            ]
        });
        PROPERTIES.as_ref()
    }

    fn property(&self, _id: usize, pspec: &ParamSpec) -> Value {
        match pspec.name() {
            super::ScrobblePage::STATUS_SUMMARY_PROPERTY => {
                self.status_summary_label.borrow().text().to_value()
            }
            super::ScrobblePage::TITLE_PROPERTY => self.title_label.borrow().text().to_value(),
            super::ScrobblePage::EPISODE_PROPERTY => self.episode_label.borrow().text().to_value(),
            super::ScrobblePage::PLAYER_PROPERTY => self.player_label.borrow().text().to_value(),
            super::ScrobblePage::STATUS_PROPERTY => self.status_label.borrow().text().to_value(),
            super::ScrobblePage::WEBSITE_URL_PROPERTY => self.website_url.borrow().to_value(),
            super::ScrobblePage::IMAGE_PROPERTY => self.picture.borrow().paintable().to_value(),
            _ => unimplemented!(),
        }
    }

    fn set_property(&self, _id: usize, value: &Value, pspec: &ParamSpec) {
        match pspec.name() {
            super::ScrobblePage::STATUS_SUMMARY_PROPERTY => self
                .status_summary_label
                .borrow()
                .set_markup(&Self::get_status_summary_label_markup(value)),
            super::ScrobblePage::TITLE_PROPERTY => {
                self.title_label.borrow().set_text(value.get().unwrap())
            }
            super::ScrobblePage::EPISODE_PROPERTY => {
                self.episode_label.borrow().set_text(value.get().unwrap())
            }
            super::ScrobblePage::PLAYER_PROPERTY => {
                self.player_label.borrow().set_text(value.get().unwrap())
            }
            super::ScrobblePage::STATUS_PROPERTY => {
                self.status_label.borrow().set_text(value.get().unwrap())
            }
            super::ScrobblePage::WEBSITE_URL_PROPERTY => {
                let website_url: String = value.get().unwrap();

                let image = self.picture.borrow();
                if !website_url.is_empty() {
                    image.set_cursor_from_name(Some("pointer"));
                } else {
                    image.set_cursor_from_name(Some("default"));
                }

                *self.website_url.borrow_mut() = website_url;
            }
            super::ScrobblePage::IMAGE_PROPERTY => {
                let x: Option<&gdk::Paintable> = value.get().ok();
                self.picture.borrow().set_paintable(x)
            }
            _ => unimplemented!(),
        }
    }
}

impl WidgetImpl for ScrobblePage {}

impl BoxImpl for ScrobblePage {}
