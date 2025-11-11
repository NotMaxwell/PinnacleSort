use std::fs;
use eframe::egui;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([600.0, 700.0]),
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
                ui.heading(format!("Found {} files to delete", self.scan_results.len()));
                ui.add_space(10.0);
                
                egui::ScrollArea::vertical().max_height(250.0).show(ui, |ui| {
                    for result in &self.scan_results {
                        ui.horizontal(|ui| {
                            ui.label(&result.file_name);
                            ui.label(format!("({} days old)", result.days_since_access));
                        });
                    }
                });
                
                ui.add_space(10.0);
                
                if ui.button("Delete All Listed Files").clicked() {
                    self.delete_files();
                }
            }
        });
    }
}

impl FileCleanerApp {
    fn scan_files(&mut self) {
        self.is_scanning = true;
        self.scan_results.clear();
        self.status_message = "Scanning...".to_string();
        
        let user = whoami::username();
        let (working_directory, separator) = if cfg!(target_os = "windows") {
            (format!("C:\\Users\\{}\\", user), "\\")
        } else {
            (format!("/Users/{}/", user), "/")
        };
        
        // Build list of directories to search
        let mut directories = Vec::new();
        if self.downloads_enabled {
            directories.push(format!("{}Downloads{}", working_directory, separator));
        }
        if self.documents_enabled {
            directories.push(format!("{}Documents{}", working_directory, separator));
        }
        if self.desktop_enabled {
            directories.push(format!("{}Desktop{}", working_directory, separator));
        }
        
        // Add custom directories
        for custom_dir in &self.custom_directories {
            directories.push(custom_dir.clone());
        }
        
        let time_limit = std::time::Duration::from_secs(60 * 60 * 24 * self.time_limit_days);
        
        // Scan each directory
        for directory_path in directories {
            let Ok(entries) = std::fs::read_dir(&directory_path) else {
                continue;
            };
            
            for entry in entries {
                let Ok(entry) = entry else { continue; };
                let file_name = entry.file_name();
                let file_name_str = file_name.to_str().unwrap_or("").to_string();
                let path = entry.path();
                
                // Skip hidden files
                if file_name_str.starts_with('.') {
                    continue;
                }
                
                // Skip directories
                if path.is_dir() {
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
        
        self.status_message = format!("Scan complete. Found {} files.", self.scan_results.len());
        self.is_scanning = false;
    }
    
    fn delete_files(&mut self) {
        let mut deleted_count = 0;
        let mut failed_count = 0;
        
        for result in &self.scan_results {
            if result.should_delete {
                match fs::remove_file(&result.file_path) {
                    Ok(_) => deleted_count += 1,
                    Err(_) => failed_count += 1,
                }
            }
        }
        
        self.status_message = format!(
            "Deleted {} files. {} failed.",
            deleted_count, failed_count
        );
        self.scan_results.clear();
    }
}
