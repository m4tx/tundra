use gettextrs::gettext;
use gtk::{Orientation, ScrolledWindow, TextBuffer, TextView};
use libadwaita::prelude::*;
use libadwaita::{ApplicationWindow, HeaderBar, Window};

use crate::logging::get_logs;

pub struct LogsWindow {
    window: Window,
}

const DEFAULT_WIDTH: i32 = 600;
const DEFAULT_HEIGHT: i32 = 450;

impl LogsWindow {
    pub fn new(application: &gtk::Application, application_window: &ApplicationWindow) -> Self {
        let content = gtk::Box::new(Orientation::Vertical, 0);
        let text_view = Self::make_text_view();
        content.append(&text_view);

        let header_bar = HeaderBar::builder()
            .title_widget(&libadwaita::WindowTitle::new(&gettext("Tundra Logs"), ""))
            .build();

        let window_content = gtk::Box::new(Orientation::Vertical, 0);
        window_content.append(&header_bar);
        window_content.append(&content);

        let window = Window::builder()
            .application(application)
            .default_width(DEFAULT_WIDTH)
            .default_height(DEFAULT_HEIGHT)
            .content(&window_content)
            .modal(true)
            .transient_for(application_window)
            .build();

        Self { window }
    }

    fn make_text_view() -> ScrolledWindow {
        let logs_str = {
            let logs = get_logs().lock().expect("Could not lock logs store");
            logs.join("\n")
        };

        let text_view = TextView::new();

        let buffer = TextBuffer::builder().text(logs_str).build();

        text_view.set_buffer(Some(&buffer));
        text_view.set_editable(false);
        text_view.set_monospace(true);

        let scrolled_window = ScrolledWindow::new();
        scrolled_window.set_child(Some(&text_view));
        scrolled_window.set_hexpand(true);
        scrolled_window.set_vexpand(true);

        scrolled_window
    }

    pub fn show(&self) {
        self.window.show();
    }
}
