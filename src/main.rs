use std::fs;
use eframe::egui;
use std::collections::HashMap;

fn load_icon() -> egui::IconData {
    // Create a simple 256x256 icon programmatically
    let size = 256;
    let mut pixels = vec![0u8; size * size * 4]; // RGBA
    
    // Fill with blue background
    for y in 0..size {
        for x in 0..size {
            let idx = (y * size + x) * 4;
            let dx = x as f32 - 128.0;
            let dy = y as f32 - 128.0;
            let dist = (dx * dx + dy * dy).sqrt();
            
            // Draw circular background
            if dist < 120.0 {
                pixels[idx] = 74;      // R - Blue background
                pixels[idx + 1] = 144; // G
                pixels[idx + 2] = 226; // B
                pixels[idx + 3] = 255; // A
                
                // Draw folder shape (simplified)
                if y >= 90 && y <= 170 && x >= 60 && x <= 196 {
                    pixels[idx] = 255;     // R - Orange folder
                    pixels[idx + 1] = 165; // G
                    pixels[idx + 2] = 0;   // B
                }
                
                // Draw folder tab
                if y >= 70 && y <= 90 && x >= 110 && x <= 196 {
                    pixels[idx] = 255;     // R - Yellow tab
                    pixels[idx + 1] = 215; // G
                    pixels[idx + 2] = 0;   // B
                }
                
                // Add checkmark (simplified - just draw it as colored pixels)
                if (x >= 75 && x <= 85 && y >= 130 && y <= 140) ||
                   (x >= 85 && x <= 105 && y >= 110 && y <= 130 && 
                    ((x as i32 - 85).abs() + (y as i32 - 120).abs()) < 10) {
                    pixels[idx] = 0;       // R - Green checkmark
                    pixels[idx + 1] = 255; // G
                    pixels[idx + 2] = 0;   // B
                }
            } else {
                pixels[idx + 3] = 0; // Transparent outside circle
            }
        }
    }
    
    egui::IconData {
        rgba: pixels,
        width: size as u32,
        height: size as u32,
    }
}

fn main() -> Result<(), eframe::Error> {
    let icon = load_icon();
    
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([600.0, 700.0])
            .with_icon(std::sync::Arc::new(icon)),
        ..Default::default()
    };
    
    eframe::run_native(
        "PinnacleSort - File Cleaner",
        options,
        Box::new(|_cc| Ok(Box::new(FileCleanerApp::default()))),
    )
}

struct FileCleanerApp {
    time_limit_days: u64,
    downloads_enabled: bool,
    documents_enabled: bool,
    desktop_enabled: bool,
    custom_directories: Vec<String>,
    new_directory: String,
    scan_results: Vec<ScanResult>,
    is_scanning: bool,
    status_message: String,
    smart_filter_enabled: bool,
}

#[derive(Clone)]
struct ScanResult {
    file_path: String,
    file_name: String,
    should_delete: bool,
    days_since_access: u64,
}

impl Default for FileCleanerApp {
    fn default() -> Self {
        Self {
            time_limit_days: 14,
            downloads_enabled: true,
            documents_enabled: true,
            desktop_enabled: true,
            custom_directories: Vec::new(),
            new_directory: String::new(),
            scan_results: Vec::new(),
            is_scanning: false,
            status_message: String::new(),
            smart_filter_enabled: true,
        }
    }
}

