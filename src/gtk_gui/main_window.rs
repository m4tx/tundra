use gettextrs::gettext;
use gtk::gio::{Menu, SimpleAction};
use gtk::glib::clone;
use gtk::{
    gdk, Application, InfoBar, Label, MenuButton, MessageType, Orientation, PopoverMenu, Stack,
    StackTransitionType, Switch,
};
use libadwaita::prelude::*;
use libadwaita::{ApplicationWindow, HeaderBar};

use crate::clients::WebsiteUrl;
use crate::gtk_gui::login_page::LoginPage;
use crate::gtk_gui::scrobble_page::ScrobblePage;

pub struct MainWindow {
    app: Application,
    window: ApplicationWindow,
    enable_switch: gtk::Switch,
    overflow_button: gtk::MenuButton,
    info_bar: gtk::InfoBar,
    info_bar_text: gtk::Label,
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

        let (info_bar, info_bar_text) = Self::make_info_bar();

        let content = gtk::Box::new(Orientation::Vertical, 0);
        content.append(&info_bar);
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
            info_bar,
            info_bar_text,
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

    fn make_info_bar() -> (InfoBar, Label) {
        let info_bar = InfoBar::new();
        info_bar.set_show_close_button(true);
        info_bar.set_revealed(false);
        info_bar.connect_response(|bar, response| {
            if response == gtk::ResponseType::Close {
                bar.set_revealed(false);
            }
        });

        let info_bar_text = Label::new(None);
        info_bar_text.set_wrap(true);
        info_bar_text.set_wrap_mode(gtk::pango::WrapMode::WordChar);
        info_bar_text.set_halign(gtk::Align::Center);
        info_bar_text.set_hexpand(true);
        info_bar_text.set_justify(gtk::Justification::Center);
        info_bar.add_child(&info_bar_text);

        (info_bar, info_bar_text)
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
        self.login_page.set_sensitive(!loading);
    }

    pub fn show_info(&self, error_string: &str) {
        self.info_bar_text.set_text(error_string);
        self.info_bar.set_message_type(MessageType::Info);
        self.info_bar.set_revealed(true);
    }

    pub fn show_error(&self, error_string: &str) {
        self.info_bar_text.set_text(error_string);
        self.info_bar.set_message_type(MessageType::Error);
        self.info_bar.set_revealed(true);
    }

    pub fn switch_to_scrobble_page(&self) {
        self.main_stack.set_visible_child(&self.scrobble_page);
        self.enable_switch.show();
        self.overflow_button.show();
        self.info_bar.set_revealed(false);
    }

    pub fn switch_to_login_page(&self) {
        self.main_stack.set_visible_child(&self.login_page);
        self.enable_switch.hide();
        self.overflow_button.hide();
        self.info_bar.set_revealed(false);
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
        self.window.show();
    }

    pub fn window(&self) -> ApplicationWindow {
        self.window.clone()
    }
}
