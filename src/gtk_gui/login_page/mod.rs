use glib::Object;
use glib::object::ObjectExt;
use gtk::glib;

mod imp;

glib::wrapper! {
    pub struct LoginPage(ObjectSubclass<imp::LoginPage>)
        @extends gtk::Grid, gtk::Widget,
        @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

#[allow(clippy::new_without_default)]
impl LoginPage {
    pub const ACTIVATE_PROPERTY: &'static str = "activate";

    pub fn new() -> Self {
        Object::new::<Self>()
    }

    pub fn connect_activate<F: Fn() + 'static>(&self, f: F) {
        self.connect_local(Self::ACTIVATE_PROPERTY, false, move |_args| {
            f();
            None
        });
    }
}
