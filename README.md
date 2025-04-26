# File and Folder Search

A simple file and folder search utility built with Python and Tkinter.

![Screenshot of File and Folder Search](screenshot.jpg)

## Features

- Real-time search as you type
- Search through files and folders in current directory and subdirectories
- Double-click to open files/folders
- Right-click context menu with options:
  - Copy Path
  - Copy Filename
  - Open in Explorer
  - Touch (update timestamp)
  - Open with Editor (for text files)
  - Convert Spaces to Underscores
  - Rename
  - Move to Recycle Bin
  - Move to Archive
- Keyboard shortcuts:
  - F2: Rename selected item
  - Ctrl+Backspace: Delete previous word in search
  - Enter: Open selected item

## Requirements

- Python 3.x
- pyperclip
- send2trash

## Usage

Run `search.pyw` to start the application. Type in the search box to filter files and folders.
