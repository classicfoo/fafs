import os
import tkinter as tk
from tkinter import ttk
import tkinter.simpledialog
import tkinter.messagebox
import pyperclip  # A library for clipboard operations
import time
import send2trash  # Import the send2trash library for moving files to the recycle bin
from functools import partial  # Import functools.partial
import subprocess
import ctypes
import json
from tkinter import filedialog

def move_to_archive():
    item = results.selection()
    if item:
        item_values = results.item(item, "values")
        if item_values:
            item_path = item_values[2]  # Get the full path (index 2)
            
            # Get archive folder from config
            archive_folder = load_config()['archive_directory']
            
            # Construct the new path in the archive folder
            new_path = os.path.join(archive_folder, os.path.basename(item_path))

            try:
                # Move the file/folder to the archive folder
                os.rename(item_path, new_path)
            except Exception as e:
                tkinter.messagebox.showerror("Error", f"An error occurred: {str(e)}")
            else:
                # Re-perform the search to refresh the results
                search_files()

def select_first_item(treeview):
    # Get the first item in the Treeview (if available)
    first_item = treeview.get_children()
    
    # Select the first item
    if first_item:
        treeview.selection_set(first_item[0])

def on_entry_focus_in(event, entry, tree):
    # Deselect any items in the Treeview widget
    tree.selection_remove(tree.selection())

def delete_previous_word(event):
    # Get the current content of the text entry field
    current_text = entry.get()
    
    # Find the cursor position in the text entry field
    cursor_index = entry.index(tk.INSERT)
    
    # Find the start index of the word to be deleted
    start_index = cursor_index
    while start_index > 0 and current_text[start_index - 1] != ' ':
        start_index -= 1

    # Delete the previous word
    entry.delete(start_index, cursor_index)

def center_window(window):
    window_width = 700
    window_height = 300
    screen_width = window.winfo_screenwidth()
    screen_height = window.winfo_screenheight()
    x = (screen_width - window_width) // 2
    y = (screen_height - window_height) // 2
    window.geometry(f"{window_width}x{window_height}+{x}+{y}")

def convert_spaces_to_underscores(item_path):
    try:
        # Extract the directory and filename from the path
        directory, filename = os.path.split(item_path)
        
        # Replace spaces with underscores in the filename
        new_filename = filename.replace(' ', '_')
        
        # Convert filenames tolower
        new_filename = new_filename.lower()
        

        # Construct the new path with the converted filename
        new_path = os.path.join(directory, new_filename)
        
        # Rename the file
        os.rename(item_path, new_path)
        
        # Return the new path
        return new_path
    except Exception as e:
        tkinter.messagebox.showerror("Error", f"An error occurred: {str(e)}")
        return None

def convert_spaces_to_underscores_context_menu():
    item = results.selection()
    if item:
        item_values = results.item(item, "values")
        if item_values:
            item_path = item_values[2]  # Get the full path (index 2)
            new_path = convert_spaces_to_underscores(item_path)
            if new_path:
                # Update the Treeview values with the new path
                results.item(item, values=[item_values[0], item_values[1], new_path])
                
                # Update the Treeview text with the new filename
                new_filename = os.path.basename(new_path)
                results.item(item, text=new_filename)

def search_files(event=None):
    query = entry.get()
    keywords = query.split()
    file_list = []
    
    search_dir = get_search_directory()  # Function to get configured directory
    for root, dirs, files in os.walk(search_dir):
        for item in files + dirs:
            item_lower = item.lower()
            if all(keyword.lower() in item_lower for keyword in keywords):
                full_path = os.path.join(root, item)
                item_type = "Folder" if os.path.isdir(full_path) else "File"
                modified_time = os.path.getmtime(full_path)
                file_list.append((item, item_type, full_path, modified_time))

    # Sort the file list by modification time (newest first)
    file_list.sort(key=lambda x: x[3], reverse=True)

    results.delete(*results.get_children())

    for item_info in file_list:
        item, item_type, full_path, _ = item_info
        results.insert('', 'end', values=[item, item_type, full_path])

    # Set focus to the Treeview
    results.focus_set()

    # Select the first item if there are results
    if results.get_children():
        first_item = results.get_children()[0]
        results.selection_set(first_item)
        results.focus(first_item) # need this to use keyboard arrows

    # Bind the <Return> key event to open the selected item
    results.bind('<Return>', open_item)

