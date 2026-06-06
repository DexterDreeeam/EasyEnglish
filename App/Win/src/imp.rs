//! Windows Search Overlay implementation.

use std::sync::Arc;
use std::path::PathBuf;
use eframe::egui;
use ee_core::{Hub, Storage, Record, RecordModel};
use ee_utils::Signal;

/// Run the Windows Search Overlay App.
pub fn run() -> Result<(), String> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_decorations(false) // Frameless
            .with_transparent(true)   // Transparent background
            .with_always_on_top()     // Floating on top of other windows
            .with_inner_size([700.0, 450.0]),
        ..Default::default()
    };

    eframe::run_native(
        "EasyEnglish Search Overlay",
        options,
        Box::new(|cc| Ok(Box::new(SearchOverlayApp::new(cc)))),
    )
    .map_err(|e| e.to_string())
}

struct SearchOverlayApp {
    input: String,
    hub: Hub,
    current_query: Option<ee_utils::DynamicResult<Vec<Record>>>,
    records: Vec<Record>,
    status: String,
    search_icon: Option<egui::TextureHandle>,
}

impl SearchOverlayApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Build the backend Hub with the three real databases
        let mut hub = Hub::new();

        let v1_path = get_db_path("word_en_v1.sqlite");
        let v2_path = get_db_path("word_en_v2.sqlite");
        let v3_path = get_db_path("word_en_v3.sqlite");

        if let Ok(s1) = Storage::new(&v1_path) {
            hub.add_provider(Arc::new(s1));
        }
        if let Ok(s2) = Storage::new(&v2_path) {
            hub.add_provider(Arc::new(s2));
        }
        if let Ok(s3) = Storage::new(&v3_path) {
            hub.add_provider(Arc::new(s3));
        }

        // Load search icon from App/Assets/search_icon.png
        let icon_path = get_asset_path("search_icon.png");
        let search_icon = if icon_path.exists() {
            if let Ok(color_img) = load_image_from_path(&icon_path) {
                Some(cc.egui_ctx.load_texture(
                    "search_icon",
                    color_img,
                    Default::default()
                ))
            } else {
                None
            }
        } else {
            None
        };

        Self {
            input: String::new(),
            hub,
            current_query: None,
            records: Vec::new(),
            status: "Type a word and press Enter or click the Search button.".to_string(),
            search_icon,
        }
    }

    fn trigger_search(&mut self) {
        let trimmed = self.input.trim();
        if trimmed.is_empty() {
            return;
        }

        self.status = "Searching...".to_string();
        self.records.clear();
        
        // Launch multi-source async streaming lookup
        let handle = self.hub.query(trimmed);
        self.current_query = Some(handle);
    }
}

