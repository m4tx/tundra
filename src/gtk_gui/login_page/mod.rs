use glib::{Object, ObjectExt};
use gtk::glib;

mod imp;

glib::wrapper! {
    pub struct LoginPage(ObjectSubclass<imp::LoginPage>)
        @extends gtk::Grid, gtk::Widget,
        @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget;
}

#[allow(clippy::new_without_default)]
impl LoginPage {
    pub const ACTIVATE_PROPERTY: &'static str = "activate";
    pub const PASSWORD_PROPERTY: &'static str = "password";
    pub const READY_PROPERTY: &'static str = "ready";
    pub const USERNAME_PROPERTY: &'static str = "username";

    pub fn new() -> Self {
        Object::new::<Self>()
    }

    pub fn username(&self) -> String {
        self.property(Self::USERNAME_PROPERTY)
    }

    pub fn password(&self) -> String {
        self.property(Self::PASSWORD_PROPERTY)
    }

    pub fn reset(&self) {
        self.set_property(Self::USERNAME_PROPERTY, "");
        self.set_property(Self::PASSWORD_PROPERTY, "");
    }

    pub fn connect_activate<F: Fn() + 'static>(&self, f: F) {
        self.connect_local(Self::ACTIVATE_PROPERTY, false, move |_args| {
            f();
            None
        });
    }
}
