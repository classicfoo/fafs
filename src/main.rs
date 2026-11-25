#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod actions;
mod config;
mod search;

use config::{AppConfig, SearchType};
use eframe::egui;
use egui::{Color32, Key, RichText};
use egui_extras::{Column, TableBuilder};
use search::SearchResult;
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
        mpsc::{self, Receiver, Sender},
    },
    time::{Duration, Instant},
};

const CANVAS: Color32 = Color32::from_rgb(246, 248, 250);
const SURFACE: Color32 = Color32::WHITE;
const BORDER: Color32 = Color32::from_rgb(209, 217, 224);
const FOREGROUND: Color32 = Color32::from_rgb(31, 35, 40);
const MUTED: Color32 = Color32::from_rgb(89, 99, 110);
const BLUE: Color32 = Color32::from_rgb(9, 105, 218);
const BLUE_TINT: Color32 = Color32::from_rgb(221, 244, 255);
const GREEN: Color32 = Color32::from_rgb(31, 136, 61);
const RED: Color32 = Color32::from_rgb(207, 34, 46);

struct SearchMessage {
    generation: u64,
    result: Result<Vec<SearchResult>, String>,
}

#[derive(Clone, Copy)]
enum FileActionKind {
    Open,
    CopyPath,
    CopyFilename,
    CopyFile,
    Reveal,
    Touch,
    OpenWithEditor,
    NormalizeName,
    Rename,
    Delete,
    Archive,
}

struct FileAction {
    kind: FileActionKind,
    path: PathBuf,
}

struct SearchApp {
    config: AppConfig,
    settings_draft: Option<AppConfig>,
    query: String,
    last_submitted_query: String,
    query_deadline: Option<Instant>,
    results: Vec<SearchResult>,
    selected: Option<PathBuf>,
    status: String,
    feedback: Option<(String, Instant)>,
    error: Option<String>,
    searching: bool,
    search_tx: Sender<SearchMessage>,
    search_rx: Receiver<SearchMessage>,
    search_generation: Arc<AtomicU64>,
    pending_action: Option<FileAction>,
    settings_open: bool,
    rename_target: Option<PathBuf>,
    rename_value: String,
    confirm_delete: Option<PathBuf>,
    search_focus_requested: bool,
    result_focus_ids: Vec<egui::Id>,
    focus_first_result_when_ready: bool,
}

fn primary_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(label).color(Color32::WHITE))
            .fill(GREEN)
            .stroke(egui::Stroke::new(1.0, Color32::from_rgb(31, 125, 56)))
            .corner_radius(egui::CornerRadius::same(6))
            .min_size(egui::vec2(84.0, 36.0)),
    )
}