impl eframe::App for FileCleanerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("PinnacleSort - File Cleaner");
            ui.add_space(10.0);
            
            // Time limit section
            ui.horizontal(|ui| {
                ui.label("Delete files not accessed in:");
                ui.add(egui::Slider::new(&mut self.time_limit_days, 1..=365).suffix(" days"));
            });
            ui.add_space(10.0);
            
            // Directory selection
            ui.heading("Directories to Search:");
            ui.checkbox(&mut self.downloads_enabled, "Downloads");
            ui.checkbox(&mut self.documents_enabled, "Documents");
            ui.checkbox(&mut self.desktop_enabled, "Desktop");
            ui.add_space(10.0);
            
            // Smart filter option
            ui.checkbox(&mut self.smart_filter_enabled, "Smart Filter (exclude binary/system files)");
            ui.add_space(10.0);
            
            // Custom directories
            ui.heading("Custom Directories:");
            ui.horizontal(|ui| {
                ui.label("Add directory:");
                ui.text_edit_singleline(&mut self.new_directory);
                if ui.button("Add").clicked() && !self.new_directory.is_empty() {
                    self.custom_directories.push(self.new_directory.clone());
                    self.new_directory.clear();
                }
            });
            
            // Display custom directories
            ui.add_space(5.0);
            let mut to_remove = None;
            for (idx, dir) in self.custom_directories.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(dir);
                    if ui.button("Remove").clicked() {
                        to_remove = Some(idx);
                    }
                });
            }
            if let Some(idx) = to_remove {
                self.custom_directories.remove(idx);
            }
            
            ui.add_space(10.0);
            ui.separator();
            ui.add_space(10.0);
            
            // Scan button
            if ui.button("Scan for Old Files").clicked() && !self.is_scanning {
                self.scan_files();
            }
            
            ui.add_space(10.0);
            
            // Status message
            if !self.status_message.is_empty() {
                ui.label(&self.status_message);
                ui.add_space(10.0);
            }
            
            // Results section
            if !self.scan_results.is_empty() {
                ui.separator();
                
                let selected_count = self.scan_results.iter().filter(|r| r.should_delete).count();
                ui.heading(format!("Found {} files ({} selected to move to trash)", 
                    self.scan_results.len(), selected_count));
                
                ui.horizontal(|ui| {
                    if ui.button("Select All").clicked() {
                        for result in &mut self.scan_results {
                            result.should_delete = true;
                        }
                    }
                    if ui.button("Deselect All").clicked() {
                        for result in &mut self.scan_results {
                            result.should_delete = false;
                        }
                    }
                });
                
                ui.add_space(10.0);
                
                // Calculate available height for scroll area
                let available_height = ui.available_height() - 80.0; // Reserve space for button at bottom
                
                egui::ScrollArea::vertical()
                    .max_height(available_height)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        self.render_directory_tree(ui, 0);
                    });
                
                ui.add_space(10.0);
                
                if selected_count > 0 {
                    if ui.button(format!("Move {} Selected Files to Trash", selected_count)).clicked() {
                        self.delete_files();
                    }
                } else {
                    ui.label("No files selected");
                }
            }
        });
    }
}

impl FileCleanerApp {
    fn render_directory_tree(&mut self, ui: &mut egui::Ui, _depth: usize) {
        // Build a tree structure mapping paths to their children
        let mut tree: HashMap<String, Vec<String>> = HashMap::new();
        let mut file_map: HashMap<String, Vec<usize>> = HashMap::new();
        
        for (idx, result) in self.scan_results.iter().enumerate() {
            let path = std::path::Path::new(&result.file_path);
            let dir = path.parent()
                .and_then(|p| p.to_str())
                .unwrap_or("")
                .to_string();
            
            file_map.entry(dir.clone()).or_insert_with(Vec::new).push(idx);
            
            // Build parent-child relationships
            if !dir.is_empty() {
                let parts: Vec<&str> = dir.split('/').filter(|s| !s.is_empty()).collect();
                
                // Add all parent paths
                for i in 0..parts.len() {
                    let current_path = "/".to_string() + &parts[0..=i].join("/");
                    
                    if i > 0 {
                        let parent_path = "/".to_string() + &parts[0..i].join("/");
                        tree.entry(parent_path)
                            .or_insert_with(Vec::new)
                            .push(current_path.clone());
                    }
                }
            }
        }
        
        // Deduplicate children
        for children in tree.values_mut() {
            children.sort();
            children.dedup();
        }
        
        // Find root directories (those that appear in file_map but have a common ancestor)
        let user = whoami::username();
        let user_home = format!("/Users/{}", user);
        
        let mut roots: Vec<String> = file_map.keys()
            .filter_map(|path| {
                if path.starts_with(&user_home) {
                    let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
                    if parts.len() >= 3 {
                        Some(format!("/{}/{}/{}", parts[0], parts[1], parts[2]))
                    } else {
                        None
                    }
                } else {
                    Some(path.clone())
                }
            })
            .collect();
        
        roots.sort();
        roots.dedup();
        
        for root in roots {
            self.render_tree_node(ui, &root, &tree, &file_map, 0);
        }
    }
    
