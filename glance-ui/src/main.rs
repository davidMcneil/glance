use std::fs;

use anyhow::Result;
use glance_lib::index::media::Media;
use glance_lib::index::{AddDirectoryConfig, Index};
use iced::executor;
use iced::widget::image::Handle;
use iced::widget::{button, column, container, image, row};
use iced::{Application, Command, Element, Settings, Theme};

pub fn main() -> Result<()> {
    GlanceUi::run(Settings::default())?;
    Ok(())
}

#[derive(Default)]
struct GlanceUi {
    media_vec: Vec<Media>,
    image_handles: Vec<Handle>,
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
        let mut index = Index::new_in_memory().expect("unable to initialize index");
        index
            .add_directory("../test-media", &AddDirectoryConfig::default())
            .expect("to be able to add directory");
        let media_vec = index.get_media().expect("get media to work");
        let image_handles = media_vec
            .iter()
            .map(|media| {
                let bytes = fs::read(&media.filepath).unwrap();
                Handle::from_memory(bytes)
            })
            .collect();
        let current_media_idx = if !media_vec.is_empty() { Some(0) } else { None };
        (
            Self {
                media_vec,
                image_handles,
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
            let handle = self.image_handles.get(idx).unwrap();
            let image = image(handle.clone());
            contents = contents.push(image);
        }

        container(contents).padding(20).into()
    }
}