def open_item(event):
    #item = results.selection()
    #if item:
    #    item = results.item(item, "values")[1]  # Get the full path (index 1)
    #    os.startfile(item)
    double_click()

def double_click(event=None):
    item = results.selection()

    if item:
        file_path = results.item(item, 'values')[2]   # Get the full path (index 2)

        # Check if the file is a text file (you can add more extensions)
        if file_path.endswith(('.txt','.md')):
            # Get editor path from config
            editor_path = load_config()['editor_path']
            subprocess.Popen(['python', editor_path, file_path], creationflags=subprocess.CREATE_NO_WINDOW)
        else:
            # Just open the file with default application, no message needed
            os.startfile(file_path)

def copy_path_to_clipboard():
    item = results.selection()
    if item:
        item = results.item(item, "values")[2]  # Get the full path (index 2)
        pyperclip.copy(item)

def copy_filename_to_clipboard():
    item = results.selection()
    if item:
        item = results.item(item, "values")[2]  # Get the full path (index 2)
        filename = os.path.basename(item)  # Get only the filename
        filename_with_quotes = f'"{filename}"'  # Add quotes around the filename
        pyperclip.copy(filename_with_quotes)

def open_in_explorer():
    item = results.selection()
    if item:
        item = results.item(item, "values")[2]  # Get the full path (index 2)
        os.system(f'explorer /select,"{item}"')

def touch_command():
    item = results.selection()
    if item:
        file_path = results.item(item, 'values')[2]   # Get the full path (index 2)
        current_time = time.time()
        os.utime(file_path, (current_time, current_time))  # Update the modification time
        search_files()

def confirm_close():
    confirmed = tkinter.messagebox.askyesno("Confirm Close", "Are you sure you want to close the program?")
    if confirmed:
        window.destroy()  # Close the program if the user confirms


def rename_item():
    item = results.selection()
    if item:
        item_values = results.item(item, "values")
        if item_values:
            item_path = item_values[2]  # Get the full path (index 2)
            
            # Extract the existing name from the path
            existing_name = os.path.basename(item_path)
            
            # Ask the user for the new name and prepopulate it with the existing name
            new_name = tkinter.simpledialog.askstring("Rename", "Enter a new name:\t\t\t", initialvalue=existing_name)
            if new_name:
                new_path = os.path.join(os.path.dirname(item_path), new_name)
                try:
                    os.rename(item_path, new_path)
                except Exception as e:
                    tkinter.messagebox.showerror("Error", f"An error occurred: {str(e)}")
                else:
                    # Update the Treeview values with the new path
                    results.item(item, values=[item_values[0], item_values[1], new_path])

                    # Update the Treeview text with the new filename
                    new_filename = os.path.basename(new_path)
                    results.item(item, text=new_filename)

def move_to_recycle_bin():
    item = results.selection()
    if item:
        item_values = results.item(item, "values")
        if item_values:
            item_path = item_values[2]  # Get the full path (index 2)

            # Prompt the user to confirm the action
            confirmed = tkinter.messagebox.askyesno("Confirm Move to Recycle Bin", f"Are you sure you want to move {item_path} to the recycle bin?")
            if confirmed:
                try:
                    send2trash.send2trash(item_path)  # Move the file to the recycle bin
                except Exception as e:
                    tkinter.messagebox.showerror("Error", f"An error occurred: {str(e)}")
                else:
                    # Remove the item from the Treeview
                    results.delete(item)

def open_with_editor():
    item = results.selection()
    if item:
        file_path = results.item(item, 'values')[2]   # Get the full path (index 2)

        # Check if the file is a text file (you can add more extensions)
        if file_path.endswith(('.txt','.md')):
            # Get editor path from config
            editor_path = load_config()['editor_path']
            subprocess.Popen(['python', editor_path, file_path], creationflags=subprocess.CREATE_NO_WINDOW)
        else:
            tkinter.messagebox.showinfo("Info", "Selected file is not a text file.")

def load_config():
    config_path = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'config.json')
    try:
        with open(config_path, 'r') as f:
            return json.load(f)
    except FileNotFoundError:
        config = {
            'search_directory': "C:\\Users\\61415\\Documents\\_my_documents",
            'archive_directory': "C:\\Users\\61415\\Documents\\archive",
            'editor_path': "C:\\Users\\61415\\Documents\\_my_documents\\projects\\editor\\editor.pyw"
        }
        with open(config_path, 'w') as f:
            json.dump(config, f, indent=4)
        return config

