use crate::models::module::ModuleInfo;

#[test]
fn test_sdk_models_instantiation() {
    let info = ModuleInfo {
        code: "test_mod".to_string(),
    };
    assert_eq!(info.code, "test_mod");
}
