#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::egui;
use glance_lib::index::media::Media;
use glance_lib::index::{AddDirectoryConfig, Index};

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
}

impl GlanceEgui {
    fn new() -> Self {
        let mut index = Index::new_in_memory().expect("unable to initialize index");
        index
            .add_directory("../../../test-photos", &AddDirectoryConfig::default())
            .expect("to be able to add directory");
        let media_vec = index.get_media().expect("get media to work");
        let current_media_idx = if !media_vec.is_empty() { Some(0) } else { None };
        Self {
            media_vec,
            current_media_idx,
        }
    }
}

impl eframe::App for GlanceEgui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::both().show(ui, |ui| {
                if ui.button("Previous").clicked() {
                    self.current_media_idx =
                        self.current_media_idx
                            .map(|idx| if idx == 0 { 0 } else { idx - 1 });
                }

                if ui.button("Next").clicked() {
                    self.current_media_idx = self.current_media_idx.map(|idx| {
                        if idx == self.media_vec.len() - 1 {
                            idx
                        } else {
                            idx + 1
                        }
                    });
                }

                if let Some(idx) = self.current_media_idx {
                    let path = self.media_vec.get(idx).unwrap().filepath.to_str().unwrap();
                    ui.image(format!("file://{}", path));
                }
            });
        });
    }
}