def get_search_directory():
    config = load_config()
    return config['search_directory']

# Create the main window
window = tk.Tk()
window.title("File and Folder Search")

# Apply the clam theme and configure styles
style = ttk.Style()
style.theme_use('clam')

style.configure("Vertical.TScrollbar", arrowsize=16, width=21) # keep this scrollbar width consistent

# Configure some style elements
style.configure('Treeview', rowheight=25)  # Make rows a bit taller
style.configure('Treeview.Heading', font=('TkDefaultFont', 9, 'bold'))  # Make headers bold
style.configure('TEntry', padding=5)  # Add padding to entry

center_window(window)

# Bind the confirm_close function to the WM_DELETE_WINDOW protocol
window.protocol("WM_DELETE_WINDOW", confirm_close)

# Configure main window grid weights
window.grid_rowconfigure(1, weight=1)  # Make the treeview row expandable
window.grid_columnconfigure(0, weight=1)  # Make the window width expandable

# Create an entry widget for user input
entry = ttk.Entry(window)
entry.grid(row=0, column=0, columnspan=2, sticky="ew", padx=5, pady=(5,5))
entry.focus()

# Bind the <Return> key event to the search function
entry.bind('<Return>', search_files)
# Bind the Ctrl+Backspace key combination to the delete_previous_word function
entry.bind("<Control-BackSpace>", delete_previous_word)

# Create a frame to hold both the treeview and scrollbar
tree_frame = ttk.Frame(window)
tree_frame.grid(row=1, column=0, columnspan=2, sticky="nsew", padx=5, pady=(0,5))

# Configure tree_frame grid weights
tree_frame.grid_rowconfigure(0, weight=1)
tree_frame.grid_columnconfigure(0, weight=1)

# Create a treeview widget to display results
results = ttk.Treeview(tree_frame, columns=("Item", "Type", "Path"), show="headings")
results.heading("Item", text="Item")  # Label the first column as "Item"
results.heading("Type", text="Type")  # Label the second column as "Type"
results.heading("Path", text="Path")  # Label the third column as "Path"

# Configure column layout
results.column("Item", width=100)
results.column("Type", width=100)
results.column("Path", width=400)

# Create and configure the scrollbar to be integrated with the treeview
scrollbar = ttk.Scrollbar(tree_frame, orient="vertical", command=results.yview)
results.configure(yscrollcommand=scrollbar.set)

# Grid layout - place them together
results.grid(row=0, column=0, sticky="nsew")
scrollbar.grid(row=0, column=1, sticky="ns")

# Bind double-click event to open items
results.bind('<Double-1>', double_click)

