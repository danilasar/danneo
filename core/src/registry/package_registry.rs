use async_trait::async_trait;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};

pub use danneo_sdk::registry::{BlockManifest, PackageManifest, VerificationResult};

pub struct PackageRegistry {
    pub packages_dir: PathBuf,
    pub staging_dir: PathBuf,
    pub packages: Arc<RwLock<HashMap<String, PackageManifest>>>,
    pub blocks: Arc<RwLock<HashMap<String, BlockManifest>>>,
}

#[async_trait]
impl danneo_sdk::registry::IPackageRegistry for PackageRegistry {
    async fn get_packages(&self) -> HashMap<String, PackageManifest> {
        self.packages.read().await.clone()
    }

    async fn get_blocks(&self) -> HashMap<String, BlockManifest> {
        self.blocks.read().await.clone()
    }

    fn get_packages_dir(&self) -> PathBuf {
        self.packages_dir.clone()
    }

    async fn scan(&self) {
        self.scan_internal().await;
    }

    async fn extract_and_verify(
        &self,
        zip_bytes: &[u8],
        installed_versions: &HashMap<String, String>,
    ) -> Result<VerificationResult, String> {
        self.extract_and_verify_internal(zip_bytes, installed_versions)
            .await
    }
}

impl PackageRegistry {
    pub fn new(packages_dir: impl Into<PathBuf>) -> Self {
        let pkg_dir = packages_dir.into();
        let staging_dir = pkg_dir.join("staging");
        Self {
            packages_dir: pkg_dir,
            staging_dir,
            packages: Arc::new(RwLock::new(HashMap::new())),
            blocks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn scan_internal(&self) {
        {
            let mut p = self.packages.write().await;
            p.clear();
            let mut b = self.blocks.write().await;
            b.clear();
        }

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
                                    self.packages
                                        .write()
                                        .await
                                        .insert(module_id.clone(), manifest);

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
                                                                self.blocks.write().await.insert(
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

    pub async fn extract_and_verify_internal(
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
        let staging_path = self.staging_dir.join(&temp_id);
        std::fs::create_dir_all(&staging_path).map_err(|e| e.to_string())?;

        archive.extract(&staging_path).map_err(|e| e.to_string())?;

        let manifest_path = staging_path.join("module.toml");
        if !manifest_path.exists() {
            std::fs::remove_dir_all(&staging_path).ok();
            return Err("module.toml not found in package".to_string());
        }

        let manifest = self
            .load_package_manifest(&manifest_path)
            .map_err(|e| e.to_string())?;
        let module_id = &manifest.package.id;
        let current_version = installed_versions.get(module_id).cloned();
        let is_upgrade = current_version.is_some();

        Ok(VerificationResult {
            manifest,
            staging_path,
            is_upgrade,
            current_version,
            issues: vec![],
        })
    }

    fn load_package_manifest(&self, path: &Path) -> Result<PackageManifest, String> {
        let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        let manifest: PackageManifest = toml::from_str(&content).map_err(|e| e.to_string())?;
        Ok(manifest)
    }

    fn load_block_manifest(&self, path: &Path) -> Result<BlockManifest, String> {
        let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        let manifest: BlockManifest = toml::from_str(&content).map_err(|e| e.to_string())?;
        Ok(manifest)
    }
}
