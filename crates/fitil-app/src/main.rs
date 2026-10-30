//! Fitil desktop application entry point.

// Release builds on Windows should not open a console window next to the app.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod i18n;

use fluent_bundle::FluentArgs;
use i18n::{Language, Translations};
use iced::widget::{button, column, container, row, text};
use iced::{Center, Element, Fill, Task, Theme};

fn main() -> iced::Result {
    iced::application(Fitil::new, Fitil::update, Fitil::view)
        .title(Fitil::title)
        .theme(Fitil::theme)
        .window_size((1280.0, 800.0))
        .centered()
        .run()
}

struct Fitil {
    translations: Translations,
}

#[derive(Debug, Clone, Copy)]
enum Message {
    LanguageSelected(Language),
}

impl Fitil {
    fn new() -> (Self, Task<Message>) {
        let translations = Translations::new(Language::from_system());
        (Self { translations }, Task::none())
    }

    fn title(&self) -> String {
        self.translations.get("app-title")
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::LanguageSelected(language) => {
                self.translations = Translations::new(language);
            }
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let t = &self.translations;

        let mut version_args = FluentArgs::new();
        version_args.set("version", env!("CARGO_PKG_VERSION"));

        let language_buttons = row(Language::ALL.map(|language| {
            let style = if language == t.language() {
                button::primary
            } else {
                button::secondary
            };
            button(text(t.get(language.name_key())))
                .style(style)
                .on_press(Message::LanguageSelected(language))
                .into()
        }))
        .spacing(8);

        let content = column![
            text(t.get("app-title")).size(64),
            text(t.get("app-tagline")).size(22),
            text(t.get("app-status")).size(16),
            text(t.get_with("app-version", &version_args)).size(14),
            language_buttons,
        ]
        .spacing(16)
        .align_x(Center);

        container(content).center(Fill).into()
    }

    fn theme(&self) -> Theme {
        Theme::Dark
    }
}
