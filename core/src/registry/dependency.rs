use crate::registry::PackageManifest;
use pubgrub::{
    Range,
    SelectedDependencies,
    DependencyProvider,
    resolve,
    SemanticVersion,
    PubGrubError,
    PackageResolutionStatistics,
    DependencyConstraints,
};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DependencyProviderError {
    #[error("Package {0} not found")]
    PackageNotFound(String),
    #[error("Version {1} not found for package {0}")]
    VersionNotFound(String, SemanticVersion),
    #[error("Dependency resolution failed")]
    ResolutionFailed,
}

/// Провайдер зависимостей для NeoDanneo
pub struct ModuleDependencyProvider {
    pub available_packages: HashMap<String, Vec<(SemanticVersion, PackageManifest)>>,
}

impl DependencyProvider for ModuleDependencyProvider {
    type P = String;
    type V = SemanticVersion;
    type VS = Range<Self::V>;
    type M = String; // Metadata
    type Priority = u32;
    type Err = DependencyProviderError;

    fn get_dependencies(
        &self,
        package: &Self::P,
        version: &Self::V,
    ) -> Result<pubgrub::Dependencies<Self::P, Self::VS, Self::M>, Self::Err> {
        let packages = self.available_packages.get(package)
            .ok_or_else(|| DependencyProviderError::PackageNotFound(package.clone()))?;
        
        let manifest = packages.iter()
            .find(|(v, _)| v == version)
            .map(|(_, m)| m)
            .ok_or_else(|| DependencyProviderError::VersionNotFound(package.clone(), *version))?;

        let mut deps = Vec::new();
        if let Some(mandatory) = &manifest.dependencies {
            for (dep_id, req_str) in mandatory {
                let range = if req_str.starts_with(">=") {
                    let ver_part = &req_str[2..];
                    if let Ok(v) = parse_semantic_version(ver_part) {
                        Range::higher_than(v)
                    } else {
                        Range::full()
                    }
                } else {
                    Range::full()
                };
                deps.push((dep_id.clone(), range));
            }
        }

        Ok(pubgrub::Dependencies::Available(DependencyConstraints::from_iter(deps)))
    }

    fn choose_version(&self, package: &Self::P, range: &Self::VS) -> Result<Option<Self::V>, Self::Err> {
        let packages = self.available_packages.get(package);
        if let Some(versions) = packages {
            let mut matched_versions: Vec<_> = versions.iter()
                .filter(|(v, _)| range.contains(v))
                .map(|(v, _)| *v)
                .collect();
            matched_versions.sort();
            Ok(matched_versions.pop())
        } else {
            Ok(None)
        }
    }

    fn prioritize(&self, _package: &Self::P, _range: &Self::VS, _stats: &PackageResolutionStatistics) -> Self::Priority {
        0
    }
}

pub fn parse_semantic_version(s: &str) -> Result<SemanticVersion, ()> {
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() != 3 { return Err(()); }
    let major = parts[0].parse().map_err(|_| ())?;
    let minor = parts[1].parse().map_err(|_| ())?;
    let patch = parts[2].parse().map_err(|_| ())?;
    Ok(SemanticVersion::new(major, minor, patch))
}

/// Проверка зависимостей перед установкой
pub async fn resolve_dependencies(
    target_package: String,
    target_version: SemanticVersion,
    all_manifests: HashMap<String, Vec<(SemanticVersion, PackageManifest)>>,
) -> Result<SelectedDependencies<String, SemanticVersion>, String> {
    let provider = ModuleDependencyProvider { available_packages: all_manifests };
    
    match resolve(&provider, target_package, target_version) {
        Ok(res) => Ok(res),
        Err(PubGrubError::NoSolution(e)) => Err(format!("Dependency resolution failed: {:?}", e)),
        Err(e) => Err(format!("PubGrub error: {}", e)),
    }
}

/// Вспомогательная функция для конвертации semver -> pubgrub
pub fn to_pubgrub_ver(v: &str) -> SemanticVersion {
    parse_semantic_version(v).unwrap_or(SemanticVersion::zero())
}

/// Проверка доступен ли модуль (установлен и активен)
pub async fn is_module_available(module_code: &str, state: std::sync::Arc<crate::state::AppState>) -> bool {
    state.is_module_available(module_code).await
}
