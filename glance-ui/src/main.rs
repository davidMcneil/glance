use anyhow::Result;
use chrono::Local;
use glance_lib::index::media::Media;
use glance_lib::index::Index;
use iced::widget::{button, column, container, image, row, text};
use iced::{executor, subscription, Application, Command, Element, Event, Settings, Theme};
use iced::{keyboard, Subscription};
use sloggers::terminal::TerminalLoggerBuilder;
use sloggers::Build;

pub fn main() -> Result<()> {
    GlanceUi::run(Settings::default())?;
    Ok(())
}

#[derive(Default)]
struct GlanceUi {
    media_vec: Vec<Media>,
    current_media_idx: Option<usize>,
}

#[derive(Debug, Clone, Copy)]
enum Message {
    NextImage,
    PreviousImage,
}

impl Application for GlanceUi {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        let index = Index::new("test.db")
            .expect("unable to initialize index")
            .with_logger(TerminalLoggerBuilder::new().build().unwrap());
        // index
        //     .add_directory(
        //         "/media/luke/TOSHIBA-SILVER/pictures/2012",
        //         // "../../../test-photos",
        //         &AddDirectoryConfig {
        //             hash: false,
        //             filter_by_media: true,
        //             use_modified_if_created_not_set: true,
        //             calculate_nearest_city: false,
        //         },
        //     )
        //     .expect("to be able to add directory");
        let media_vec = index.get_media().expect("get media to work");
        let current_media_idx = if !media_vec.is_empty() { Some(0) } else { None };
        (
            Self {
                media_vec,
                current_media_idx,
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("Glance")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::NextImage => {
                self.current_media_idx = self.current_media_idx.map(|idx| {
                    if idx == self.media_vec.len() - 1 {
                        idx
                    } else {
                        idx + 1
                    }
                });
            }
            Message::PreviousImage => {
                self.current_media_idx =
                    self.current_media_idx
                        .map(|idx| if idx == 0 { 0 } else { idx - 1 });
            }
        };
        Command::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        subscription::events_with(|event, _status| match event {
            Event::Keyboard(keyboard_event) => match keyboard_event {
                keyboard::Event::KeyPressed {
                    key_code: keyboard::KeyCode::Right,
                    modifiers: _,
                } => Some(Message::NextImage),
                keyboard::Event::KeyPressed {
                    key_code: keyboard::KeyCode::Left,
                    modifiers: _,
                } => Some(Message::PreviousImage),
                _ => None,
            },
            _ => None,
        })
    }

    fn view(&self) -> Element<Message> {
        let buttons = row![
            button("Previous")
                .padding([10, 20])
                .on_press(Message::PreviousImage),
            button("Next")
                .padding([10, 20])
                .on_press(Message::NextImage)
        ]
        .spacing(10);

        let mut contents = column![buttons];
        if let Some(idx) = self.current_media_idx {
            let media = self.media_vec.get(idx).unwrap();
            contents = contents.push(text(format!("path: {}", media.filepath.display())));
            if let Some(created) = &media.created {
                contents =
                    contents.push(text(format!("Created: {}", created.with_timezone(&Local))));
            }
            if let Some(device) = &media.device {
                contents = contents.push(text(format!("Device: {}", device.0)));
            }
            if let Some(location) = &media.location {
                contents = contents.push(text(format!("Location: {}", location)));
            }
            contents = contents.push(text(format!("Size: {}", media.size.0)));

            let image = image(media.filepath.clone());
            contents = contents.push(image);
        }

        container(contents).padding(20).into()
    }
}
