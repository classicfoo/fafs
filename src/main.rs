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
}

impl SearchApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = Color32::from_rgb(18, 22, 30);
        visuals.window_fill = Color32::from_rgb(23, 28, 38);
        visuals.extreme_bg_color = Color32::from_rgb(12, 15, 21);
        visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(28, 34, 45);
        visuals.widgets.inactive.bg_fill = Color32::from_rgb(33, 40, 53);
        visuals.widgets.hovered.bg_fill = Color32::from_rgb(44, 56, 73);
        visuals.widgets.active.bg_fill = Color32::from_rgb(49, 102, 130);
        visuals.selection.bg_fill = Color32::from_rgb(28, 106, 137);
        cc.egui_ctx.set_visuals(visuals);

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
                self.status = message.to_owned();
                self.error = None;
                if matches!(action.kind, FileActionKind::Touch) {
                    self.start_search();
                }
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
                    self.status = "Moved to Recycle Bin".to_owned();
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

        root_ui.vertical(|ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.heading(RichText::new("Search").size(24.0).strong());
                ui.add_space(12.0);
                let search = ui.add(
                    egui::TextEdit::singleline(&mut self.query)
                        .hint_text("Search file and folder names…")
                        .desired_width((ui.available_width() - 270.0).max(180.0)),
                );
                if search.changed() {
                    self.query_deadline = Some(Instant::now() + Duration::from_millis(220));
                }
                if search.has_focus() && ui.input(|input| input.key_pressed(Key::Enter)) {
                    run_search_now = true;
                }
                if ui.button("Search").clicked() {
                    run_search_now = true;
                }
                if ui.button("Folder…").clicked() {
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
                if ui.button("Settings").clicked() {
                    self.settings_draft = Some(self.config.clone());
                    self.settings_open = true;
                }
            });

            ui.horizontal(|ui| {
                ui.label(RichText::new(self.config.search_path().display().to_string()).weak());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.menu_button("Search Type", |ui| {
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
                    }
                });
            });
            ui.add_space(5.0);
        });

        root_ui.vertical(|ui| {
            if let Some(error) = self.error.clone() {
                ui.horizontal(|ui| {
                    ui.colored_label(Color32::from_rgb(255, 130, 120), error);
                    if ui.small_button("Dismiss").clicked() {
                        self.error = None;
                    }
                });
            }

            TableBuilder::new(ui)
                .id_salt("search-results-table")
                .striped(true)
                .resizable(true)
                .column(Column::initial(260.0).at_least(120.0).resizable(true))
                .column(Column::initial(90.0).at_least(70.0))
                .column(Column::remainder())
                .header(28.0, |mut header| {
                    header.col(|ui| {
                        ui.heading("Name");
                    });
                    header.col(|ui| {
                        ui.heading("Type");
                    });
                    header.col(|ui| {
                        ui.heading("Path");
                    });
                })
                .body(|body| {
                    body.rows(28.0, self.results.len(), |mut row| {
                        let index = row.index();
                        let Some(item) = self.results.get(index).cloned() else {
                            return;
                        };
                        let selected = self.selected.as_ref() == Some(&item.path);
                        row.col(|ui| {
                            let response = ui.selectable_label(selected, &item.name);
                            if response.clicked() {
                                self.selected = Some(item.path.clone());
                            }
                            if response.double_clicked() {
                                self.pending_action = Some(FileAction {
                                    kind: FileActionKind::Open,
                                    path: item.path.clone(),
                                });
                            }
                            response.context_menu(|ui| {
                                Self::show_context_menu(ui, &item, &mut self.pending_action)
                            });
                        });
                        row.col(|ui| {
                            ui.label(if item.is_folder { "Folder" } else { "File" });
                        });
                        row.col(|ui| {
                            let text = item.path.display().to_string();
                            ui.add_sized(
                                [ui.available_width(), 22.0],
                                egui::Label::new(text).truncate(),
                            );
                        });
                    });
                });

            if self.results.is_empty() && !self.searching && self.error.is_none() {
                ui.centered_and_justified(|ui| {
                    ui.label("No matching files or folders");
                });
            }
        });

        root_ui.vertical(|ui| {
            ui.horizontal(|ui| {
                if self.searching {
                    ui.spinner();
                    ui.label("Searching…");
                } else {
                    ui.label(RichText::new(&self.status).weak());
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new("Enter open · F2 rename · Ctrl+Backspace delete word")
                            .weak()
                            .small(),
                    );
                });
            });
        });

        if let Some(deadline) = self.query_deadline {
            if Instant::now() >= deadline && self.query != self.last_submitted_query {
                self.start_search();
            } else if self.query != self.last_submitted_query {
                ctx.request_repaint_after(Duration::from_millis(40));
            }
        }

        if run_search_now {
            self.start_search();
        }

        if !ctx.egui_wants_keyboard_input() && !ctx.any_popup_open() {
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
    }
}

fn main() -> eframe::Result<()> {
    env_logger::init();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Search")
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
