use std::cell::RefCell;
use std::collections::HashMap;
use std::env::args;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};

use async_std::sync::Mutex;
use gettextrs::gettext;
use glib::clone;
use gtk::Application;
use libadwaita::prelude::*;
use log::error;
use tokio::time;

use about_dialog::AboutDialog;
use logs_window::LogsWindow;

use crate::app::PlayedTitle;
use crate::clients::PictureUrl;
use crate::constants::{REFRESH_INTERVAL, USER_AGENT};
use crate::gtk_gui::main_window::MainWindow;
use crate::TundraApp;

mod about_dialog;
mod login_page;
mod logs_window;
mod main_window;
mod scrobble_page;

#[derive(Clone)]
pub struct GtkApp {
    gtk_application: gtk::Application,
    app: Arc<Mutex<TundraApp>>,
    main_window: Rc<MainWindow>,
    images: Arc<RwLock<HashMap<PictureUrl, glib::Bytes>>>,
    current_image_url: Rc<RefCell<PictureUrl>>,
    scrobbling_enabled: Arc<AtomicBool>,
}

impl GtkApp {
    pub fn start(app: TundraApp) {
        let application = Application::builder()
            .application_id("moe.tundra.Tundra")
            .build();

        application.connect_startup(|_| {
            libadwaita::init().expect("Could not initialize libadwaita");
        });

        let rc_app = Arc::new(Mutex::new(app));

        application.connect_activate(move |gtk_application| {
            let mut gtk_app = Self {
                app: rc_app.clone(),
                gtk_application: gtk_application.clone(),
                main_window: Rc::new(MainWindow::new(gtk_application)),
                images: Arc::new(RwLock::new(HashMap::new())),
                current_image_url: Rc::new(RefCell::new(PictureUrl::default())),
                scrobbling_enabled: Arc::new(AtomicBool::new(false)),
            };
            gtk_app.build_ui();
        });

        application.run_with_args(&args().collect::<Vec<_>>());
    }

    fn build_ui(&mut self) {
        self.main_window
            .connect_sign_in(clone!(@strong self as this => move || {
                this.clone().sign_in();
            }));

        self.main_window
            .connect_enable_switch(clone!(@strong self as this => move |state| {
                this.set_scrobbling_enabled(state);
                if !state {
                    this.main_window.set_anime_info_none();
                    this.current_image_url.replace(PictureUrl::default());
                }
            }));

        let app = self.gtk_application.clone();
        let window = self.main_window.window();

        self.main_window
            .connect_quit(clone!(@strong app => move || {
                app.quit();
            }));
        self.main_window
            .connect_about(clone!(@strong app, @strong window => move || {
                AboutDialog::new(&app, &window).run();
            }));
        self.main_window
            .connect_show_logs(clone!(@strong app, @strong window => move || {
                LogsWindow::new(&app, &window).show();
            }));
        self.main_window
            .connect_sign_out(clone!(@strong self as this => move || {
                this.switch_to_sign_in_page();
            }));

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
            this.main_window.show();

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
        self.set_scrobbling_enabled(self.main_window.is_scrobbling_enabled());
    }

    fn sign_in(&mut self) {
        let (username, password) = self.main_window.login_data();
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
                let new_result = result.map_err(|error| {
                    error!("{}", error);
                    if let Some(source) = error.source() {
                        error!("{}", source);
                    }
                    error.to_string()
                });
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
        images: &Arc<RwLock<HashMap<PictureUrl, glib::Bytes>>>,
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

    async fn get_picture(url: &PictureUrl) -> Result<bytes::Bytes, Box<dyn std::error::Error>> {
        let client: reqwest::Client = reqwest::Client::builder().user_agent(USER_AGENT).build()?;
        Ok(client.get(&url.0).send().await?.bytes().await?)
    }

    fn handle_ui_daemon_tick(
        result: &Result<Option<PlayedTitle>, String>,
        main_window: &Rc<MainWindow>,
        images: &Arc<RwLock<HashMap<PictureUrl, glib::Bytes>>>,
        current_image_url: &Rc<RefCell<PictureUrl>>,
    ) {
        if let Err(error_string) = result {
            main_window.show_error(error_string);
        } else if let Ok(Some(result)) = result {
            let anime_info = &result.anime_info;
            let title = &anime_info.title;
            let episode_number = &anime_info.episode_watched.to_string();
            let player_name = &result.player_name;
            let status = if result.scrobbled {
                gettext("scrobbled")
            } else {
                gettext("not yet scrobbled")
            };

            let website_url = &anime_info.website_url;
            let picture = if *current_image_url.borrow() != anime_info.picture {
                current_image_url.replace(anime_info.picture.clone());
                Some(images.read().unwrap()[&anime_info.picture].clone())
            } else {
                None
            };

            main_window.set_anime_info(
                title,
                episode_number,
                player_name,
                &status,
                website_url,
                picture,
            );
        } else {
            main_window.set_anime_info_none();
            current_image_url.replace(PictureUrl::default());
        }
    }
}
