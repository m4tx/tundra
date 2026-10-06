use gettextrs::gettext;
use gtk::gio::{Menu, SimpleAction};
use gtk::glib::clone;
use gtk::{
    Application, MenuButton, Orientation, PopoverMenu, Stack, StackTransitionType, Switch, gdk,
};
use libadwaita::prelude::*;
use libadwaita::{ApplicationWindow, Banner, HeaderBar};

use crate::clients::WebsiteUrl;
use crate::gtk_gui::login_page::LoginPage;
use crate::gtk_gui::scrobble_page::ScrobblePage;

pub struct MainWindow {
    app: Application,
    window: ApplicationWindow,
    enable_switch: gtk::Switch,
    overflow_button: gtk::MenuButton,
    error_banner: Banner,
    main_stack: gtk::Stack,
    login_page: LoginPage,
    scrobble_page: ScrobblePage,
}

const DEFAULT_WIDTH: i32 = 425;
const DEFAULT_HEIGHT: i32 = 275;

impl MainWindow {
    pub fn new(app: &Application) -> Self {
        let login_page = LoginPage::new();
        let scrobble_page = ScrobblePage::new();

        let main_stack = Self::make_main_stack();
        main_stack.add_child(&login_page);
        main_stack.add_child(&scrobble_page);

        let error_banner = Self::make_error_banner();

        let content = gtk::Box::new(Orientation::Vertical, 0);
        content.append(&error_banner);
        content.append(&main_stack);

        let enable_switch = Self::make_enable_switch();
        let overflow_button = Self::make_overflow_button();

        let header_bar = HeaderBar::builder()
            .title_widget(&libadwaita::WindowTitle::new(&gettext("Tundra"), ""))
            .build();
        header_bar.pack_start(&enable_switch);
        header_bar.pack_end(&overflow_button);

        let window_content = gtk::Box::new(Orientation::Vertical, 0);
        window_content.append(&header_bar);
        window_content.append(&content);

        let window = ApplicationWindow::builder()
            .application(app)
            .default_width(DEFAULT_WIDTH)
            .default_height(DEFAULT_HEIGHT)
            .content(&window_content)
            .build();

        let main_window = Self {
            app: app.clone(),
            window,
            error_banner,
            enable_switch,
            overflow_button,
            main_stack,
            login_page,
            scrobble_page,
        };

        main_window.set_anime_info_none();

        main_window
    }

    fn make_main_stack() -> Stack {
        let main_stack = Stack::new();
        main_stack.set_transition_type(StackTransitionType::SlideLeftRight);
        main_stack
    }

    fn make_error_banner() -> Banner {
        let banner = Banner::new("");
        banner.set_use_markup(false);
        banner.set_button_label(Some(&gettext("_Dismiss")));
        banner.connect_button_clicked(|banner| banner.set_revealed(false));
        banner
    }

    fn make_overflow_button() -> MenuButton {
        let menu_model = Menu::new();
        menu_model.append(Some(&gettext("_Sign out")), Some("app.sign-out"));
        menu_model.append(Some(&gettext("Show _logs")), Some("app.show-logs"));
        menu_model.append(Some(&gettext("_About Tundra")), Some("app.about"));
        let popover_menu = PopoverMenu::builder().menu_model(&menu_model).build();

        let overflow_button = MenuButton::new();
        overflow_button.set_icon_name("open-menu-symbolic");
        overflow_button.set_popover(Some(&popover_menu));

        overflow_button
    }

    fn make_enable_switch() -> Switch {
        let enable_switch = Switch::new();
        enable_switch.set_tooltip_text(Some(&gettext("Enable scrobbling")));
        enable_switch.set_active(true);
        enable_switch
    }

    pub fn connect_quit<F: Fn() + 'static>(&self, f: F) {
        let action = SimpleAction::new("quit", None);
        action.connect_activate(clone!(move |_, _| {
            f();
        }));

        self.app.add_action(&action);
        self.app.set_accels_for_action("app.quit", &["<primary>Q"]);
    }

    pub fn connect_about<F: Fn() + 'static>(&self, f: F) {
        let action = SimpleAction::new("about", None);
        action.connect_activate(clone!(move |_, _| {
            f();
        }));

        self.app.add_action(&action);
    }

    pub fn connect_show_logs<F: Fn() + 'static>(&self, f: F) {
        let action = SimpleAction::new("show-logs", None);
        action.connect_activate(clone!(move |_, _| {
            f();
        }));

        self.app.add_action(&action);
    }

    pub fn connect_sign_out<F: Fn() + 'static>(&self, f: F) {
        let action = SimpleAction::new("sign-out", None);
        action.connect_activate(clone!(move |_, _| {
            f();
        }));

        self.app.add_action(&action);
    }

    pub fn connect_sign_in<F: Fn() + Clone + 'static>(&self, f: F) {
        self.login_page.connect_activate(move || {
            f();
        });
    }

    pub fn connect_enable_switch<F: Fn(bool) + 'static>(&self, f: F) {
        self.enable_switch.connect_state_set(move |_, state| {
            f(state);

            gtk::glib::Propagation::Proceed
        });
    }

    pub fn is_scrobbling_enabled(&self) -> bool {
        self.enable_switch.state()
    }

    pub fn set_login_page_loading(&self, loading: bool) {
        self.login_page.set_loading(loading);
        if !loading {
            self.login_page.set_info(None);
        }
    }

    pub fn show_login_info(&self, info: &str) {
        self.login_page.set_info(Some(info));
    }

    pub fn show_error(&self, error_string: &str) {
        self.error_banner.set_title(error_string);
        self.error_banner.set_revealed(true);
    }

    pub fn switch_to_scrobble_page(&self) {
        self.main_stack.set_visible_child(&self.scrobble_page);
        self.enable_switch.set_visible(true);
        self.overflow_button.set_visible(true);
        self.error_banner.set_revealed(false);
    }

    pub fn switch_to_login_page(&self) {
        self.main_stack.set_visible_child(&self.login_page);
        self.enable_switch.set_visible(false);
        self.overflow_button.set_visible(false);
        self.error_banner.set_revealed(false);
    }

    pub fn set_anime_info(
        &self,
        title: &str,
        episode: &str,
        player_name: &str,
        status: &str,
        website_url: &WebsiteUrl,
        picture: Option<gtk::glib::Bytes>,
    ) {
        let picture_texture = picture.map(|bytes| {
            let stream = gtk::gio::MemoryInputStream::from_bytes(&bytes);
            let pixbuf =
                gtk::gdk_pixbuf::Pixbuf::from_stream(&stream, gtk::gio::Cancellable::NONE).unwrap();
            gdk::Texture::for_pixbuf(&pixbuf)
        });

        self.scrobble_page.set_anime_info(
            title,
            episode,
            player_name,
            status,
            &website_url.0,
            picture_texture,
        );
    }

    pub fn set_anime_info_none(&self) {
        self.scrobble_page.set_anime_info_none();
    }

    pub fn show(&self) {
        self.window.present();
    }

    pub fn window(&self) -> ApplicationWindow {
        self.window.clone()
    }
}
