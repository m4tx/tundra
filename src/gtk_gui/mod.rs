use std::cell::RefCell;
use std::collections::HashMap;
use std::env::args;
use std::rc::Rc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::{Arc, RwLock};

use async_std::sync::Mutex;
use gio::prelude::*;
use glib::clone;
use gtk::prelude::*;
use gtk::Builder;
use tokio::time;

use crate::app::{PlayedTitle, TundraApp};
use crate::constants::{APP_VERSION, REFRESH_INTERVAL, USER_AGENT};

static LOGO_BYTES: &[u8] = include_bytes!("../../data/logo-64.png");

struct MainWindow {
    window: gtk::ApplicationWindow,
    about_dialog: gtk::AboutDialog,

    overflow_menu: gtk::MenuButton,
    sign_out_button: gtk::ModelButton,
    about_button: gtk::ModelButton,

    sign_in_button: gtk::Button,
    enabled_switch: gtk::Switch,
    info_bar: gtk::InfoBar,
    info_bar_text: gtk::Label,
    main_stack: gtk::Stack,

    sign_in_page: gtk::Container,
    username_entry: gtk::Entry,
    password_entry: gtk::Entry,

    scrobble_page: gtk::Container,
    image: gtk::Image,
    status_summary_label: gtk::Label,
    title_label: gtk::Label,
    episode_number_label: gtk::Label,
    player_name_label: gtk::Label,
    status_label: gtk::Label,
}

impl MainWindow {
    fn set_sign_in_page_loading(&self, loading: bool) {
        self.username_entry.set_sensitive(!loading);
        self.password_entry.set_sensitive(!loading);
        self.sign_in_button.set_sensitive(!loading);
    }

    fn show_error(&self, error_string: &str) {
        self.info_bar_text.set_text(&error_string);
        self.info_bar.set_revealed(true);
    }

    fn switch_to_scrobble_page(&self) {
        self.main_stack.set_visible_child(&self.scrobble_page);
        self.sign_in_button.hide();
        self.enabled_switch.show();
        self.overflow_menu.show();
    }

    fn switch_to_sign_in_page(&self) {
        self.main_stack.set_visible_child(&self.sign_in_page);
        self.sign_in_button.show();
        self.enabled_switch.hide();
        self.overflow_menu.hide();
        self.username_entry.set_text("");
        self.password_entry.set_text("");
    }

    fn get_login_data(&self) -> (String, String) {
        let username = self.username_entry.get_text().unwrap().as_str().to_owned();
        let password = self.password_entry.get_text().unwrap().as_str().to_owned();

        (username.to_owned(), password.to_owned())
    }

    fn set_anime_info(
        &self,
        title: &str,
        episode_number: &str,
        player_name: &str,
        status: &str,
        picture: Option<&gdk_pixbuf::Pixbuf>,
    ) {
        self.status_summary_label.set_text("Scrobbling now");
        self.title_label.set_text(title);
        self.episode_number_label.set_text(episode_number);
        self.player_name_label.set_text(player_name);
        self.status_label.set_text(status);
        if picture.is_some() {
            self.image.set_from_pixbuf(picture);
        }
    }

    fn set_anime_info_none(&self) {
        self.status_summary_label.set_text("Not scrobbling now");
        self.title_label.set_text("N/A");
        self.episode_number_label.set_text("N/A");
        self.player_name_label.set_text("N/A");
        self.status_label.set_text("N/A");
        self.image.clear();
    }
}

#[derive(Clone)]
pub struct GtkApp {
    gtk_application: gtk::Application,
    app: Arc<Mutex<TundraApp>>,
    main_window: Rc<MainWindow>,
    images: Arc<RwLock<HashMap<String, glib::Bytes>>>,
    current_image_url: Rc<RefCell<String>>,
    scrobbling_enabled: Arc<AtomicBool>,
}

impl GtkApp {
    pub fn start(app: TundraApp) {
        let application = gtk::Application::new(Some("pl.m4tx.Tundra"), Default::default())
            .expect("Initialization failed...");
        let rc_app = Arc::new(Mutex::new(app));

        application.connect_activate(move |gtk_application| {
            let mut gtk_app = Self {
                app: rc_app.clone(),
                gtk_application: gtk_application.clone(),
                main_window: Rc::new(Self::build_main_window()),
                images: Arc::new(RwLock::new(HashMap::new())),
                current_image_url: Rc::new(RefCell::new(String::new())),
                scrobbling_enabled: Arc::new(AtomicBool::new(false)),
            };
            gtk_app.build_ui();
        });

        application.run(&args().collect::<Vec<_>>());
    }

