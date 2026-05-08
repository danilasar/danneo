use crate::registry::manifest::{BlockManifest, PackageManifest};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{error, info, warn};

pub struct PackageRegistry {
    pub packages_dir: PathBuf,
    pub blocks_dir: PathBuf,
    pub staging_dir: PathBuf,
    pub packages: HashMap<String, PackageManifest>,
    pub blocks: HashMap<String, BlockManifest>,
}

impl PackageRegistry {
    pub fn new(packages_dir: impl Into<PathBuf>, blocks_dir: impl Into<PathBuf>) -> Self {
        let pkg_dir = packages_dir.into();
        let staging_dir = pkg_dir.join("staging");
        Self {
            packages_dir: pkg_dir,
            blocks_dir: blocks_dir.into(),
            staging_dir,
            packages: HashMap::new(),
            blocks: HashMap::new(),
        }
    }

    pub fn scan(&mut self) {
        // ... existing scan logic (to be updated later if needed)
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
}

#[derive(serde::Serialize)]
pub struct VerificationResult {
    pub manifest: PackageManifest,
    pub staging_path: PathBuf,
    pub is_upgrade: bool,
    pub current_version: Option<String>,
    pub issues: Vec<String>,
}

impl PackageRegistry {
    pub fn extract_and_verify(
        &self,
        zip_bytes: &[u8],
        installed_versions: &HashMap<String, String>,
    ) -> Result<VerificationResult, String> {
        use zip::ZipArchive;

        if !self.staging_dir.exists() {
            std::fs::create_dir_all(&self.staging_dir).map_err(|e| e.to_string())?;
        }

        let reader = std::io::Cursor::new(zip_bytes);
        let mut archive = ZipArchive::new(reader).map_err(|e| e.to_string())?;

        let temp_id = uuid::Uuid::new_v4().to_string();
        let temp_path = self.staging_dir.join(&temp_id);
        std::fs::create_dir_all(&temp_path).map_err(|e| e.to_string())?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i).map_err(|e| e.to_string())?;
            let outpath = match file.enclosed_name() {
                Some(path) => temp_path.join(path),
                None => continue,
            };

            if (*file.name()).ends_with('/') {
                std::fs::create_dir_all(&outpath).map_err(|e| e.to_string())?;
            } else {
                if let Some(p) = outpath.parent() {
                    if !p.exists() {
                        std::fs::create_dir_all(p).map_err(|e| e.to_string())?;
                    }
                }
                let mut outfile = std::fs::File::create(&outpath).map_err(|e| e.to_string())?;
                std::io::copy(&mut file, &mut outfile).map_err(|e| e.to_string())?;
            }
        }

        let manifest_path = temp_path.join("module.toml");
        if !manifest_path.exists() {
            let _ = std::fs::remove_dir_all(&temp_path);
            return Err("Missing module.toml in package".to_string());
        }

        let manifest = self.load_package_manifest(&manifest_path).map_err(|e| {
            let _ = std::fs::remove_dir_all(&temp_path);
            format!("Invalid manifest: {}", e)
        })?;

        let id = &manifest.package.id;
        let current_version = installed_versions.get(id).cloned();
        let is_upgrade = current_version.is_some();

        let mut issues = Vec::new();
        if let Some(deps) = &manifest.dependencies {
            for (dep_id, version_req) in deps {
                if let Some(installed_ver) = installed_versions.get(dep_id) {
                    // Simple version check (equality or placeholder)
                    if version_req != "*" && version_req != installed_ver {
                        issues.push(format!(
                            "Зависимость '{}' имеет версию {}, требуется {}",
                            dep_id, installed_ver, version_req
                        ));
                    }
                } else {
                    issues.push(format!(
                        "Отсутствует необходимая зависимость: '{}' (требуется {})",
                        dep_id, version_req
                    ));
                }
            }
        }

        Ok(VerificationResult {
            manifest,
            staging_path: temp_path,
            is_upgrade,
            current_version,
            issues,
        })
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
