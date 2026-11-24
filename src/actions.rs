use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

pub fn copy_text(text: String) -> Result<(), String> {
    let mut clipboard = arboard::Clipboard::new().map_err(|error| error.to_string())?;
    clipboard.set_text(text).map_err(|error| error.to_string())
}

#[cfg(windows)]
pub fn copy_file(path: &Path) -> Result<(), String> {
    use clipboard_win::{Clipboard, Setter, formats};

    let _clipboard = Clipboard::new_attempts(10).map_err(|error| error.to_string())?;
    let path = path.to_string_lossy().into_owned();
    formats::FileList
        .write_clipboard(&[path])
        .map_err(|error| error.to_string())
}

#[cfg(not(windows))]
pub fn copy_file(_path: &Path) -> Result<(), String> {
    Err("Copying files to the clipboard is supported on Windows only".to_owned())
}

pub fn open_path(path: &Path) -> Result<(), String> {
    open::that(path).map_err(|error| error.to_string())
}

pub fn open_with_editor(path: &Path, editor: &Path) -> Result<(), String> {
    if editor.as_os_str().is_empty() {
        return open_path(path);
    }

    let extension = editor
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let result = if matches!(extension.as_str(), "py" | "pyw") {
        #[cfg(windows)]
        let mut command = {
            let mut command = Command::new(if extension == "pyw" {
                "pythonw"
            } else {
                "python"
            });
            command.arg(editor);
            command
        };
        #[cfg(not(windows))]
        let mut command = {
            let mut command = Command::new("python3");
            command.arg(editor);
            command
        };
        command.arg(path).spawn()
    } else {
        Command::new(editor).arg(path).spawn()
    };

    result
        .map(|_| ())
        .map_err(|error| format!("Could not start {}: {error}", editor.display()))
}

pub fn reveal_in_file_manager(path: &Path) -> Result<(), String> {
    #[cfg(windows)]
    let result = Command::new("explorer.exe")
        .arg(format!("/select,{}", path.display()))
        .spawn();

    #[cfg(target_os = "linux")]
    let result = Command::new("xdg-open")
        .arg(path.parent().unwrap_or(path))
        .spawn();

    #[cfg(target_os = "macos")]
    let result = Command::new("open").arg("-R").arg(path).spawn();

    result
        .map(|_| ())
        .map_err(|error| format!("Could not open the file manager: {error}"))
}

pub fn touch(path: &Path) -> Result<(), String> {
    filetime::set_file_mtime(path, filetime::FileTime::now()).map_err(|error| error.to_string())
}

pub fn rename(path: &Path, new_name: &str) -> Result<PathBuf, String> {
    let new_name = new_name.trim();
    if new_name.is_empty() || new_name == "." || new_name == ".." {
        return Err("Enter a valid file or folder name".to_owned());
    }
    if new_name.contains(['/', '\\']) {
        return Err("Enter a name without directory separators".to_owned());
    }

    let parent = path
        .parent()
        .ok_or_else(|| "The selected path has no parent directory".to_owned())?;
    let destination = parent.join(new_name);
    if destination.exists() && destination != path {
        return Err(format!("An item named {new_name:?} already exists"));
    }
    fs::rename(path, &destination).map_err(|error| error.to_string())?;
    Ok(destination)
}

pub fn spaces_to_underscores(path: &Path) -> Result<PathBuf, String> {
    let old_name = path
        .file_name()
        .ok_or_else(|| "The selected path has no file name".to_owned())?;
    let new_name = old_name.to_string_lossy().replace(' ', "_").to_lowercase();
    rename(path, &new_name)
}

pub fn move_to_archive(path: &Path, archive_directory: &Path) -> Result<PathBuf, String> {
    fs::create_dir_all(archive_directory).map_err(|error| error.to_string())?;
    let name = path
        .file_name()
        .ok_or_else(|| "The selected path has no file name".to_owned())?;
    let destination = archive_directory.join(name);
    if destination.exists() {
        return Err(format!("{} already exists", destination.display()));
    }
    fs::rename(path, &destination).map_err(|error| {
        format!(
            "Could not move the item to the archive at {}: {error}",
            archive_directory.display()
        )
    })?;
    Ok(destination)
}

pub fn move_to_recycle_bin(path: &Path) -> Result<(), String> {
    trash::delete(path).map_err(|error| error.to_string())
}

pub fn filename(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}
