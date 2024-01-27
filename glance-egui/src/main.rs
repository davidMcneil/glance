#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use std::collections::VecDeque;

use chrono::Local;
use eframe::egui;
use glance_lib::index::media::Media;
use glance_lib::index::{AddDirectoryConfig, Index};
use sloggers::terminal::TerminalLoggerBuilder;
use sloggers::Build;

fn main() -> Result<(), eframe::Error> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([600.0, 800.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Image Viewer",
        options,
        Box::new(|cc| {
            // This gives us image support:
            egui_extras::install_image_loaders(&cc.egui_ctx);

            Box::new(GlanceEgui::new())
        }),
    )
}

#[derive(Default)]
struct GlanceEgui {
    media_vec: Vec<Media>,
    current_media_idx: Option<usize>,
    previously_seen_images: VecDeque<String>,
}

impl GlanceEgui {
    fn new() -> Self {
        let mut index = Index::new("testing.db")
            .expect("unable to initialize index")
            .with_logger(TerminalLoggerBuilder::new().build().unwrap());
        // index
        //     .add_directory(
        //         "/media/luke/TOSHIBA-SILVER/pictures/2013",
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
        Self {
            media_vec,
            current_media_idx,
            previously_seen_images: VecDeque::new(),
        }
    }

    fn add_previously_seen_image(&mut self, path: String, ctx: &egui::Context) {
        if self.previously_seen_images.contains(&path) {
            return;
        }

        self.previously_seen_images.push_front(path);
        if self.previously_seen_images.len() > 10 {
            let to_evict = self.previously_seen_images.pop_back().unwrap();
            ctx.forget_image(format!("file://{}", to_evict).as_str());
        }
    }
}

impl eframe::App for GlanceEgui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::both().show(ui, |ui| {
                if ui.button("Previous").clicked()
                    || ctx.input(|i| i.key_pressed(egui::Key::ArrowLeft))
                {
                    if let Some(idx) = self.current_media_idx {
                        if let Some(media) = self.media_vec.get(idx) {
                            self.add_previously_seen_image(
                                media.filepath.display().to_string(),
                                ctx,
                            );
                        }
                    }

                    self.current_media_idx =
                        self.current_media_idx
                            .map(|idx| if idx == 0 { 0 } else { idx - 1 });
                }

                if ui.button("Next").clicked()
                    || ctx.input(|i| i.key_pressed(egui::Key::ArrowRight))
                {
                    if let Some(idx) = self.current_media_idx {
                        if let Some(media) = self.media_vec.get(idx) {
                            self.add_previously_seen_image(
                                media.filepath.display().to_string(),
                                ctx,
                            );
                        }
                    }

                    self.current_media_idx = self.current_media_idx.map(|idx| {
                        if idx == self.media_vec.len() - 1 {
                            idx
                        } else {
                            idx + 1
                        }
                    });
                }

                if ui.button("Clear Cache").clicked() {
                    ctx.forget_all_images();
                }

                if let Some(idx) = self.current_media_idx {
                    let media = self.media_vec.get(idx).unwrap();
                    let path = media.filepath.display();

                    ui.label(format!("Path: {}", path));
                    if let Some(created_date) = media.created {
                        ui.label(format!("Taken: {}", created_date.with_timezone(&Local)));
                    }
                    if let Some(device) = &media.device {
                        ui.label(format!("Device: {}", device.0));
                    }
                    if let Some(location) = &media.location {
                        ui.label(format!("Location: {}", location));
                    }
                    ui.label(format!("Size: {}", media.size.0));

                    ui.image(format!("file://{}", path));

                    ui.horizontal(|ui| {
                        for i in 1..10 {
                            if let Some(media) = self.media_vec.get(i + idx) {
                                ui.image(format!("file://{}", media.filepath.display()));
                            }
                        }
                    });
                }
            });
        });
    }
}
