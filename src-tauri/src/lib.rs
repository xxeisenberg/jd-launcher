mod helper;
mod java_manager;
mod microsoft_auth;
mod modloaders;
mod modpacks;
mod mojang_api;
mod profiles;

use crate::helper::get_system_memory_mb;
use crate::java_manager::{
    detect_system_javas, download_java, get_local_java_version, get_required_java_version,
};
use crate::microsoft_auth::{
    get_active_account, get_auth_mode, list_accounts, logout_account, poll_ms_login,
    refresh_active_account, start_ms_login, switch_account,
};
use crate::modloaders::get_modloader_versions;
use crate::modpacks::{
    check_modpack_update, get_modpack_versions, install_modpack, install_modrinth_content,
    search_modpacks, search_modrinth, update_modpack,
};
use crate::mojang_api::{download_version_and_run, get_available_versions, save_log_file};
use crate::profiles::{
    delete_content, delete_profile, duplicate_profile, export_profile, get_last_profile_id,
    get_settings, import_profile, list_mods, list_profiles, list_resource_packs, list_shaders,
    reset_settings, save_profile, set_last_profile_id, toggle_content, update_settings,
};
#[cfg(debug_assertions)]
use specta_typescript::Typescript;
use tauri_specta::{collect_commands, Builder};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Apply global settings on startup
    let config = profiles::load_profiles();
    let settings = config.settings;

    if let Some(proxy) = settings.http_proxy.as_ref().filter(|s| !s.is_empty()) {
        std::env::set_var("HTTP_PROXY", proxy);
        std::env::set_var("HTTPS_PROXY", proxy);
    }

    if settings.verbose_logging {
        std::env::set_var("RUST_LOG", "debug");
    } else if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }

    // Initialize logger if one is being used (e.g., env_logger)
    // env_logger::init(); // Uncomment if env_logger is added to dependencies later

    let builder = Builder::<tauri::Wry>::new().commands(collect_commands![
        get_available_versions,
        download_version_and_run,
        list_profiles,
        save_profile,
        delete_profile,
        duplicate_profile,
        get_last_profile_id,
        set_last_profile_id,
        get_settings,
        update_settings,
        reset_settings,
        export_profile,
        import_profile,
        list_mods,
        list_shaders,
        list_resource_packs,
        get_modloader_versions,
        get_auth_mode,
        start_ms_login,
        poll_ms_login,
        get_active_account,
        list_accounts,
        switch_account,
        logout_account,
        refresh_active_account,
        detect_system_javas,
        download_java,
        get_required_java_version,
        get_local_java_version,
        save_log_file,
        search_modpacks,
        get_modpack_versions,
        install_modpack,
        check_modpack_update,
        update_modpack,
        get_system_memory_mb,
        search_modrinth,
        install_modrinth_content,
        toggle_content,
        delete_content
    ]);

    #[cfg(debug_assertions)]
    builder
        .export(
            Typescript::default().header("// @ts-nocheck\n"),
            "../src/bindings.ts",
        )
        .expect("Failed to export typescript bindings");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(builder.invoke_handler())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
