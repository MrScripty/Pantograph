// Exercise the exact desktop-owned startup helper and its service/reopen tests
// in headless CI. This does not compile or launch the Tauri application.
#[path = "../../../src-tauri/src/app_setup/artifacts.rs"]
mod desktop_artifact_startup;
