use danneo_core::registry::{
    PackageManifest,
    dependency::{resolve_dependencies, to_pubgrub_ver},
};
use pubgrub::SemanticVersion;
use serde_json::json;
use std::collections::HashMap;

fn mock_manifest(id: &str, version: &str, deps: Vec<(&str, &str)>) -> PackageManifest {
    let mut dependencies = HashMap::new();
    for (d_id, d_ver) in deps {
        dependencies.insert(d_id.to_string(), d_ver.to_string());
    }

    serde_json::from_value(json!({
        "package": {
            "id": id,
            "type": "module",
            "version": version,
            "name": id
        },
        "dependencies": dependencies
    }))
    .unwrap()
}

#[tokio::test]
async fn test_dependency_resolver_success() {
    let mut all_manifests = HashMap::new();

    // settings 2.0.0 exists
    let settings_m = mock_manifest("settings", "2.0.0", vec![]);
    all_manifests.insert(
        "settings".to_string(),
        vec![(SemanticVersion::new(2, 0, 0), settings_m)],
    );

    // news 1.0.0 depends on settings >=2.0.0
    let news_m = mock_manifest("mod_news", "1.0.0", vec![("settings", ">=2.0.0")]);
    all_manifests.insert(
        "mod_news".to_string(),
        vec![(SemanticVersion::new(1, 0, 0), news_m)],
    );

    let res = resolve_dependencies(
        "mod_news".to_string(),
        SemanticVersion::new(1, 0, 0),
        all_manifests,
    )
    .await;

    assert!(
        res.is_ok(),
        "Should resolve correctly when dependencies are met"
    );
}

#[tokio::test]
async fn test_dependency_resolver_missing_mandatory() {
    let mut all_manifests = HashMap::new();

    // news depends on settings, but settings is missing from registry
    let news_m = mock_manifest("mod_news", "1.0.0", vec![("settings", ">=2.0.0")]);
    all_manifests.insert(
        "mod_news".to_string(),
        vec![(SemanticVersion::new(1, 0, 0), news_m)],
    );

    let res = resolve_dependencies(
        "mod_news".to_string(),
        SemanticVersion::new(1, 0, 0),
        all_manifests,
    )
    .await;

    assert!(
        res.is_err(),
        "Should fail when mandatory dependency is missing"
    );
}

#[tokio::test]
async fn test_dependency_resolver_version_mismatch() {
    let mut all_manifests = HashMap::new();

    // settings is 1.0.0
    let settings_m = mock_manifest("settings", "1.0.0", vec![]);
    all_manifests.insert(
        "settings".to_string(),
        vec![(SemanticVersion::new(1, 0, 0), settings_m)],
    );

    // news requires settings >=2.0.0
    let news_m = mock_manifest("mod_news", "1.0.0", vec![("settings", ">=2.0.0")]);
    all_manifests.insert(
        "mod_news".to_string(),
        vec![(SemanticVersion::new(1, 0, 0), news_m)],
    );

    let res = resolve_dependencies(
        "mod_news".to_string(),
        SemanticVersion::new(1, 0, 0),
        all_manifests,
    )
    .await;

    // PubGrub 0.4 handles mismatches by returning Error
    assert!(
        res.is_err(),
        "Should fail when dependency version is incompatible"
    );
}

#[tokio::test]
async fn test_pubgrub_semantic_version_conversion() {
    let v = to_pubgrub_ver("2.1.3");
    assert_eq!(v.to_string(), "2.1.3");

    let v_bad = to_pubgrub_ver("not-a-version");
    assert_eq!(v_bad.to_string(), "0.0.0");
}