# Move these before the context menu creation
class SettingsDialog(tk.Toplevel):
    def __init__(self, parent):
        super().__init__(parent)
        self.title("Settings")
        
        # Apply the same theme
        style = ttk.Style()
        
        # Configure the dialog
        self.resizable(False, False)
        self.transient(parent)
        self.grab_set()
        
        # Create main frame with padding
        main_frame = ttk.Frame(self, padding="10")
        main_frame.grid(row=0, column=0, sticky="nsew")
        
        # Directory settings
        ttk.Label(main_frame, text="Search Directory:").grid(row=0, column=0, sticky="w", pady=(0,5))
        self.search_dir_var = tk.StringVar(value=get_search_directory())
        search_dir_entry = ttk.Entry(main_frame, textvariable=self.search_dir_var, width=50)
        search_dir_entry.grid(row=1, column=0, sticky="ew", padx=(0,5))
        ttk.Button(main_frame, text="Browse", command=self.browse_search_dir).grid(row=1, column=1)
        
        ttk.Label(main_frame, text="Archive Directory:").grid(row=2, column=0, sticky="w", pady=(10,5))
        self.archive_dir_var = tk.StringVar(value=load_config()['archive_directory'])
        archive_dir_entry = ttk.Entry(main_frame, textvariable=self.archive_dir_var, width=50)
        archive_dir_entry.grid(row=3, column=0, sticky="ew", padx=(0,5))
        ttk.Button(main_frame, text="Browse", command=self.browse_archive_dir).grid(row=3, column=1)
        
        ttk.Label(main_frame, text="Editor Path:").grid(row=4, column=0, sticky="w", pady=(10,5))
        self.editor_path_var = tk.StringVar(value=load_config()['editor_path'])
        editor_path_entry = ttk.Entry(main_frame, textvariable=self.editor_path_var, width=50)
        editor_path_entry.grid(row=5, column=0, sticky="ew", padx=(0,5))
        ttk.Button(main_frame, text="Browse", command=self.browse_editor_path).grid(row=5, column=1)
        
        # Buttons
        button_frame = ttk.Frame(main_frame)
        button_frame.grid(row=6, column=0, columnspan=2, pady=(15,0))
        ttk.Button(button_frame, text="Save", command=self.save_settings).pack(side="right", padx=5)
        ttk.Button(button_frame, text="Cancel", command=self.destroy).pack(side="right")
        
        # Center the dialog
        self.center_dialog()
    
    def center_dialog(self):
        self.update_idletasks()
        width = self.winfo_width()
        height = self.winfo_height()
        x = (self.winfo_screenwidth() // 2) - (width // 2)
        y = (self.winfo_screenheight() // 2) - (height // 2)
        self.geometry(f'{width}x{height}+{x}+{y}')
    
    def browse_search_dir(self):
        directory = tk.filedialog.askdirectory(initialdir=self.search_dir_var.get())
        if directory:
            self.search_dir_var.set(directory)
    
    def browse_archive_dir(self):
        directory = tk.filedialog.askdirectory(initialdir=self.archive_dir_var.get())
        if directory:
            self.archive_dir_var.set(directory)
    
    def browse_editor_path(self):
        file_path = tk.filedialog.askopenfilename(
            initialdir=os.path.dirname(self.editor_path_var.get()),
            filetypes=[("Python files", "*.py;*.pyw"), ("All files", "*.*")]
        )
        if file_path:
            self.editor_path_var.set(file_path)
    
    def save_settings(self):
        config = {
            'search_directory': self.search_dir_var.get(),
            'archive_directory': self.archive_dir_var.get(),
            'editor_path': self.editor_path_var.get()
        }
        with open('config.json', 'w') as f:
            json.dump(config, f, indent=4)
        self.destroy()

def open_settings():
    SettingsDialog(window)

# Create a context menu for the Treeview
context_menu = tk.Menu(window, tearoff=0)
context_menu.add_command(label="Copy Path", command=copy_path_to_clipboard)
context_menu.add_command(label="Copy Filename", command=copy_filename_to_clipboard)
context_menu.add_command(label="Open in Explorer", command=open_in_explorer)
context_menu.add_command(label="Touch", command=touch_command)
context_menu.add_command(label="Open with Editor", command=open_with_editor)
context_menu.add_command(label="Spaces to Underscores", command=convert_spaces_to_underscores_context_menu)
context_menu.add_command(label="Rename", command=rename_item)
context_menu.add_command(label="Move to Recycle Bin", command=move_to_recycle_bin)
context_menu.add_command(label="Move to Archive", command=move_to_archive)
context_menu.add_separator()  # Add a horizontal line
context_menu.add_command(label="Settings", command=open_settings)


def show_context_menu(event):
    item = results.identify_row(event.y)
    if item:
        on_right_click(event)
        context_menu.post(event.x_root, event.y_root)

def on_right_click(event):
    # Identify the region at the cursor (column, row, etc.)
    region = results.identify_region(event.x, event.y)
    
    # Check if the region is a row (an item in the TreeView)
    if region == "tree":
        # Identify the row under the cursor
        row_id = results.identify_row(event.y)
        
        # Change the selection to this row
        results.selection_set(row_id)

# Bind the right-click event to show the context menu
results.bind("<Button-3>", show_context_menu)

# Bind the focus-in event of the Entry widget to the on_entry_focus_in function
entry.bind("<FocusIn>", partial(on_entry_focus_in, entry=entry, tree=results))

# Bind the focus-in event of the Treeview widget to select the first item
results.bind("<FocusIn>", lambda event: select_first_item(results))

results.bind('<F2>', lambda e: rename_item())

# Bind the Backspace key to the move_to_archive function for the results Treeview
# results.bind('<BackSpace>', lambda e: move_to_archive())

# after bindings are set up
search_files()

window.mainloop()