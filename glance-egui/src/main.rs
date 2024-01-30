#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use std::collections::VecDeque;

use chrono::{Local, NaiveDate, Utc};
use eframe::egui;
use glance_lib::index::media::{Media, MediaFilter};
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
        "Glance",
        options,
        Box::new(|cc| {
            // This gives us image support:
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Box::new(GlanceEgui::new())
        }),
    )
}

struct GlanceEgui {
    index: Option<Index>,
    media_vec: Vec<Media>,
    current_media_idx: Option<usize>,
    previously_seen_images: VecDeque<String>,
    start_date: NaiveDate,
    end_date: NaiveDate,
    stats: Option<String>,
    picked_path: Option<String>,
}

impl GlanceEgui {
    fn new() -> Self {
        Self {
            index: None,
            media_vec: Vec::new(),
            current_media_idx: None,
            previously_seen_images: VecDeque::new(),
            start_date: NaiveDate::from_ymd_opt(2008, 1, 1).unwrap(),
            end_date: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            stats: None,
            picked_path: None,
        }
    }

    fn change_index(&mut self) {
        if let Some(path) = &self.picked_path {
            self.index = Some(
                Index::new(format!("{}/glance.db", path))
                    .expect("to be able to initialize index")
                    .with_logger(TerminalLoggerBuilder::new().build().unwrap()),
            );
        }
        self.update_media();
    }

    fn add_directory(&mut self) {
        if let Some(index) = &mut self.index {
            if let Some(path) = &self.picked_path {
                index
                    .add_directory(
                        path,
                        &AddDirectoryConfig {
                            hash: false,
                            filter_by_media: true,
                            use_modified_if_created_not_set: true,
                            calculate_nearest_city: false,
                        },
                    )
                    .expect("to be able to add directory");
            }
        }
        self.update_media();
    }

    fn update_media(&mut self) {
        let media_filter = MediaFilter {
            created_start: Some(chrono::DateTime::from_naive_utc_and_offset(
                self.start_date.and_hms_opt(0, 0, 0).unwrap(),
                Utc,
            )),
            created_end: Some(chrono::DateTime::from_naive_utc_and_offset(
                self.end_date.and_hms_opt(0, 0, 0).unwrap(),
                Utc,
            )),
        };
        if let Some(index) = &self.index {
            self.media_vec = index
                .get_media_with_filter(media_filter)
                .expect("get media to work");
            self.current_media_idx = if !self.media_vec.is_empty() {
                Some(0)
            } else {
                None
            };
            self.stats = index
                .stats()
                .ok()
                .map(|s| serde_json::to_string_pretty(&s).unwrap());
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
                ui.horizontal(|ui| {
                    if ui.button("Open folder…").clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_folder() {
                            self.picked_path = Some(path.display().to_string());
                            ctx.forget_all_images();
                            self.change_index()
                        }
                    }
                    if let Some(path) = &self.picked_path {
                        ui.label(path);
                    }
                });

                if ui.button("Index Chosen Folder").clicked() {
                    self.add_directory();
                }

                ui.horizontal(|ui| {
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
                });

                if ui.button("Clear Cache").clicked() {
                    ctx.forget_all_images();
                }

                ui.horizontal(|ui| {
                    ui.label("Start date");
                    if ui
                        .add(
                            egui_extras::DatePickerButton::new(&mut self.start_date)
                                .id_source("start"),
                        )
                        .changed()
                    {
                        self.update_media();
                    };

                    ui.label("End date");
                    if ui
                        .add(
                            egui_extras::DatePickerButton::new(&mut self.end_date).id_source("end"),
                        )
                        .changed()
                    {
                        self.update_media();
                    };
                });

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

                if let Some(stats) = &self.stats {
                    ui.label(stats);
                }
            });
        });
    }
}
