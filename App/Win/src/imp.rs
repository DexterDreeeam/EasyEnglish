//! Windows Search Overlay implementation (Pure Compact Black Rectangular Box).

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
            .with_inner_size([460.0, 90.0]), // Initial small size
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

        // Configure Microsoft YaHei to support Chinese characters beautifully
        let mut fonts = egui::FontDefinitions::default();
        let font_path = "C:\\Windows\\Fonts\\msyh.ttc"; // Microsoft YaHei
        if std::path::Path::new(font_path).exists() {
            if let Ok(font_data) = std::fs::read(font_path) {
                fonts.font_data.insert(
                    "msyh".to_owned(),
                    egui::FontData::from_owned(font_data),
                );
                fonts.families.get_mut(&egui::FontFamily::Proportional).unwrap().insert(0, "msyh".to_owned());
                fonts.families.get_mut(&egui::FontFamily::Monospace).unwrap().push("msyh".to_owned());
            }
        }
        cc.egui_ctx.set_fonts(fonts);

        Self {
            input: String::new(),
            hub,
            current_query: None,
            records: Vec::new(),
            status: "Type a word and press Enter...".to_string(),
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

        // Dynamic Height Calculation: automatically resize the OS window height based on displayed content
        let mut desired_height = 80.0; // Base: input box + status label + paddings
        if !self.records.is_empty() {
            let mut results_height = 16.0; // padding
            for rec in &self.records {
                results_height += 36.0; // Group header & gap
                if let Ok(model) = rec.deserialize() {
                    match model {
                        RecordModel::WordEn(word) => {
                            results_height += 28.0; // Word title + IPA
                            if let Some(definitions) = &word.definitions {
                                results_height += (definitions.len() * 22) as f32;
                            }
                            if let Some(inf) = &word.inflections {
                                results_height += 22.0;
                            }
                            if let Some(examples) = &word.examples {
                                results_height += (examples.len() * 22) as f32;
                            }
                        }
                        _ => {
                            results_height += 30.0;
                        }
                    }
                }
            }
            // Cap scrollable area height to max 300px
            let results_height = results_height.min(300.0);
            desired_height += results_height + 20.0; // Spacing & results panel margin
        }

        // Apply window resize command dynamically
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(460.0, desired_height)));

        // Translucent container with NO window background
        let transparent_panel = egui::CentralPanel::default().frame(
            egui::Frame::none().fill(egui::Color32::TRANSPARENT)
        );

        transparent_panel.show(ctx, |ui| {
            ui.add_space(10.0);

            // Clean black rectangular search box with thin border (matching user's request)
            egui::Frame::none()
                .fill(egui::Color32::from_rgb(15, 15, 15)) // Pure deep black
                .stroke(egui::Stroke::new(1.5, egui::Color32::from_gray(120))) // Precise gray border
                .rounding(4.0) // Compact rectangle
                .inner_margin(egui::Margin::symmetric(14.0, 10.0))
                .show(ui, |ui| {
                    let edit_resp = ui.add(
                        egui::TextEdit::singleline(&mut self.input)
                            .hint_text("Enter word...")
                            .frame(false)
                            .text_color(egui::Color32::WHITE)
                    );

                    // Re-acquire focus dynamically
                    edit_resp.request_focus();

                    if edit_resp.lost_focus() && ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
                        self.trigger_search();
                    }
                });

            // Status message
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.add_space(10.0);
                ui.label(egui::RichText::new(&self.status).color(egui::Color32::from_gray(180)).size(11.0));
            });

            // Results Pane (shown below when we have active records)
            if !self.records.is_empty() {
                ui.add_space(10.0);
                egui::Frame::none()
                    .fill(egui::Color32::from_black_alpha(220)) // Dark semi-transparent container
                    .rounding(8.0)
                    .inner_margin(14.0)
                    .show(ui, |ui| {
                        egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                            for (i, rec) in self.records.iter().enumerate() {
                                ui.group(|ui| {
                                    ui.set_width(ui.available_width());
                                    ui.horizontal(|ui| {
                                        ui.label(egui::RichText::new(format!("Source DB #{}", i + 1))
                                            .color(egui::Color32::from_gray(130))
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
                                                            .color(egui::Color32::from_rgb(225, 225, 225)));
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
                                                            .color(egui::Color32::from_rgb(140, 215, 140)));
                                                    }
                                                }

                                                if let Some(examples) = &word.examples {
                                                    for ex in examples {
                                                        ui.label(egui::RichText::new(format!("• {}: {}", ex.en, ex.zh))
                                                            .color(egui::Color32::from_rgb(225, 215, 175)));
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
                                ui.add_space(6.0);
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