    fn build_main_window() -> MainWindow {
        let glade_src = include_str!("ui.glade");
        let builder = Builder::new();
        builder
            .add_from_string(glade_src)
            .expect("Couldn't build UI from string");

        MainWindow {
            window: builder.get_object("window").unwrap(),
            about_dialog: builder.get_object("about_dialog").unwrap(),

            overflow_menu: builder.get_object("overflow_menu").unwrap(),
            sign_out_button: builder.get_object("sign_out_button").unwrap(),
            about_button: builder.get_object("about_button").unwrap(),

            sign_in_button: builder.get_object("sign_in_button").unwrap(),
            enabled_switch: builder.get_object("enabled_switch").unwrap(),
            info_bar: builder.get_object("info_bar").unwrap(),
            info_bar_text: builder.get_object("info_bar_text").unwrap(),
            main_stack: builder.get_object("main_stack").unwrap(),

            sign_in_page: builder.get_object("sign_in_page").unwrap(),
            username_entry: builder.get_object("username").unwrap(),
            password_entry: builder.get_object("password").unwrap(),

            scrobble_page: builder.get_object("scrobble_page").unwrap(),
            image: builder.get_object("image").unwrap(),
            status_summary_label: builder.get_object("status_summary_label").unwrap(),
            title_label: builder.get_object("title_label").unwrap(),
            episode_number_label: builder.get_object("episode_number_label").unwrap(),
            player_name_label: builder.get_object("player_name_label").unwrap(),
            status_label: builder.get_object("status_label").unwrap(),
        }
    }

    fn build_ui(&mut self) {
        self.main_window
            .window
            .set_application(Some(&self.gtk_application));

        self.main_window.about_dialog.set_version(Some(APP_VERSION));
        let bytes = glib::Bytes::from(LOGO_BYTES);
        let stream = gio::MemoryInputStream::new_from_bytes(&bytes);
        let pixbuf =
            gdk_pixbuf::Pixbuf::new_from_stream::<_, gio::Cancellable>(&stream, None)
                .unwrap();
        self.main_window.about_dialog.set_logo(Some(&pixbuf));

        self.main_window.info_bar.connect_response(|bar, response| {
            if response == gtk::ResponseType::Close {
                bar.set_revealed(false);
            }
        });

        self.main_window
            .sign_in_button
            .connect_clicked(clone!(@strong self as this => move |_| {
                this.clone().sign_in();
            }));

        self.main_window.enabled_switch.connect_state_set(
            clone!(@strong self as this => move |_, state| {
                this.set_scrobbling_enabled(state);
                if !state {
                    this.main_window.set_anime_info_none();
                    this.current_image_url.replace("".to_owned());
                }

                gtk::prelude::Inhibit(false)
            }),
        );

        self.main_window
            .username_entry
            .connect_changed(clone!(@strong self as this => move |_| {
                this.clone().check_sign_in_enabled();
            }));
        self.main_window
            .password_entry
            .connect_changed(clone!(@strong self as this => move |_| {
                this.clone().check_sign_in_enabled();
            }));
        self.main_window
            .username_entry
            .connect_activate(clone!(@strong self as this => move |_| {
                this.clone().sign_in();
            }));
        self.main_window
            .password_entry
            .connect_activate(clone!(@strong self as this => move |_| {
                this.clone().sign_in();
            }));

        self.main_window
            .about_button
            .connect_clicked(clone!(@strong self as this => move |_| {
                this.main_window.about_dialog.run();
                this.main_window.about_dialog.hide();
            }));

        self.main_window
            .sign_out_button
            .connect_clicked(clone!(@strong self as this => move |_| {
                this.switch_to_sign_in_page();
            }));

        self.main_window.set_anime_info_none();

        self.start_main();
    }