    fn render_tree_node(
        &mut self,
        ui: &mut egui::Ui,
        path: &str,
        tree: &HashMap<String, Vec<String>>,
        file_map: &HashMap<String, Vec<usize>>,
        depth: usize,
    ) {
        let indent = depth as f32 * 20.0;
        
        // Get folder name from path
        let folder_name = std::path::Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(path);
        
        // Count files in this directory and all subdirectories
        let (total_files, selected_files) = self.count_files_recursive(path, tree, file_map);
        
        if total_files > 0 {
            ui.add_space(5.0);
            
            // Determine icon and color based on selection state
            let icon = if depth == 0 { "📁" } else { "📂" };
            let selection_status = if selected_files == total_files {
                "✅" // All selected
            } else if selected_files > 0 {
                "⚠️" // Partially selected
            } else {
                "⬜" // None selected
            };
            
            let header_text = format!("{} {} {} ({} of {} selected)", 
                selection_status, icon, folder_name, selected_files, total_files);
            
            // Use a stable ID for the collapsing header to maintain state
            egui::CollapsingHeader::new(header_text)
                .id_salt(path)
                .default_open(false)
                .show(ui, |ui| {
                    ui.add_space(indent);
                    
                    // Add select/deselect buttons with color
                    ui.horizontal(|ui| {
                        ui.add_space(indent);
                        
                        let select_btn = egui::Button::new(
                            egui::RichText::new("✓ Select All").color(egui::Color32::WHITE)
                        ).fill(egui::Color32::from_rgb(220, 80, 80));
                        if ui.add(select_btn).clicked() {
                            self.select_all_recursive(path, tree, file_map, true);
                        }
                        
                        let deselect_btn = egui::Button::new(
                            egui::RichText::new("✗ Deselect All").color(egui::Color32::WHITE)
                        ).fill(egui::Color32::from_rgb(80, 180, 80));
                        if ui.add(deselect_btn).clicked() {
                            self.select_all_recursive(path, tree, file_map, false);
                        }
                    });
                    
                    // Render child directories
                    if let Some(children) = tree.get(path) {
                        for child in children {
                            self.render_tree_node(ui, child, tree, file_map, depth + 1);
                        }
                    }
                    
                    // Render files in this directory
                    if let Some(indices) = file_map.get(path) {
                        ui.add_space(5.0);
                        for &idx in indices {
                            let result = &mut self.scan_results[idx];
                            
                            // Color code the row based on selection
                            let bg_color = if result.should_delete {
                                egui::Color32::from_rgba_unmultiplied(255, 200, 200, 40) // Light red tint
                            } else {
                                egui::Color32::from_rgba_unmultiplied(200, 255, 200, 20) // Light green tint
                            };
                            
                            let frame = egui::Frame::none()
                                .fill(bg_color)
                                .inner_margin(egui::Margin::same(4.0))
                                .rounding(egui::Rounding::same(3.0));
                            
                            frame.show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.add_space(indent + 20.0);
                                    ui.checkbox(&mut result.should_delete, "");
                                    
                                    let file_icon = if result.should_delete { "🗑️" } else { "📄" };
                                    ui.label(file_icon);
                                    
                                    let text_color = if result.should_delete {
                                        egui::Color32::from_rgb(180, 50, 50)
                                    } else {
                                        egui::Color32::from_rgb(50, 120, 50)
                                    };
                                    
                                    ui.colored_label(text_color, &result.file_name);
                                    ui.label(format!("({} days)", result.days_since_access));
                                });
                            });
                        }
                    }
                });
        }
    }
    
    fn count_files_recursive(
        &self,
        path: &str,
        tree: &HashMap<String, Vec<String>>,
        file_map: &HashMap<String, Vec<usize>>,
    ) -> (usize, usize) {
        let mut total = 0;
        let mut selected = 0;
        
        // Count files in this directory
        if let Some(indices) = file_map.get(path) {
            total += indices.len();
            selected += indices.iter()
                .filter(|&&idx| self.scan_results[idx].should_delete)
                .count();
        }
        
        // Count files in subdirectories
        if let Some(children) = tree.get(path) {
            for child in children {
                let (child_total, child_selected) = self.count_files_recursive(child, tree, file_map);
                total += child_total;
                selected += child_selected;
            }
        }
        
        (total, selected)
    }
    
    fn select_all_recursive(
        &mut self,
        path: &str,
        tree: &HashMap<String, Vec<String>>,
        file_map: &HashMap<String, Vec<usize>>,
        select: bool,
    ) {
        // Select/deselect files in this directory
        if let Some(indices) = file_map.get(path) {
            for &idx in indices {
                self.scan_results[idx].should_delete = select;
            }
        }
        
        // Recursively select/deselect in subdirectories
        if let Some(children) = tree.get(path) {
            for child in children {
                self.select_all_recursive(child, tree, file_map, select);
            }
        }
    }
    
    fn should_exclude_file(&self, file_name: &str) -> bool {
        if !self.smart_filter_enabled {
            return false;
        }
        
        let file_lower = file_name.to_lowercase();
        
        // Binary and supporting files (excluding .exe which we want to check)
        let binary_extensions = [
            ".dll", ".so", ".dylib", ".bin", ".o", ".a", 
            ".lib", ".sys", ".drv", ".class", ".pyc", ".pyo",
        ];
        
        // System and cache files
        let system_patterns = [
            ".cache", ".tmp", ".temp", ".log", ".bak", ".swp", ".swo",
            ".lock", ".pid", ".dat", ".db", ".sqlite", ".idx",
        ];
        
        // Build and dependency directories content
        let build_patterns = [
            "node_modules", "target", "build", "dist", ".git", ".svn",
        ];
        
        // Check extensions
        for ext in &binary_extensions {
            if file_lower.ends_with(ext) {
                return true;
            }
        }
        
        // Check system patterns
        for pattern in &system_patterns {
            if file_lower.contains(pattern) {
                return true;
            }
        }
        
        // Check if file is in a build/dependency directory
        for pattern in &build_patterns {
            if file_lower.contains(pattern) {
                return true;
            }
        }
        
        false
    }
    
    fn get_exe_base_name(path: &str) -> Option<String> {
        if path.to_lowercase().ends_with(".exe") {
            let file_name = std::path::Path::new(path)
                .file_stem()?
                .to_str()?;
            Some(file_name.to_string())
        } else {
            None
        }
    }
    
    fn find_associated_files(&self, exe_path: &str) -> Vec<String> {
        let mut associated_files = Vec::new();
        
        let Some(base_name) = Self::get_exe_base_name(exe_path) else {
            return associated_files;
        };
        
        let exe_dir = std::path::Path::new(exe_path).parent();
        let Some(dir) = exe_dir else {
            return associated_files;
        };
        
        let Ok(entries) = std::fs::read_dir(dir) else {
            return associated_files;
        };
        
        // Supporting file extensions that should be deleted with .exe
        let supporting_extensions = [".dll", ".dat", ".ini", ".cfg", ".config"];
        
        for entry in entries {
            let Ok(entry) = entry else { continue; };
            let path = entry.path();
            let file_name = entry.file_name();
            let file_name_str = file_name.to_str().unwrap_or("");
            
            // Skip the .exe itself
            if path.to_string_lossy() == exe_path {
                continue;
            }
            
            // Check if file name starts with the base name and has a supporting extension
            let file_lower = file_name_str.to_lowercase();
            let base_lower = base_name.to_lowercase();
            
            if file_lower.starts_with(&base_lower) {
                for ext in &supporting_extensions {
                    if file_lower.ends_with(ext) {
                        associated_files.push(path.to_string_lossy().to_string());
                        break;
                    }
                }
            }
        }
        
        associated_files
    }
    
    fn scan_files(&mut self) {
        self.is_scanning = true;
        self.scan_results.clear();
        self.status_message = "Scanning...".to_string();
        
        let user = whoami::username();
        let working_directory = if cfg!(target_os = "windows") {
            format!("C:\\Users\\{}\\", user)
        } else {
            format!("/Users/{}/", user)
        };
        
        // Build list of directories to search
        let mut directories = Vec::new();
        if self.downloads_enabled {
            directories.push(format!("{}Downloads", working_directory));
        }
        if self.documents_enabled {
            directories.push(format!("{}Documents", working_directory));
        }
        if self.desktop_enabled {
            directories.push(format!("{}Desktop", working_directory));
        }
        
        // Add custom directories
        for custom_dir in &self.custom_directories {
            directories.push(custom_dir.clone());
        }
        
        let time_limit = std::time::Duration::from_secs(60 * 60 * 24 * self.time_limit_days);
        
        // Scan each directory recursively
        for directory_path in directories {
            self.scan_directory_recursive(&directory_path, time_limit);
        }
        
        self.status_message = format!("Scan complete. Found {} files.", self.scan_results.len());
        self.is_scanning = false;
    }
    
    fn scan_directory_recursive(&mut self, directory_path: &str, time_limit: std::time::Duration) {
        let Ok(entries) = std::fs::read_dir(directory_path) else {
            return;
        };
        
        for entry in entries {
            let Ok(entry) = entry else { continue; };
            let file_name = entry.file_name();
            let file_name_str = file_name.to_str().unwrap_or("").to_string();
            let path = entry.path();
            
            // Skip hidden files and directories
            if file_name_str.starts_with('.') {
                continue;
            }
            
            // If it's a directory, recurse into it
            if path.is_dir() {
                self.scan_directory_recursive(&path.to_string_lossy(), time_limit);
                continue;
            }
            
            // Apply smart filter to exclude binary/system files
            if self.should_exclude_file(&file_name_str) {
                continue;
            }
            
            // Get metadata and accessed time
            let Ok(metadata) = fs::metadata(&path) else {
                continue;
            };
            
            let Ok(accessed) = metadata.accessed() else {
                continue;
            };
            
            let recently_accessed = accessed >= std::time::SystemTime::now() - time_limit;
            
            if !recently_accessed {
                // Calculate days since access
                let duration = std::time::SystemTime::now()
                    .duration_since(accessed)
                    .unwrap_or_default();
                let days_since_access = duration.as_secs() / (60 * 60 * 24);
                
                self.scan_results.push(ScanResult {
                    file_path: path.to_string_lossy().to_string(),
                    file_name: file_name_str,
                    should_delete: true,
                    days_since_access,
                });
            }
        }
    }
    
    fn get_trash_directory() -> Result<String, String> {
        let user = whoami::username();
        let trash_dir = if cfg!(target_os = "windows") {
            format!("C:\\Users\\{}\\Desktop\\PinnacleSort_Trash\\", user)
        } else {
            format!("/Users/{}/Desktop/PinnacleSort_Trash/", user)
        };
        
        // Create trash directory if it doesn't exist
        if let Err(e) = fs::create_dir_all(&trash_dir) {
            return Err(format!("Failed to create trash directory: {}", e));
        }
        
        Ok(trash_dir)
    }
    
    fn move_to_trash(&self, file_path: &str, trash_dir: &str) -> Result<(), String> {
        let source_path = std::path::Path::new(file_path);
        let file_name = source_path
            .file_name()
            .ok_or("Invalid file name")?
            .to_str()
            .ok_or("Invalid UTF-8 in file name")?;
        
        let mut dest_path = std::path::PathBuf::from(trash_dir);
        dest_path.push(file_name);
        
        // If file already exists in trash, add a timestamp
        if dest_path.exists() {
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            let stem = source_path.file_stem().unwrap().to_str().unwrap();
            let ext = source_path.extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");
            let new_name = if ext.is_empty() {
                format!("{}_{}", stem, timestamp)
            } else {
                format!("{}_{}.{}", stem, timestamp, ext)
            };
            dest_path = std::path::PathBuf::from(trash_dir);
            dest_path.push(new_name);
        }
        
        fs::rename(file_path, &dest_path)
            .map_err(|e| format!("Failed to move file: {}", e))?;
        
        Ok(())
    }
    
    fn delete_files(&mut self) {
        let mut moved_count = 0;
        let mut failed_count = 0;
        let mut associated_moved = 0;
        
        let trash_dir = match Self::get_trash_directory() {
            Ok(dir) => dir,
            Err(e) => {
                self.status_message = format!("Error: {}", e);
                return;
            }
        };
        
        for result in &self.scan_results {
            if result.should_delete {
                // If it's an .exe file, find and move associated files first
                if result.file_path.to_lowercase().ends_with(".exe") {
                    let associated_files = self.find_associated_files(&result.file_path);
                    for assoc_file in associated_files {
                        if self.move_to_trash(&assoc_file, &trash_dir).is_ok() {
                            associated_moved += 1;
                        }
                    }
                }
                
                // Move the main file
                match self.move_to_trash(&result.file_path, &trash_dir) {
                    Ok(_) => moved_count += 1,
                    Err(_) => failed_count += 1,
                }
            }
        }
        
        let message = if associated_moved > 0 {
            format!(
                "Moved {} files ({} associated files) to trash. {} failed. Location: {}",
                moved_count, associated_moved, failed_count, trash_dir
            )
        } else {
            format!(
                "Moved {} files to trash. {} failed. Location: {}",
                moved_count, failed_count, trash_dir
            )
        };
        
        self.status_message = message;
        self.scan_results.clear();
    }
}
