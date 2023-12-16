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
            item_path = item_values[1]  # Get the full path (index 1)
            new_path = convert_spaces_to_underscores(item_path)
            if new_path:
                # Update the Treeview values with the new path
                results.item(item, values=[item_values[0], new_path])
                
                # Update the Treeview text with the new filename
                new_filename = os.path.basename(new_path)
                results.item(item, text=new_filename)

def search_files(event=None):
    query = entry.get()
    keywords = query.split()  # Split the input into multiple keywords
    file_list = []

    for root, dirs, files in os.walk(os.getcwd()):
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
        results.insert('', 'end', text=item, values=[item_type, full_path])

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
    item = results.selection()
    if item:
        item = results.item(item, "values")[1]  # Get the full path (index 1)
        os.startfile(item)

def double_click(event):
    item = results.selection()
    if item:
        item = results.item(item, "values")[1]  # Get the full path (index 1)
        os.startfile(item)

def copy_path_to_clipboard():
    item = results.selection()
    if item:
        item = results.item(item, "values")[1]  # Get the full path (index 1)
        pyperclip.copy(item)

def copy_filename_to_clipboard():
    item = results.selection()
    if item:
        item = results.item(item, "values")[1]  # Get the full path (index 1)
        filename = os.path.basename(item)  # Get only the filename
        filename_with_quotes = f'"{filename}"'  # Add quotes around the filename
        pyperclip.copy(filename_with_quotes)

def open_in_explorer():
    item = results.selection()
    if item:
        item = results.item(item, "values")[1]  # Get the full path (index 1)
        os.system(f'explorer /select,"{item}"')

def touch_command():
    item = results.selection()
    if item:
        file_path = results.item(item, 'values')[1]   # Get the full path (index 1)
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
            item_path = item_values[1]  # Get the full path (index 1)
            
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
                    results.item(item, values=[item_values[0], new_path])

                    # Update the Treeview text with the new filename
                    new_filename = os.path.basename(new_path)
                    results.item(item, text=new_filename)

def move_to_recycle_bin():
    item = results.selection()
    if item:
        item_values = results.item(item, "values")
        if item_values:
            item_path = item_values[1]  # Get the full path (index 1)

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
        file_path = results.item(item, 'values')[1]   # Get the full path (index 1)

    # Check if the file is a text file (you can add more extensions)
    if file_path.endswith(('.txt','.md')):
        editor_path = "C:\\Users\\micha\\Documents\\my_documents\\projects\\editor\\editor.pyw"
        subprocess.Popen(['python', editor_path, file_path])
    else:
        tkinter.messagebox.messagebox.showinfo("Info", "Selected file is not a text file.")

# Assuming you have some way in your GUI to trigger this function (e.g., double-click, button, etc.)

# Create the main window
window = tk.Tk()
window.title("File and Folder Search")
center_window(window)

# Bind the confirm_close function to the WM_DELETE_WINDOW protocol
window.protocol("WM_DELETE_WINDOW", confirm_close)

# Create an entry widget for user input
entry = tk.Entry(window, width=30)
entry.grid(row=0, column=0, padx=10, pady=10, sticky="ew")
entry.focus()

# Bind the <Return> key event to the search function
entry.bind('<Return>', search_files)
# Bind the Ctrl+Backspace key combination to the delete_previous_word function
entry.bind("<Control-BackSpace>", delete_previous_word)

# Create a treeview widget to display results with three columns
results = ttk.Treeview(window, columns=("Type", "Path"))
results.heading("#0", text="Item")  # Label the first column as "Item"
results.heading("#1", text="Type")  # Label the second column as "Type"
results.heading("#2", text="Path")  # Label the third column as "Path"

# Configure column layout using the `column` method
results.column("#0", width=200)
results.column("#1", width=100)
results.column("#2", width=400)

# Bind double-click event to open items
results.bind('<Double-1>', double_click)

# Add a vertical scrollbar to the results
scrollbar = ttk.Scrollbar(window, orient="vertical", command=results.yview)
scrollbar.grid(row=1, column=1, sticky="ns")
results.configure(yscrollcommand=scrollbar.set)

# Configure row and column weights for resizing
window.grid_rowconfigure(1, weight=1)
window.grid_columnconfigure(0, weight=1)

# Place the Treeview widget in the window
results.grid(row=1, column=0, padx=10, pady=10, sticky="nsew")

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

window.mainloop()