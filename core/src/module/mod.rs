pub use danneo_sdk::module::*;

pub mod lua_adapter;

pub fn init_native_modules() {
    let _ = inventory::iter::<NativeModuleRegistration>().count();
}
