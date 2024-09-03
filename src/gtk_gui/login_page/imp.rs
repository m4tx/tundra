use gettextrs::gettext;
use glib::clone;
use glib::subclass::Signal;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use once_cell::sync::Lazy;

#[derive(Default)]
pub struct LoginPage {}

impl LoginPage {
    fn make_title() -> gtk::Label {
        let title = gtk::Label::new(None);
        title.set_markup(&format!("<b>{}</b>", gettext("Sign in to MyAnimeList")));
        title.set_halign(gtk::Align::Center);

        title
    }

    fn make_sign_in_button(&self) -> gtk::Button {
        let sign_in_button = gtk::Button::with_mnemonic(&gettext("_Sign in"));
        sign_in_button.set_receives_default(true);
        sign_in_button.set_hexpand(true);
        sign_in_button.style_context().add_class("suggested-action");

        let this = self.to_owned();
        sign_in_button.connect_clicked(clone!(
            #[strong]
            this,
            move |_| {
                this.emit_activate(&this.obj());
            }
        ));

        sign_in_button
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
        obj.attach(&self.make_sign_in_button(), 0, 1, 1, 1);
    }

    fn signals() -> &'static [Signal] {
        static SIGNALS: Lazy<Vec<Signal>> =
            Lazy::new(|| vec![Signal::builder(super::LoginPage::ACTIVATE_PROPERTY).build()]);
        SIGNALS.as_ref()
    }
}

impl WidgetImpl for LoginPage {}

impl GridImpl for LoginPage {}
