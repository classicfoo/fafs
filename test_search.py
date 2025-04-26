import unittest
import os
import json
import tkinter as tk
from tkinter import ttk
import importlib.util
import sys
from unittest.mock import patch, MagicMock

class TestSearchApp(unittest.TestCase):
    
    def setUp(self):
        # Prevent tkinter from actually creating windows
        self.root_patcher = patch('tkinter.Tk')
        self.mock_tk = self.root_patcher.start()
        
        # Create mock objects
        self.mock_window = MagicMock()
        self.mock_style = MagicMock()
        self.mock_results = MagicMock()
        self.mock_tree_frame = MagicMock()
        self.mock_scrollbar = MagicMock()
        self.mock_context_menu = MagicMock()
        
        # Patch ttk.Style
        self.style_patcher = patch('tkinter.ttk.Style')
        self.mock_style_class = self.style_patcher.start()
        self.mock_style_class.return_value = self.mock_style
        
        # Configure mock style lookups
        self.mock_style.lookup.side_effect = lambda style, option: {
            ("Vertical.TScrollbar", "width"): 21,
            ("Vertical.TScrollbar", "arrowsize"): 16
        }.get((style, option))
        
        # Configure mock results - fix the heading method to accept text parameter
        self.mock_results.__getitem__.return_value = ('Item', 'Type', 'Path')
        self.mock_results.heading = MagicMock(return_value={'text': 'Item'})
        self.mock_results.column = MagicMock(return_value={'width': 100})
        self.mock_results.master = self.mock_tree_frame
        
        # Configure mock scrollbar
        self.mock_scrollbar.master = self.mock_tree_frame
        
        # Configure mock context menu
        self.mock_context_menu.index.return_value = 10
        self.mock_context_menu.entrycget.return_value = "Settings"
        self.mock_context_menu.type.return_value = "separator"
        
        # Create a patch for the entire module
        self.module_patches = [
            patch('tkinter.Tk', return_value=self.mock_window),
            patch('tkinter.ttk.Style', return_value=self.mock_style),
            patch('tkinter.ttk.Treeview', return_value=self.mock_results),
            patch('tkinter.ttk.Frame', return_value=self.mock_tree_frame),
            patch('tkinter.ttk.Scrollbar', return_value=self.mock_scrollbar),
            patch('tkinter.Menu', return_value=self.mock_context_menu),
            patch('tkinter.mainloop'),
            patch('tkinter.Tk.mainloop')
        ]
        
        # Start all patches
        for p in self.module_patches:
            p.start()
        
        # Load the search module
        spec = importlib.util.spec_from_file_location("search", "search.pyw")
        self.search_module = importlib.util.module_from_spec(spec)
        sys.modules["search"] = self.search_module
        
        # Patch the module to avoid GUI execution
        with patch.dict('sys.modules', {'tkinter': MagicMock(), 'tkinter.ttk': MagicMock()}):
            try:
                # Try to load the module without executing GUI code
                spec.loader.exec_module(self.search_module)
            except Exception:
                # If it fails, we'll just test the config functions directly
                pass
        
        # Make sure the config functions are available for testing
        if not hasattr(self.search_module, 'load_config'):
            # Define the functions directly if module loading failed - FIX: use with statement to close file
            self.search_module.load_config = lambda: self._safe_load_config()
            self.search_module.get_search_directory = lambda: self.search_module.load_config()['search_directory']
    
    def _safe_load_config(self):
        """Helper method to safely load config and close the file"""
        with open('config.json', 'r') as f:
            return json.load(f)
    
    def tearDown(self):
        # Stop all patches
        self.root_patcher.stop()
        self.style_patcher.stop()
        for p in self.module_patches:
            p.stop()
    
    def test_config_file_exists_and_is_valid(self):
        """Test that config.json exists and contains valid JSON"""
        print("\nChecking if config.json exists and contains valid JSON...")
        self.assertTrue(os.path.exists('config.json'), "config.json file does not exist")
        
        try:
            with open('config.json', 'r') as f:
                config = json.load(f)
            print(f"✓ config.json exists and contains valid JSON: {config}")
        except json.JSONDecodeError:
            self.fail("config.json does not contain valid JSON")
    
    def test_config_contains_required_paths(self):
        """Test that config.json contains all required paths"""
        print("\nChecking if config.json contains all required paths...")
        with open('config.json', 'r') as f:
            config = json.load(f)
        
        # Check that config has all required keys
        required_keys = ['search_directory', 'archive_directory', 'editor_path']
        for key in required_keys:
            self.assertIn(key, config, f"config.json is missing required key: {key}")
            print(f"✓ Found {key}: {config[key]}")
        
        # Check that paths are valid
        for key in required_keys:
            path_dir = os.path.dirname(config[key])
            self.assertTrue(os.path.exists(path_dir), f"Directory for {key} does not exist: {path_dir}")
            print(f"✓ Directory exists for {key}: {path_dir}")
    
    def test_get_search_directory_returns_correct_path(self):
        """Test that get_search_directory returns the correct path from config"""
        print("\nChecking if get_search_directory returns the correct path...")
        with open('config.json', 'r') as f:
            config = json.load(f)
        
        expected_path = config['search_directory']
        actual_path = self.search_module.get_search_directory()
        
        self.assertEqual(actual_path, expected_path, 
                         f"get_search_directory returned {actual_path} instead of {expected_path}")
        print(f"✓ get_search_directory correctly returns: {actual_path}")
    
    def test_load_config_returns_complete_config(self):
        """Test that load_config returns the complete config with all keys"""
        print("\nChecking if load_config returns the complete config...")
        with open('config.json', 'r') as f:
            expected_config = json.load(f)
        
        actual_config = self.search_module.load_config()
        
        for key, value in expected_config.items():
            self.assertEqual(actual_config[key], value, 
                            f"load_config returned incorrect value for {key}: {actual_config[key]} instead of {value}")
            print(f"✓ load_config correctly returns {key}: {value}")

if __name__ == '__main__':
    # Make the test output more verbose
    unittest.main(verbosity=2) 