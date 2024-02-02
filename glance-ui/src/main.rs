#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use std::collections::VecDeque;

use chrono::{Local, NaiveDate, Utc};
use eframe::egui;
use glance_lib::index::media::{Media, MediaFilter};
use glance_lib::index::{AddDirectoryConfig, Index};
use slog::{warn, Logger};
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
            Box::new(GlanceUi::new())
        }),
    )
}

struct GlanceUi {
    index: Option<Index>,
    media_vec: Vec<Media>,
    current_media_idx: Option<usize>,
    previously_seen_images: VecDeque<String>,
    start_date: NaiveDate,
    end_date: NaiveDate,
    stats: Option<String>,
    picked_path: Option<String>,
    add_directory_config: AddDirectoryConfig,
    label_to_add: String,
    label_to_filter: Option<String>,
    all_labels: Vec<String>,
    logger: Logger,
}

impl GlanceUi {
    fn new() -> Self {
        Self {
            start_date: NaiveDate::from_ymd_opt(2008, 1, 1).unwrap(),
            end_date: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            index: Default::default(),
            media_vec: Default::default(),
            current_media_idx: Default::default(),
            previously_seen_images: Default::default(),
            stats: Default::default(),
            picked_path: Default::default(),
            add_directory_config: Default::default(),
            label_to_add: Default::default(),
            label_to_filter: Default::default(),
            all_labels: Default::default(),
            logger: TerminalLoggerBuilder::new().build().unwrap(),
        }
    }

    fn change_index(&mut self) {
        if let Some(path) = &self.picked_path {
            self.index = Some(
                Index::new(format!("{}/glance.db", path))
                    .expect("to be able to initialize index")
                    .with_logger(self.logger.clone()),
            );
        }
        self.update_media();
    }

    fn add_directory(&mut self) {
        if let Some(index) = &mut self.index {
            if let Some(path) = &self.picked_path {
                index
                    .add_directory(path, &self.add_directory_config)
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
            label: self.label_to_filter.clone(),
        };
        self.update_labels();
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

    fn update_labels(&mut self) {
        if let Some(index) = &self.index {
            if let Ok(all_labels) = index.get_all_labels() {
                self.all_labels = all_labels;
            }
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

impl eframe::App for GlanceUi {
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

                ui.horizontal(|ui| {
                    if ui.button("Index Chosen Folder").clicked() {
                        self.add_directory();
                    }
                    ui.checkbox(&mut self.add_directory_config.hash, "hash");
                    ui.checkbox(
                        &mut self.add_directory_config.filter_by_media,
                        "filter by media",
                    );
                    ui.checkbox(
                        &mut self.add_directory_config.use_modified_if_created_not_set,
                        "use modified if created not set",
                    );
                    ui.checkbox(
                        &mut self.add_directory_config.calculate_nearest_city,
                        "calculate nearest city",
                    );
                });

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

                ui.horizontal(|ui| {
                    egui::ComboBox::from_label("Filter by label")
                        .selected_text(format!(
                            "{}",
                            match &self.label_to_filter {
                                Some(label) => label,
                                None => "all",
                            }
                        ))
                        .show_ui(ui, |ui| {
                            if ui
                                .selectable_value(&mut self.label_to_filter, None, "all")
                                .changed()
                            {
                                self.update_media();
                            }
                            for label in self.all_labels.clone() {
                                if ui
                                    .selectable_value(
                                        &mut self.label_to_filter,
                                        Some(label.clone()),
                                        label,
                                    )
                                    .changed()
                                {
                                    self.update_media();
                                }
                            }
                        });
                });

                if let Some(idx) = self.current_media_idx {
                    let media = self.media_vec.get(idx).unwrap();
                    let path = media.filepath.clone();
                    ui.label(format!("Path: {}", path.display()));
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
                    if let Some(hash) = &media.hash {
                        ui.label(format!("Hash: {}", hash.to_string()));
                    }

                    if let Some(index) = &mut self.index {
                        ui.horizontal(|ui| {
                            ui.text_edit_singleline(&mut self.label_to_add);
                            if ui.button("Add Label").clicked() {
                                if let Err(e) = index.add_label(&path, self.label_to_add.clone()) {
                                    warn!(self.logger, "failed to add label";
                                        "error" => e.to_string(),
                                    );
                                }
                                if let Ok(all_labels) = index.get_all_labels() {
                                    self.all_labels = all_labels;
                                }
                            }
                            if ui.button("Remove Label").clicked() {
                                if let Err(e) = index.delete_label(&path, self.label_to_add.clone())
                                {
                                    warn!(self.logger, "failed to delete label";
                                        "error" => e.to_string(),
                                    );
                                }
                                if let Ok(all_labels) = index.get_all_labels() {
                                    self.all_labels = all_labels;
                                }
                            }
                        });

                        if let Ok(labels) = index.get_labels(&path) {
                            ui.label(format!("Labels: {}", labels.join(",")));
                        }
                    }

                    ui.image(format!("file://{}", path.display()));

                    ui.horizontal(|ui| {
                        for i in 1..10 {
                            if let Some(next_media) = self.media_vec.get(i + idx) {
                                ui.image(format!("file://{}", next_media.filepath.display()));
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
