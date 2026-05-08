use crate::registry::manifest::{BlockManifest, PackageManifest};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{error, info, warn};

pub struct PackageRegistry {
    pub packages_dir: PathBuf,
    pub blocks_dir: PathBuf,
    pub packages: HashMap<String, PackageManifest>,
    pub blocks: HashMap<String, BlockManifest>,
}

impl PackageRegistry {
    pub fn new(packages_dir: impl Into<PathBuf>, blocks_dir: impl Into<PathBuf>) -> Self {
        Self {
            packages_dir: packages_dir.into(),
            blocks_dir: blocks_dir.into(),
            packages: HashMap::new(),
            blocks: HashMap::new(),
        }
    }

    pub fn scan(&mut self) {
        self.packages.clear();
        self.blocks.clear();

        // Scan modules
        if self.packages_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&self.packages_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        let manifest_path = path.join("module.toml");
                        if manifest_path.exists() {
                            match self.load_package_manifest(&manifest_path) {
                                Ok(manifest) => {
                                    info!("Loaded package manifest: {}", manifest.package.id);
                                    self.packages.insert(manifest.package.id.clone(), manifest);
                                }
                                Err(e) => {
                                    error!(
                                        "Failed to load package manifest from {}: {}",
                                        manifest_path.display(),
                                        e
                                    );
                                }
                            }
                        }
                    }
                }
            }
        } else {
            warn!(
                "Packages directory not found: {}",
                self.packages_dir.display()
            );
            if let Err(e) = std::fs::create_dir_all(&self.packages_dir) {
                error!("Failed to create packages directory: {}", e);
            }
        }

        // Scan blocks
        if self.blocks_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&self.blocks_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        let manifest_path = path.join("block.toml");
                        if manifest_path.exists() {
                            match self.load_block_manifest(&manifest_path) {
                                Ok(manifest) => {
                                    info!("Loaded block manifest: {}", manifest.block.id);
                                    self.blocks.insert(manifest.block.id.clone(), manifest);
                                }
                                Err(e) => {
                                    error!(
                                        "Failed to load block manifest from {}: {}",
                                        manifest_path.display(),
                                        e
                                    );
                                }
                            }
                        }
                    }
                }
            }
        } else {
            warn!("Blocks directory not found: {}", self.blocks_dir.display());
            if let Err(e) = std::fs::create_dir_all(&self.blocks_dir) {
                error!("Failed to create blocks directory: {}", e);
            }
        }
    }

    fn load_package_manifest(
        &self,
        path: &Path,
    ) -> Result<PackageManifest, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let manifest: PackageManifest = toml::from_str(&content)?;
        Ok(manifest)
    }

    fn load_block_manifest(
        &self,
        path: &Path,
    ) -> Result<BlockManifest, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let manifest: BlockManifest = toml::from_str(&content)?;
        Ok(manifest)
    }
}
