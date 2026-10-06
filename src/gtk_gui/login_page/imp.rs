use std::sync::LazyLock;

use gettextrs::gettext;
use glib::clone;
use glib::subclass::Signal;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;

#[derive(Default)]
pub struct LoginPage {
    sign_in_button: gtk::Button,
    info_label: gtk::Label,
}

impl LoginPage {
    fn make_title() -> gtk::Label {
        let title = gtk::Label::new(None);
        title.set_markup(&format!("<b>{}</b>", gettext("Sign in to MyAnimeList")));
        title.set_halign(gtk::Align::Center);

        title
    }

    fn setup_sign_in_button(&self) {
        let sign_in_button = &self.sign_in_button;
        sign_in_button.set_label(&gettext("_Sign in"));
        sign_in_button.set_use_underline(true);
        sign_in_button.set_receives_default(true);
        sign_in_button.set_hexpand(true);
        sign_in_button.add_css_class("suggested-action");

        let this = self.to_owned();
        sign_in_button.connect_clicked(clone!(
            #[strong]
            this,
            move |_| {
                this.emit_activate(&this.obj());
            }
        ));
    }

    fn setup_info_label(&self) {
        let info_label = &self.info_label;
        info_label.set_wrap(true);
        info_label.set_wrap_mode(gtk::pango::WrapMode::WordChar);
        info_label.set_width_chars(35);
        info_label.set_justify(gtk::Justification::Center);
        info_label.set_visible(false);
    }

    pub(super) fn set_loading(&self, loading: bool) {
        self.sign_in_button.set_sensitive(!loading);
    }

    pub(super) fn set_info(&self, info: Option<&str>) {
        self.info_label.set_text(info.unwrap_or_default());
        self.info_label.set_visible(info.is_some());
    }

    fn emit_activate(&self, obj: &super::LoginPage) {
        obj.emit_by_name::<()>(super::LoginPage::ACTIVATE_PROPERTY, &[]);
    }
}

#[glib::object_subclass]
impl ObjectSubclass for LoginPage {
    type ParentType = gtk::Grid;
    type Type = super::LoginPage;

    const NAME: &'static str = "TundraLoginPage";
}

impl ObjectImpl for LoginPage {
    fn constructed(&self) {
        self.parent_constructed();

        let obj = self.obj();
        obj.set_column_spacing(10);
        obj.set_row_spacing(10);
        obj.set_margin_start(10);
        obj.set_margin_end(10);
        obj.set_margin_top(10);
        obj.set_margin_bottom(10);
        obj.set_valign(gtk::Align::Center);

        let title = Self::make_title();
        obj.attach(&title, 0, 0, 1, 1);
        self.setup_sign_in_button();
        obj.attach(&self.sign_in_button, 0, 1, 1, 1);
        self.setup_info_label();
        obj.attach(&self.info_label, 0, 2, 1, 1);
    }

    fn signals() -> &'static [Signal] {
        static SIGNALS: LazyLock<Vec<Signal>> =
            LazyLock::new(|| vec![Signal::builder(super::LoginPage::ACTIVATE_PROPERTY).build()]);
        SIGNALS.as_ref()
    }
}

impl WidgetImpl for LoginPage {}

impl GridImpl for LoginPage {}
