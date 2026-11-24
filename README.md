# Search

<p align="center"><img src="assets/logo/search-app-icon.svg" width="112" alt="Search squared S app icon"></p>

Search is a desktop file and folder finder built with Rust and egui. It runs on Linux and Windows.

## Windows download

Download the latest portable [Search.exe](https://github.com/classicfoo/fafs/releases/latest/download/search.exe) from the [GitHub Releases page](https://github.com/classicfoo/fafs/releases).

## Features

- Live, case-insensitive filename search across nested folders. Separate words match independently, and results are sorted by modification time with the newest first.
- Search all files and folders or folders only.
- Open an item with its default application by double-clicking or pressing Enter.
- Right-click actions for copying a path or quoted filename, revealing an item in the file manager, opening a text file with an editor, touching its modification time, renaming, converting spaces to lowercase underscores, archiving, and moving it to the system recycle bin/trash.
- Copy a file or folder directly to the clipboard on Windows.
- Search root, archive folder, editor, and result type settings saved in the operating system's per-user configuration directory.
- F2 renames the selected item. Ctrl+Backspace removes the previous word from the search field.

## Build

Install Rust with Cargo, then run:

```sh
cargo build --release
```

On Linux, the window system development packages for X11/Wayland and OpenGL are required. The GitHub Actions workflow builds on Ubuntu and Windows. On Windows, run the same Cargo command from a Rust toolchain prompt.

## Settings

Open **Settings** in the app to choose the search directory, archive directory, optional editor application or script, and whether to include files or show folders only. The first launch uses the Documents folder when available, otherwise the home folder. An empty editor setting opens text files with the system's default application.

The bundled `config.example.json` shows the settings format. User settings are stored under the platform's standard configuration directory and are not written into the source checkout.

## License

This project is licensed under GPL-3.0-or-later. See [LICENSE](LICENSE).
