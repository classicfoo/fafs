import unittest
import os
import re
import json

class TestSearchStructure(unittest.TestCase):
    
    def setUp(self):
        # Load the search.pyw file content
        with open('search.pyw', 'r', encoding='utf-8') as f:
            self.code = f.read()
        
        # Load config.json
        with open('config.json', 'r') as f:
            self.config = json.load(f)
    
    def test_scrollbar_configuration(self):
        """Test that scrollbar has the correct style configuration"""
        print("\nChecking scrollbar configuration...")
        
        # Check for scrollbar width configuration
        scrollbar_width_pattern = r'style\.configure\("Vertical\.TScrollbar".*width=(\d+)'
        width_match = re.search(scrollbar_width_pattern, self.code)
        self.assertTrue(width_match, "Scrollbar width configuration not found")
        
        # Extract the actual width value
        actual_width = width_match.group(1)
        expected_width = "21"
        print(f"✓ Scrollbar width found: {actual_width} (expected: {expected_width})")
        self.assertEqual(actual_width, expected_width, 
                        f"Scrollbar width is {actual_width}, but expected {expected_width}")
        
        # Check for scrollbar arrowsize configuration
        scrollbar_arrow_pattern = r'style\.configure\("Vertical\.TScrollbar".*arrowsize=(\d+)'
        arrow_match = re.search(scrollbar_arrow_pattern, self.code)
        self.assertTrue(arrow_match, "Scrollbar arrowsize configuration not found")
        
        # Extract the actual arrowsize value
        actual_arrowsize = arrow_match.group(1)
        expected_arrowsize = "16"
        print(f"✓ Scrollbar arrowsize found: {actual_arrowsize} (expected: {expected_arrowsize})")
        self.assertEqual(actual_arrowsize, expected_arrowsize, 
                        f"Scrollbar arrowsize is {actual_arrowsize}, but expected {expected_arrowsize}")
    
    def test_treeview_frame_structure(self):
        """Test that treeview and scrollbar are grouped in a frame"""
        print("\nChecking treeview frame structure...")
        
        # Check for tree_frame creation
        tree_frame_pattern = r'tree_frame\s*=\s*ttk\.Frame\(window\)'
        self.assertTrue(re.search(tree_frame_pattern, self.code),
                       "tree_frame creation not found")
        print("✓ tree_frame creation found")
        
        # Check that results is created inside tree_frame
        results_in_frame_pattern = r'results\s*=\s*ttk\.Treeview\(tree_frame'
        self.assertTrue(re.search(results_in_frame_pattern, self.code),
                       "Treeview not created inside tree_frame")
        print("✓ Treeview created inside tree_frame")
        
        # Check that scrollbar is created inside tree_frame
        scrollbar_in_frame_pattern = r'scrollbar\s*=\s*ttk\.Scrollbar\(tree_frame'
        self.assertTrue(re.search(scrollbar_in_frame_pattern, self.code),
                       "Scrollbar not created inside tree_frame")
        print("✓ Scrollbar created inside tree_frame")
    
    def test_config_usage(self):
        """Test that functions use directories from config.json"""
        print("\nChecking config usage...")
        
        # Check for load_config function
        load_config_pattern = r'def\s+load_config\(\):'
        self.assertTrue(re.search(load_config_pattern, self.code),
                       "load_config function not found")
        print("✓ load_config function found")
        
        # Check for get_search_directory function
        get_search_dir_pattern = r'def\s+get_search_directory\(\):'
        self.assertTrue(re.search(get_search_dir_pattern, self.code),
                       "get_search_directory function not found")
        print("✓ get_search_directory function found")
        
        # Check that editor_path is loaded from config
        editor_path_pattern = r'editor_path\s*=\s*load_config\(\)\[\'editor_path\'\]'
        self.assertTrue(re.search(editor_path_pattern, self.code),
                       "editor_path not loaded from config")
        print("✓ editor_path loaded from config")
    
    def test_context_menu_settings(self):
        """Test that Settings is in the context menu"""
        print("\nChecking context menu settings...")
        
        # Check for Settings in context menu
        settings_pattern = r'context_menu\.add_command\(label="Settings"'
        self.assertTrue(re.search(settings_pattern, self.code),
                       "Settings not found in context menu")
        print("✓ Settings found in context menu")
        
        # Check for separator before Settings
        separator_pattern = r'context_menu\.add_separator\(\).*context_menu\.add_command\(label="Settings"'
        self.assertTrue(re.search(separator_pattern, self.code, re.DOTALL),
                       "Separator before Settings not found")
        print("✓ Separator before Settings found")
    
    def test_treeview_columns(self):
        """Test that treeview has the correct columns"""
        print("\nChecking treeview columns...")
        
        # Check for three columns in treeview
        columns_pattern = r'results\s*=\s*ttk\.Treeview\(.*columns=\("Item",\s*"Type",\s*"Path"\)'
        self.assertTrue(re.search(columns_pattern, self.code, re.DOTALL),
                       "Treeview columns not correctly defined")
        print("✓ Treeview columns correctly defined")
        
        # Check column headings
        headings_pattern = r'results\.heading\("Item",\s*text="Item"\).*' + \
                          r'results\.heading\("Type",\s*text="Type"\).*' + \
                          r'results\.heading\("Path",\s*text="Path"\)'
        self.assertTrue(re.search(headings_pattern, self.code, re.DOTALL),
                       "Treeview headings not correctly defined")
        print("✓ Treeview headings correctly defined")
        
        # Check column widths
        widths_pattern = r'results\.column\("Item",\s*width=100\).*' + \
                        r'results\.column\("Type",\s*width=100\).*' + \
                        r'results\.column\("Path",\s*width=400\)'
        self.assertTrue(re.search(widths_pattern, self.code, re.DOTALL),
                       "Treeview column widths not correctly defined")
        print("✓ Treeview column widths correctly defined")
    
    def test_search_files_function(self):
        """Test that search_files inserts data in the correct columns"""
        print("\nChecking search_files function...")
        
        # Check that search_files inserts values in the correct order
        insert_pattern = r'results\.insert\(\'\'.*values=\[item,\s*item_type,\s*full_path\]\)'
        self.assertTrue(re.search(insert_pattern, self.code, re.DOTALL),
                       "search_files not inserting values in correct order")
        print("✓ search_files inserts values in correct order")

if __name__ == '__main__':
    unittest.main(verbosity=2) 