impl SearchApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut visuals = egui::Visuals::light();
        visuals.panel_fill = CANVAS;
        visuals.window_fill = SURFACE;
        visuals.window_stroke = egui::Stroke::new(1.0, BORDER);
        visuals.extreme_bg_color = SURFACE;
        visuals.text_edit_bg_color = Some(SURFACE);
        visuals.faint_bg_color = CANVAS;
        visuals.code_bg_color = CANVAS;
        visuals.hyperlink_color = BLUE;
        visuals.weak_text_color = Some(MUTED);
        visuals.error_fg_color = RED;
        visuals.selection.bg_fill = BLUE_TINT;
        visuals.selection.stroke = egui::Stroke::new(1.0, BLUE);

        visuals.widgets.noninteractive.bg_fill = SURFACE;
        visuals.widgets.noninteractive.weak_bg_fill = CANVAS;
        visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, BORDER);
        visuals.widgets.noninteractive.fg_stroke.color = FOREGROUND;
        visuals.widgets.inactive.bg_fill = SURFACE;
        visuals.widgets.inactive.weak_bg_fill = CANVAS;
        visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, BORDER);
        visuals.widgets.inactive.fg_stroke.color = FOREGROUND;
        visuals.widgets.hovered.bg_fill = Color32::from_rgb(243, 244, 246);
        visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(243, 244, 246);
        visuals.widgets.hovered.bg_stroke =
            egui::Stroke::new(1.0, Color32::from_rgb(175, 184, 193));
        visuals.widgets.hovered.fg_stroke.color = FOREGROUND;
        visuals.widgets.active.bg_fill = BLUE_TINT;
        visuals.widgets.active.weak_bg_fill = BLUE_TINT;
        visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, BLUE);
        visuals.widgets.active.fg_stroke.color = FOREGROUND;
        visuals.widgets.open.bg_fill = Color32::from_rgb(243, 244, 246);
        visuals.widgets.open.weak_bg_fill = Color32::from_rgb(243, 244, 246);
        visuals.widgets.open.bg_stroke = egui::Stroke::new(1.0, BORDER);
        visuals.widgets.open.fg_stroke.color = FOREGROUND;
        for widget in [
            &mut visuals.widgets.noninteractive,
            &mut visuals.widgets.inactive,
            &mut visuals.widgets.hovered,
            &mut visuals.widgets.active,
            &mut visuals.widgets.open,
        ] {
            widget.corner_radius = egui::CornerRadius::same(6);
        }

        let mut style = (*cc.egui_ctx.style_of(egui::Theme::Light)).clone();
        style.visuals = visuals;
        style.spacing.item_spacing = egui::vec2(10.0, 10.0);
        style.spacing.button_padding = egui::vec2(12.0, 7.0);
        style.spacing.interact_size = egui::vec2(40.0, 34.0);
        cc.egui_ctx.set_style_of(egui::Theme::Light, style);
        cc.egui_ctx.set_theme(egui::Theme::Light);

        let (search_tx, search_rx) = mpsc::channel();
        let (config, warning) = AppConfig::load();
        let mut app = Self {
            config,
            settings_draft: None,
            query: String::new(),
            last_submitted_query: String::new(),
            query_deadline: None,
            results: Vec::new(),
            selected: None,
            status: "Ready".to_owned(),
            feedback: None,
            error: warning,
            searching: false,
            search_tx,
            search_rx,
            search_generation: Arc::new(AtomicU64::new(0)),
            pending_action: None,
            settings_open: false,
            rename_target: None,
            rename_value: String::new(),
            confirm_delete: None,
            search_focus_requested: false,
            result_focus_ids: Vec::new(),
            focus_first_result_when_ready: false,
        };
        app.start_search();
        app
    }

    fn start_search(&mut self) {
        let generation = search::next_generation(&self.search_generation);
        let query = self.query.clone();
        let root = self.config.search_path();
        let search_type = self.config.search_type;
        let tx = self.search_tx.clone();
        let latest = Arc::clone(&self.search_generation);

        self.last_submitted_query = query.clone();
        self.query_deadline = None;
        self.searching = true;
        self.status = format!("Searching {}…", root.display());

        std::thread::spawn(move || {
            let result = search::scan(&root, &query, search_type, generation, &latest);
            let _ = tx.send(SearchMessage { generation, result });
        });
    }

    fn poll_search(&mut self) {
        while let Ok(message) = self.search_rx.try_recv() {
            if message.generation != self.search_generation.load(Ordering::Relaxed) {
                continue;
            }
            self.searching = false;
            match message.result {
                Ok(results) => {
                    let count = results.len();
                    self.results = results;
                    if self.results.is_empty() {
                        self.focus_first_result_when_ready = false;
                    }
                    if self
                        .selected
                        .as_ref()
                        .is_some_and(|path| !self.results.iter().any(|item| &item.path == path))
                    {
                        self.selected = None;
                    }
                    self.status = format!("{count} result{}", if count == 1 { "" } else { "s" });
                    self.error = None;
                }
                Err(error) => {
                    self.results.clear();
                    self.selected = None;
                    self.status = "Search could not run".to_owned();
                    self.error = Some(error);
                }
            }
        }
    }

    fn selected_result(&self) -> Option<SearchResult> {
        let selected = self.selected.as_ref()?;
        self.results
            .iter()
            .find(|result| &result.path == selected)
            .cloned()
    }

    fn queue_action(&mut self, kind: FileActionKind, path: PathBuf) {
        self.selected = Some(path.clone());
        self.pending_action = Some(FileAction { kind, path });
    }

    fn set_feedback(&mut self, message: &str) {
        self.feedback = Some((message.to_owned(), Instant::now() + Duration::from_secs(3)));
    }

    fn run_pending_action(&mut self) {
        let Some(action) = self.pending_action.take() else {
            return;
        };
        let path = action.path;

        let result = match action.kind {
            FileActionKind::Open => actions::open_path(&path).map(|_| "Opened item"),
            FileActionKind::CopyPath => {
                actions::copy_text(path.display().to_string()).map(|_| "Path copied")
            }
            FileActionKind::CopyFilename => {
                actions::copy_text(format!("\"{}\"", actions::filename(&path)))
                    .map(|_| "Filename copied")
            }
            FileActionKind::CopyFile => actions::copy_file(&path).map(|_| "Item copied"),
            FileActionKind::Reveal => {
                actions::reveal_in_file_manager(&path).map(|_| "Opened in file manager")
            }
            FileActionKind::Touch => actions::touch(&path).map(|_| "Updated modification time"),
            FileActionKind::OpenWithEditor => {
                actions::open_with_editor(&path, &self.config.editor())
                    .map(|_| "Opened with editor")
            }
            FileActionKind::NormalizeName => match actions::spaces_to_underscores(&path) {
                Ok(new_path) => {
                    self.selected = Some(new_path);
                    self.start_search();
                    Ok("Renamed item")
                }
                Err(error) => Err(error),
            },
            FileActionKind::Rename => {
                self.rename_value = actions::filename(&path);
                self.rename_target = Some(path);
                return;
            }
            FileActionKind::Delete => {
                self.confirm_delete = Some(path);
                return;
            }
            FileActionKind::Archive => {
                match actions::move_to_archive(&path, &self.config.archive_path()) {
                    Ok(_) => {
                        self.selected = None;
                        self.start_search();
                        Ok("Moved to archive")
                    }
                    Err(error) => Err(error),
                }
            }
        };

        match result {
            Ok(message) => {
                self.error = None;
                if matches!(action.kind, FileActionKind::Touch) {
                    self.start_search();
                }
                self.set_feedback(message);
            }
            Err(error) => self.error = Some(error),
        }
    }

    fn show_context_menu(
        ui: &mut egui::Ui,
        item: &SearchResult,
        pending_action: &mut Option<FileAction>,
    ) {
        let mut queue = |ui: &mut egui::Ui, label: &str, kind| {
            if ui.button(label).clicked() {
                *pending_action = Some(FileAction {
                    kind,
                    path: item.path.clone(),
                });
                ui.close();
            }
        };

        queue(ui, "Open", FileActionKind::Open);
        ui.separator();
        queue(ui, "Copy Path", FileActionKind::CopyPath);
        queue(ui, "Copy Filename", FileActionKind::CopyFilename);
        if cfg!(windows) {
            queue(ui, "Copy File", FileActionKind::CopyFile);
        }
        queue(ui, "Open in File Manager", FileActionKind::Reveal);
        ui.separator();
        queue(ui, "Touch", FileActionKind::Touch);
        if !item.is_folder {
            queue(ui, "Open with Editor", FileActionKind::OpenWithEditor);
        }
        queue(ui, "Spaces to Underscores", FileActionKind::NormalizeName);
        queue(ui, "Rename…", FileActionKind::Rename);
        queue(ui, "Move to Recycle Bin…", FileActionKind::Delete);
        queue(ui, "Move to Archive", FileActionKind::Archive);
    }

    fn show_settings(&mut self, ctx: &egui::Context) {
        if !self.settings_open {
            return;
        }
        let mut open = self.settings_open;
        let mut save = false;
        let mut cancel = false;
        egui::Window::new("Settings")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                let Some(draft) = self.settings_draft.as_mut() else {
                    return;
                };

                ui.label("Search Directory");
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut draft.search_directory)
                            .desired_width(360.0),
                    );
                    if ui.button("Browse…").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .set_directory(draft.search_path())
                            .pick_folder()
                        {
                            draft.search_directory = path.to_string_lossy().into_owned();
                        }
                    }
                });

                ui.add_space(8.0);
                ui.label("Archive Directory");
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut draft.archive_directory)
                            .desired_width(360.0),
                    );
                    if ui.button("Browse…").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .set_directory(draft.archive_path())
                            .pick_folder()
                        {
                            draft.archive_directory = path.to_string_lossy().into_owned();
                        }
                    }
                });

                ui.add_space(8.0);
                ui.label("Editor application or script");
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut draft.editor_path).desired_width(360.0));
                    if ui.button("Browse…").clicked() {
                        let mut dialog = rfd::FileDialog::new();
                        if !draft.editor().as_os_str().is_empty() {
                            if let Some(name) =
                                draft.editor().file_name().and_then(|name| name.to_str())
                            {
                                dialog = dialog.set_file_name(name);
                            }
                        }
                        if let Some(path) = dialog.pick_file() {
                            draft.editor_path = path.to_string_lossy().into_owned();
                        }
                    }
                });

                ui.add_space(8.0);
                ui.label("Search Results");
                ui.horizontal(|ui| {
                    ui.radio_value(&mut draft.search_type, SearchType::All, "Files and folders");
                    ui.radio_value(&mut draft.search_type, SearchType::Folders, "Folders only");
                });

                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    if ui.button("Save Settings").clicked() {
                        save = true;
                    }
                    if ui.button("Cancel").clicked() {
                        cancel = true;
                    }
                });
            });
        self.settings_open = open && !cancel;

        if cancel || !open {
            self.settings_draft = None;
        }
        if save {
            if let Some(draft) = self.settings_draft.clone() {
                if !draft.search_path().is_dir() {
                    self.error = Some(format!(
                        "Search directory does not exist: {}",
                        draft.search_path().display()
                    ));
                    self.settings_open = true;
                } else if let Err(error) = draft.save() {
                    self.error = Some(format!("Could not save settings: {error}"));
                    self.settings_open = true;
                } else {
                    self.config = draft;
                    self.settings_draft = None;
                    self.settings_open = false;
                    self.error = None;
                    self.start_search();
                }
            }
        }
    }

    fn show_rename_dialog(&mut self, ctx: &egui::Context) {
        let Some(target) = self.rename_target.clone() else {
            return;
        };
        let mut open = true;
        let mut save = false;
        let mut cancel = false;
        egui::Window::new("Rename")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label("Enter a new file or folder name:");
                let response = ui.text_edit_singleline(&mut self.rename_value);
                if response.lost_focus() && ui.input(|input| input.key_pressed(Key::Enter)) {
                    save = true;
                }
                ui.horizontal(|ui| {
                    if ui.button("Rename").clicked() {
                        save = true;
                    }
                    if ui.button("Cancel").clicked() {
                        cancel = true;
                    }
                });
            });

        if cancel || !open {
            self.rename_target = None;
        }
        if save {
            match actions::rename(&target, &self.rename_value) {
                Ok(new_path) => {
                    self.selected = Some(new_path);
                    self.rename_target = None;
                    self.start_search();
                    self.error = None;
                    self.set_feedback("Renamed item");
                }
                Err(error) => self.error = Some(error),
            }
        }
    }

    fn show_delete_confirmation(&mut self, ctx: &egui::Context) {
        let Some(path) = self.confirm_delete.clone() else {
            return;
        };
        let mut open = true;
        let mut confirm = false;
        let mut cancel = false;
        egui::Window::new("Move to Recycle Bin")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label(format!(
                    "Move this item to the recycle bin?\n{}",
                    path.display()
                ));
                ui.horizontal(|ui| {
                    if ui.button("Move to Recycle Bin").clicked() {
                        confirm = true;
                    }
                    if ui.button("Cancel").clicked() {
                        cancel = true;
                    }
                });
            });

        if cancel || !open {
            self.confirm_delete = None;
        } else if confirm {
            match actions::move_to_recycle_bin(&path) {
                Ok(()) => {
                    self.confirm_delete = None;
                    self.selected = None;
                    self.start_search();
                    self.error = None;
                    self.set_feedback("Moved to Recycle Bin");
                }
                Err(error) => self.error = Some(error),
            }
        }
    }

    fn move_selection(&mut self, delta: isize) {
        if self.results.is_empty() {
            self.selected = None;
            return;
        }
        let current = self
            .selected
            .as_ref()
            .and_then(|path| self.results.iter().position(|item| &item.path == path))
            .unwrap_or(0);
        let next = (current as isize + delta).clamp(0, self.results.len() as isize - 1) as usize;
        self.selected = Some(self.results[next].path.clone());
    }
}