    fn start_main(&mut self) {
        self.run_daemon();

        let (tx, rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
        let app = self.app.clone();
        tokio::spawn(async move {
            let app = app.lock().await;
            let result = app.is_mal_authenticated();
            tx.send(result).expect("Couldn't send data to channel");
        });

        let this = self.clone();
        rx.attach(None, move |is_mal_authenticated| {
            if is_mal_authenticated {
                this.switch_to_scrobble_page();
            }
            this.main_window.window.show();

            glib::Continue(true)
        });
    }

    fn set_scrobbling_enabled(&self, state: bool) {
        self.scrobbling_enabled.store(state, Ordering::Relaxed);
    }

    fn switch_to_sign_in_page(&self) {
        self.main_window.switch_to_sign_in_page();
        self.set_scrobbling_enabled(false);
    }

    fn switch_to_scrobble_page(&self) {
        self.main_window.switch_to_scrobble_page();
        self.set_scrobbling_enabled(self.main_window.enabled_switch.get_active());
    }

    fn check_sign_in_enabled(&self) {
        let (username, password) = self.main_window.get_login_data();
        self.main_window
            .sign_in_button
            .set_sensitive(!username.is_empty() && !password.is_empty());
    }

    fn sign_in(&mut self) {
        let (username, password) = self.main_window.get_login_data();
        if username.is_empty() || password.is_empty() {
            return;
        }

        self.main_window.set_sign_in_page_loading(true);

        let (tx, rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
        let app = self.app.clone();
        tokio::spawn(async move {
            let mut app = app.lock().await;
            let result = app.authenticate_mal(&username, &password).await;

            let new_result = result.map_err(|x| x.to_string());

            tx.send(new_result).expect("Couldn't send data to channel");
        });

        let this = self.clone();
        rx.attach(None, move |result| {
            this.main_window.set_sign_in_page_loading(false);
            if let Err(error_string) = result {
                this.main_window.show_error(&error_string);
            } else {
                this.switch_to_scrobble_page();
            }

            glib::Continue(true)
        });
    }

    fn run_daemon(&mut self) {
        let app = self.app.clone();
        let images = self.images.clone();
        let scrobbling_enabled = self.scrobbling_enabled.clone();
        let (tx, rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
        tokio::spawn(async move {
            let mut interval = time::interval(REFRESH_INTERVAL);

            loop {
                interval.tick().await;
                if !scrobbling_enabled.load(Ordering::Relaxed) {
                    continue;
                }

                let result = Self::daemon_tick(&app, &images).await;
                let new_result = result.map_err(|x| x.to_string());
                tx.send(new_result).expect("Couldn't send data to channel");
            }
        });

        let main_window = self.main_window.clone();
        let images = self.images.clone();
        let current_image_url = self.current_image_url.clone();
        rx.attach(None, move |result| {
            Self::handle_ui_daemon_tick(&result, &main_window, &images, &current_image_url);

            glib::Continue(true)
        });
    }

    async fn daemon_tick(
        app: &Arc<Mutex<TundraApp>>,
        images: &Arc<RwLock<HashMap<String, glib::Bytes>>>,
    ) -> Result<Option<PlayedTitle>, Box<dyn std::error::Error>> {
        let mut app = app.lock().await;
        app.try_scrobble().await?;
        let played_title = app.get_played_title().await?;

        if let Some(played_title) = played_title.clone() {
            let picture_url = &played_title.anime_info.picture;
            let image_downloaded = images.read().unwrap().contains_key(picture_url);
            if !image_downloaded {
                let bytes = Self::get_picture(picture_url).await?;
                let vec = Vec::from(bytes.as_ref());
                let glib_bytes = glib::Bytes::from_owned(vec);
                images
                    .write()
                    .unwrap()
                    .insert(picture_url.clone(), glib_bytes);
            }
        }

        Ok(played_title)
    }

    async fn get_picture(url: &str) -> Result<bytes::Bytes, Box<dyn std::error::Error>> {
        let client: reqwest::Client = reqwest::Client::builder().user_agent(USER_AGENT).build()?;
        Ok(client.get(url).send().await?.bytes().await?)
    }

    fn handle_ui_daemon_tick(
        result: &Result<Option<PlayedTitle>, String>,
        main_window: &Rc<MainWindow>,
        images: &Arc<RwLock<HashMap<String, glib::Bytes>>>,
        current_image_url: &Rc<RefCell<String>>,
    ) {
        if let Err(error_string) = result {
            main_window.show_error(&error_string);
        } else if let Ok(Some(result)) = result {
            let anime_info = &result.anime_info;
            let title = &anime_info.title;
            let episode_number = &anime_info.episode_watched.to_string();
            let player_name = &result.player_name;
            let status = if result.scrobbled {
                "scrobbled"
            } else {
                "not yet scrobbled"
            };

            let picture = if *current_image_url.borrow() != anime_info.picture {
                current_image_url.replace(anime_info.picture.clone());
                let stream = gio::MemoryInputStream::new_from_bytes(
                    &images.read().unwrap()[&anime_info.picture],
                );
                let pixbuf =
                    gdk_pixbuf::Pixbuf::new_from_stream::<_, gio::Cancellable>(&stream, None)
                        .unwrap();
                Some(pixbuf)
            } else {
                None
            };

            main_window.set_anime_info(
                title,
                episode_number,
                player_name,
                status,
                picture.as_ref(),
            );
        } else {
            main_window.set_anime_info_none();
            current_image_url.replace("".to_owned());
        }
    }
}