impl eframe::App for SearchOverlayApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Non-blocking result polling: check if the async query has new updates
        if let Some(query_handle) = &self.current_query {
            // Wait with 0ms to check without blocking the main UI thread
            match query_handle.wait(Some(std::time::Duration::from_millis(0))) {
                Signal::Changed => {
                    self.records = query_handle.get();
                }
                Signal::Finished => {
                    self.records = query_handle.get();
                    if self.records.is_empty() {
                        self.status = format!("Not found: '{}'", self.input.trim());
                    } else {
                        self.status = format!("Found {} results.", self.records.len());
                    }
                    self.current_query = None;
                }
                Signal::Failed(err) => {
                    self.status = format!("Search failed: {}", err);
                    self.current_query = None;
                }
                Signal::TimedOut => {
                    // Check if we already received some records during this frame
                    self.records = query_handle.get();
                }
            }
            // Continuous redraw while query is active
            ctx.request_repaint();
        }

        // Translucent container for the rounded search bar (matching copilot-image-094749.png)
        let transparent_panel = egui::CentralPanel::default().frame(
            egui::Frame::none().fill(egui::Color32::TRANSPARENT)
        );

        transparent_panel.show(ctx, |ui| {
            ui.add_space(20.0);

            // Entire search bar fits in a single horizontal rounded block
            egui::Frame::none()
                .fill(egui::Color32::WHITE)
                .rounding(24.0)
                .inner_margin(0.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.add_space(14.0);

                        // Editable input box without standard borders
                        let edit_resp = ui.add(
                            egui::TextEdit::singleline(&mut self.input)
                                .hint_text("Search the web")
                                .frame(false)
                                .text_color(egui::Color32::BLACK)
                        );

                        if edit_resp.lost_focus() && ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
                            self.trigger_search();
                        }

                        // Grow text input area
                        let btn_room = 60.0;
                        let text_width = ui.available_width() - btn_room;
                        ui.set_min_width(text_width);

                        // Blue search button sharing the rounded right-side edge
                        let btn_color = egui::Color32::from_rgb(0, 120, 215);
                        let button_frame = egui::Frame::none()
                            .fill(btn_color)
                            .rounding(egui::Rounding {
                                nw: 0.0,
                                ne: 24.0,
                                sw: 0.0,
                                se: 24.0,
                            })
                            .inner_margin(egui::Margin::symmetric(20.0, 12.0));

                        button_frame.show(ui, |ui| {
                            let click_triggered = if let Some(texture) = &self.search_icon {
                                ui.add(
                                    egui::ImageButton::new(egui::Image::from_texture(texture))
                                        .tint(egui::Color32::WHITE)
                                        .frame(false)
                                ).clicked()
                            } else {
                                ui.button("🔍").clicked()
                            };

                            if click_triggered {
                                self.trigger_search();
                            }
                        });
                    });
                });

            // Status indication
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.add_space(12.0);
                ui.label(egui::RichText::new(&self.status).color(egui::Color32::from_gray(180)).size(12.0));
            });

            // Results Pane (shown below when we have active records)
            if !self.records.is_empty() {
                ui.add_space(12.0);
                egui::Frame::none()
                    .fill(egui::Color32::from_black_alpha(200)) // Translucent dark background for readability
                    .rounding(16.0)
                    .inner_margin(16.0)
                    .show(ui, |ui| {
                        egui::ScrollArea::vertical().max_height(280.0).show(ui, |ui| {
                            for (i, rec) in self.records.iter().enumerate() {
                                ui.group(|ui| {
                                    ui.set_width(ui.available_width());
                                    ui.horizontal(|ui| {
                                        ui.label(egui::RichText::new(format!("Source DB #{}", i + 1))
                                            .color(egui::Color32::from_gray(140))
                                            .size(11.0));
                                    });
                                    
                                    if let Ok(model) = rec.deserialize() {
                                        match model {
                                            RecordModel::WordEn(word) => {
                                                ui.horizontal(|ui| {
                                                    ui.heading(egui::RichText::new(&word.word).color(egui::Color32::WHITE));
                                                    if let Some(pron) = &word.pronunciation {
                                                        ui.label(egui::RichText::new(format!("US: {}", pron.ipa))
                                                            .color(egui::Color32::LIGHT_BLUE));
                                                    }
                                                });
                                                
                                                if let Some(definitions) = &word.definitions {
                                                    for def in definitions {
                                                        ui.label(egui::RichText::new(format!("{} {}", def.pos, def.meanings.join(", ")))
                                                            .color(egui::Color32::from_rgb(220, 220, 220)));
                                                    }
                                                }

                                                if let Some(inf) = &word.inflections {
                                                    let mut infs = Vec::new();
                                                    if let Some(p) = &inf.plural { infs.push(format!("pl. {}", p)); }
                                                    if let Some(pt) = &inf.past_tense { infs.push(format!("past {}", pt)); }
                                                    if let Some(pp) = &inf.past_participle { infs.push(format!("pp. {}", pp)); }
                                                    if let Some(prp) = &inf.present_participle { infs.push(format!("pres.p. {}", prp)); }
                                                    if let Some(ts) = &inf.third_singular { infs.push(format!("3sg. {}", ts)); }
                                                    if !infs.is_empty() {
                                                        ui.label(egui::RichText::new(format!("Inflections: {}", infs.join(", ")))
                                                            .color(egui::Color32::from_rgb(150, 220, 150)));
                                                    }
                                                }

                                                if let Some(examples) = &word.examples {
                                                    for ex in examples {
                                                        ui.label(egui::RichText::new(format!("• {}: {}", ex.en, ex.zh))
                                                            .color(egui::Color32::from_rgb(230, 220, 180)));
                                                    }
                                                }
                                            }
                                            _ => {
                                                ui.label(format!("Raw: {}", rec.value));
                                            }
                                        }
                                    } else {
                                        ui.label(format!("Raw Payload: {}", rec.value));
                                    }
                                });
                                ui.add_space(8.0);
                            }
                        });
                    });
            }
        });
    }
}

fn get_db_path(filename: &str) -> PathBuf {
    let path = std::env::current_dir().unwrap_or_default().join("Dict").join(filename);
    if path.exists() {
        return path;
    }
    if let Ok(exe_path) = std::env::current_exe() {
        let mut p = exe_path;
        for _ in 0..5 {
            if let Some(parent) = p.parent() {
                p = parent.to_path_buf();
                let possible = p.join("Dict").join(filename);
                if possible.exists() {
                    return possible;
                }
            }
        }
    }
    PathBuf::from("Dict").join(filename)
}

fn get_asset_path(filename: &str) -> PathBuf {
    let path = std::env::current_dir().unwrap_or_default().join("App").join("Assets").join(filename);
    if path.exists() {
        return path;
    }
    if let Ok(exe_path) = std::env::current_exe() {
        let mut p = exe_path;
        for _ in 0..5 {
            if let Some(parent) = p.parent() {
                p = parent.to_path_buf();
                let possible = p.join("App").join("Assets").join(filename);
                if possible.exists() {
                    return possible;
                }
            }
        }
    }
    PathBuf::from("App").join("Assets").join(filename)
}

fn load_image_from_path(path: &std::path::Path) -> Result<egui::ColorImage, Box<dyn std::error::Error>> {
    let image = image::io::Reader::open(path)?.decode()?;
    let size = [image.width() as _, image.height() as _];
    let image_buffer = image.to_rgba8();
    let pixels = image_buffer.as_flat_samples();
    Ok(egui::ColorImage::from_rgba_unmultiplied(
        size,
        pixels.as_slice(),
    ))
}