impl eframe::App for SearchApp {
    fn ui(&mut self, root_ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = root_ui.ctx().clone();
        self.poll_search();
        let mut run_search_now = false;
        let search_id = root_ui.make_persistent_id("search-query");
        let focused_before = ctx.memory(|memory| memory.focused());
        let search_had_focus = focused_before == Some(search_id);
        let result_had_focus =
            focused_before.is_some_and(|focused| self.result_focus_ids.contains(&focused));
        let tab_pressed = ctx.input(|input| input.key_pressed(Key::Tab));
        let enter_pressed = ctx.input(|input| input.key_pressed(Key::Enter));
        let move_focus_to_search = tab_pressed && result_had_focus;
        let move_focus_to_results = search_had_focus && (tab_pressed || enter_pressed);
        let mut current_result_focus_ids = Vec::new();
        let mut first_result_focus_id = None;
        root_ui
            .painter()
            .rect_filled(root_ui.max_rect(), egui::CornerRadius::ZERO, CANVAS);

        root_ui.vertical(|ui| {
            ui.add_space(4.0);
            egui::Frame::new()
                .fill(SURFACE)
                .stroke(egui::Stroke::new(1.0, BORDER))
                .corner_radius(egui::CornerRadius::same(8))
                .inner_margin(egui::Margin::same(16))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.heading(RichText::new("Find files and folders").size(22.0).strong());
                            ui.label(
                                RichText::new("Search names across your selected location")
                                    .color(MUTED),
                            );
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Settings").clicked() {
                                self.settings_draft = Some(self.config.clone());
                                self.settings_open = true;
                            }
                        });
                    });

                    ui.add_space(12.0);
                    ui.label(RichText::new("Name or keywords").strong());
                    ui.horizontal(|ui| {
                        let field_width = (ui.available_width() - 166.0).max(160.0);
                        let search = ui.add_sized(
                            [field_width, 38.0],
                            egui::TextEdit::singleline(&mut self.query)
                                .id(search_id)
                                .hint_text("For example: ramada resort"),
                        );
                        if !self.search_focus_requested {
                            search.request_focus();
                            self.search_focus_requested = true;
                        }
                        if search.gained_focus()
                            && !self.query.is_empty()
                            && !ui.input(|input| {
                                input.pointer.button_pressed(egui::PointerButton::Primary)
                            })
                            && let Some(mut state) = egui::TextEdit::load_state(&ctx, search.id)
                        {
                            state.cursor.set_char_range(Some(
                                egui::text::CCursorRange::select_all_str(&self.query),
                            ));
                            state.store(&ctx, search.id);
                            ctx.request_repaint();
                        }
                        if search.changed() {
                            self.query_deadline = Some(Instant::now() + Duration::from_millis(220));
                        }
                        if !self.query.is_empty() && ui.button("Clear").clicked() {
                            self.query.clear();
                            self.query_deadline = Some(Instant::now() + Duration::from_millis(220));
                        }
                        if primary_button(ui, "Search").clicked() {
                            run_search_now = true;
                        }
                    });

                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Search in").strong());
                        let path = self.config.search_path().display().to_string();
                        let path_width = (ui.available_width() - 220.0).max(100.0);
                        ui.allocate_ui_with_layout(
                            egui::vec2(path_width, 24.0),
                            egui::Layout::left_to_right(egui::Align::Center),
                            |ui| {
                                ui.add(
                                    egui::Label::new(RichText::new(&path).color(MUTED)).truncate(),
                                );
                            },
                        );
                        if ui.button("Change folder").clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .set_directory(self.config.search_path())
                                .pick_folder()
                            {
                                self.config.search_directory = path.to_string_lossy().into_owned();
                                if let Err(error) = self.config.save() {
                                    self.error = Some(format!("Could not save settings: {error}"));
                                }
                                self.start_search();
                            }
                        }

                        let search_type_label = match self.config.search_type {
                            SearchType::All => "All items",
                            SearchType::Folders => "Folders only",
                        };
                        ui.menu_button(search_type_label, |ui| {
                            if ui
                                .selectable_value(
                                    &mut self.config.search_type,
                                    SearchType::All,
                                    "Files and folders",
                                )
                                .clicked()
                            {
                                if let Err(error) = self.config.save() {
                                    self.error = Some(error);
                                }
                                self.start_search();
                                ui.close();
                            }
                            if ui
                                .selectable_value(
                                    &mut self.config.search_type,
                                    SearchType::Folders,
                                    "Folders only",
                                )
                                .clicked()
                            {
                                if let Err(error) = self.config.save() {
                                    self.error = Some(error);
                                }
                                self.start_search();
                                ui.close();
                            }
                        });
                    });
                ui.add_space(14.0);
                ui.separator();
                ui.add_space(8.0);

            if let Some(error) = self.error.clone() {
                ui.add_space(4.0);
                egui::Frame::new()
                    .fill(Color32::from_rgb(255, 242, 242))
                    .stroke(egui::Stroke::new(1.0, Color32::from_rgb(255, 201, 201)))
                    .corner_radius(egui::CornerRadius::same(6))
                    .inner_margin(egui::Margin::same(10))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(error).color(RED));
                            if ui.small_button("Dismiss").clicked() {
                                self.error = None;
                            }
                        });
                    });
            }

                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.heading(RichText::new("Results").size(17.0).strong());
                            ui.label(
                                RichText::new("Select an item to see available actions")
                                    .color(MUTED)
                                    .small(),
                            );
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if let Some(item) = self.selected_result() {
                                ui.menu_button("Actions", |ui| {
                                    for (label, kind) in [
                                        ("Open", FileActionKind::Open),
                                        ("Copy Path", FileActionKind::CopyPath),
                                        ("Copy Filename", FileActionKind::CopyFilename),
                                        ("Open in File Manager", FileActionKind::Reveal),
                                        ("Rename…", FileActionKind::Rename),
                                        ("Move to Archive", FileActionKind::Archive),
                                    ] {
                                        if ui.button(label).clicked() {
                                            self.pending_action = Some(FileAction {
                                                kind,
                                                path: item.path.clone(),
                                            });
                                            ui.close();
                                        }
                                    }
                                });
                            } else if !self.results.is_empty() {
                                ui.label(
                                    RichText::new("Double-click to open · right-click for actions")
                                        .color(MUTED)
                                        .small(),
                                );
                            }
                            let count = self.results.len();
                            egui::Frame::new()
                                .fill(CANVAS)
                                .corner_radius(egui::CornerRadius::same(12))
                                .inner_margin(egui::Margin::symmetric(9, 4))
                                .show(ui, |ui| {
                                    ui.label(RichText::new(format!("{count}")).strong());
                                });
                        });
                    });

                    ui.add_space(10.0);
                    if !self.results.is_empty() {
                        TableBuilder::new(ui)
                            .id_salt("search-results-table")
                            .striped(true)
                            .resizable(true)
                            .sense(egui::Sense::click())
                            .column(Column::initial(310.0).at_least(150.0).resizable(true))
                            .column(Column::initial(110.0).at_least(85.0))
                            .column(Column::remainder())
                            .header(34.0, |mut header| {
                                header.col(|ui| {
                                    ui.label(RichText::new("Name").strong().color(MUTED));
                                });
                                header.col(|ui| {
                                    ui.label(RichText::new("Type").strong().color(MUTED));
                                });
                                header.col(|ui| {
                                    ui.label(RichText::new("Location").strong().color(MUTED));
                                });
                            })
                            .body(|body| {
                                body.rows(40.0, self.results.len(), |mut row| {
                                    let index = row.index();
                                    let Some(item) = self.results.get(index).cloned() else {
                                        return;
                                    };
                                    let selected = self.selected.as_ref() == Some(&item.path);
                                    row.set_selected(selected);
                                    let mut name_widget_response = None;
                                    let (_, _name_cell_response) = row.col(|ui| {
                                        name_widget_response = Some(ui.add_sized(
                                            ui.available_size(),
                                            egui::Button::new(
                                                RichText::new(&item.name).color(FOREGROUND),
                                            )
                                            .frame(false)
                                            .wrap(),
                                        ));
                                    });
                                    let name_response = name_widget_response
                                        .expect("the result name button should be created");
                                    current_result_focus_ids.push(name_response.id);
                                    if index == 0 {
                                        first_result_focus_id = Some(name_response.id);
                                    }
                                    let (_, type_response) = row.col(|ui| {
                                        let (label, tint) = if item.is_folder {
                                            ("Folder", Color32::from_rgb(221, 244, 255))
                                        } else {
                                            ("File", CANVAS)
                                        };
                                        egui::Frame::new()
                                            .fill(tint)
                                            .corner_radius(egui::CornerRadius::same(10))
                                            .inner_margin(egui::Margin::symmetric(8, 3))
                                            .show(ui, |ui| {
                                                ui.label(
                                                    RichText::new(label)
                                                        .color(if item.is_folder {
                                                            BLUE
                                                        } else {
                                                            MUTED
                                                        })
                                                        .small(),
                                                );
                                            });
                                    });
                                    let (_, location_response) = row.col(|ui| {
                                        let location = item
                                            .path
                                            .parent()
                                            .unwrap_or(&item.path)
                                            .display()
                                            .to_string();
                                        ui.add(
                                            egui::Label::new(RichText::new(&location).color(MUTED))
                                                .truncate(),
                                        );
                                    });
                                    let response = row.response();
                                    if name_response.clicked() || response.clicked() {
                                        self.selected = Some(item.path.clone());
                                        ctx.memory_mut(|memory| {
                                            memory.request_focus(name_response.id)
                                        });
                                    }
                                    if name_response.double_clicked()
                                        || type_response.double_clicked()
                                        || location_response.double_clicked()
                                    {
                                        self.pending_action = Some(FileAction {
                                            kind: FileActionKind::Open,
                                            path: item.path.clone(),
                                        });
                                    }
                                    name_response.context_menu(|ui| {
                                        Self::show_context_menu(ui, &item, &mut self.pending_action)
                                    });
                                    type_response.context_menu(|ui| {
                                        Self::show_context_menu(ui, &item, &mut self.pending_action)
                                    });
                                    location_response.context_menu(|ui| {
                                        Self::show_context_menu(ui, &item, &mut self.pending_action)
                                    });
                                });
                            });
                    } else {
                        ui.allocate_ui_with_layout(
                            egui::vec2(
                                ui.available_width(),
                                (ui.available_height() - 12.0).max(220.0),
                            ),
                            egui::Layout::top_down(egui::Align::Center),
                            |ui| {
                                ui.centered_and_justified(|ui| {
                                    ui.vertical_centered(|ui| {
                                        ui.label(
                                            RichText::new(if self.searching {
                                                "Searching your files…"
                                            } else {
                                                "No matching files or folders"
                                            })
                                            .size(16.0)
                                            .strong(),
                                        );
                                        ui.label(
                                            RichText::new(if self.searching {
                                                "Results will appear here"
                                            } else {
                                                "Try a different name or search location"
                                            })
                                            .color(MUTED),
                                        );
                                    });
                                });
                            },
                        );
                    }
                });

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if let Some((message, expires)) = &self.feedback {
                    if Instant::now() < *expires {
                        ui.label(RichText::new("✓").color(GREEN).strong());
                        ui.label(RichText::new(message).color(GREEN));
                    }
                } else {
                    ui.label(
                        RichText::new("Enter to view results · Tab to switch between search and results · ↑/↓ to select · F2 to rename")
                            .color(MUTED)
                            .size(11.0),
                    );
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if self.searching {
                        ui.spinner();
                        ui.label(RichText::new("Searching…").color(MUTED).small());
                    } else {
                        ui.label(RichText::new(&self.status).color(MUTED).small());
                    }
                });
            });
        });

        self.result_focus_ids = current_result_focus_ids;
        if move_focus_to_search {
            self.focus_first_result_when_ready = false;
            ctx.memory_mut(|memory| memory.request_focus(search_id));
        } else if move_focus_to_results {
            self.focus_first_result_when_ready = true;
            if self.query != self.last_submitted_query || self.query_deadline.is_some() {
                run_search_now = true;
            }
        }

        if run_search_now {
            self.start_search();
        } else if let Some(deadline) = self.query_deadline {
            if Instant::now() >= deadline && self.query != self.last_submitted_query {
                self.start_search();
            } else if self.query != self.last_submitted_query {
                ctx.request_repaint_after(Duration::from_millis(40));
            }
        }

        if self.focus_first_result_when_ready
            && !self.searching
            && let Some(first_id) = first_result_focus_id
            && let Some(first_result) = self.results.first()
        {
            self.selected = Some(first_result.path.clone());
            ctx.memory_mut(|memory| memory.request_focus(first_id));
            self.focus_first_result_when_ready = false;
        }

        if !ctx.egui_wants_keyboard_input()
            && !ctx.any_popup_open()
            && !(search_had_focus && enter_pressed)
        {
            if ctx.input(|input| input.key_pressed(Key::ArrowDown)) {
                self.move_selection(1);
            } else if ctx.input(|input| input.key_pressed(Key::ArrowUp)) {
                self.move_selection(-1);
            } else if ctx.input(|input| input.key_pressed(Key::Enter)) {
                if let Some(item) = self.selected_result() {
                    self.queue_action(FileActionKind::Open, item.path);
                }
            } else if ctx.input(|input| input.key_pressed(Key::F2)) {
                if let Some(item) = self.selected_result() {
                    self.queue_action(FileActionKind::Rename, item.path);
                }
            }
        }

        self.run_pending_action();
        self.show_settings(&ctx);
        self.show_rename_dialog(&ctx);
        self.show_delete_confirmation(&ctx);

        if self.searching {
            ctx.request_repaint_after(Duration::from_millis(50));
        }
        if self
            .feedback
            .as_ref()
            .is_some_and(|(_, expires)| Instant::now() < *expires)
        {
            ctx.request_repaint_after(Duration::from_millis(100));
        } else {
            self.feedback = None;
        }
    }
}

fn main() -> eframe::Result<()> {
    env_logger::init();
    let icon_image =
        image::load_from_memory(include_bytes!("../assets/logo/search-app-icon-256.png"))
            .expect("the bundled Search app icon should be a valid PNG")
            .into_rgba8();
    let (icon_width, icon_height) = icon_image.dimensions();
    let app_icon = egui::IconData {
        rgba: icon_image.into_raw(),
        width: icon_width,
        height: icon_height,
    };

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Search")
            .with_icon(app_icon)
            .with_inner_size([1080.0, 700.0])
            .with_min_inner_size([720.0, 420.0]),
        centered: true,
        ..Default::default()
    };
    eframe::run_native(
        "Search",
        options,
        Box::new(|cc| Ok(Box::new(SearchApp::new(cc)))),
    )
}
