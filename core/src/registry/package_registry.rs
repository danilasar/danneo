use crate::registry::manifest::{BlockManifest, PackageManifest};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{error, info};

pub struct PackageRegistry {
    pub packages_dir: PathBuf,
    pub staging_dir: PathBuf,
    pub packages: HashMap<String, PackageManifest>,
    pub blocks: HashMap<String, BlockManifest>,
}

impl PackageRegistry {
    pub fn new(packages_dir: impl Into<PathBuf>) -> Self {
        let pkg_dir = packages_dir.into();
        let staging_dir = pkg_dir.join("staging");
        Self {
            packages_dir: pkg_dir,
            staging_dir,
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
                                    let module_id = manifest.package.id.clone();
                                    self.packages.insert(module_id.clone(), manifest);

                                    // Scan for blocks within this module
                                    let blocks_path = path.join("blocks");
                                    if blocks_path.exists() {
                                        if let Ok(block_entries) = std::fs::read_dir(&blocks_path) {
                                            for b_entry in block_entries.flatten() {
                                                let b_path = b_entry.path();
                                                if b_path.is_dir() {
                                                    let b_manifest_path = b_path.join("block.toml");
                                                    if b_manifest_path.exists() {
                                                        match self
                                                            .load_block_manifest(&b_manifest_path)
                                                        {
                                                            Ok(mut b_manifest) => {
                                                                // Force the module_code to match the parent module
                                                                b_manifest.block.module_code =
                                                                    module_id.clone();
                                                                info!(
                                                                    "Loaded block manifest: {} (from {})",
                                                                    b_manifest.block.id, module_id
                                                                );
                                                                self.blocks.insert(
                                                                    b_manifest.block.id.clone(),
                                                                    b_manifest,
                                                                );
                                                            }
                                                            Err(e) => error!(
                                                                "Failed to load block manifest from {}: {}",
                                                                b_manifest_path.display(),
                                                                e
                                                            ),
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                Err(e) => error!(
                                    "Failed to load package manifest from {}: {}",
                                    manifest_path.display(),
                                    e
                                ),
                            }
                        }
                    }
                }
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
