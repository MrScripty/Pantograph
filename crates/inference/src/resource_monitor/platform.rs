#[cfg(target_os = "linux")]
#[path = "linux.rs"]
mod linux;
#[cfg(target_os = "macos")]
#[path = "macos.rs"]
mod macos;
#[cfg(target_os = "windows")]
#[path = "windows.rs"]
mod windows;

#[cfg(target_os = "linux")]
pub(crate) use linux::default_runtime_resource_monitor;
#[cfg(target_os = "macos")]
pub(crate) use macos::default_runtime_resource_monitor;
#[cfg(target_os = "windows")]
pub(crate) use windows::default_runtime_resource_monitor;

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
pub(crate) fn default_runtime_resource_monitor(
) -> crate::resource_monitor::unsupported::UnsupportedRuntimeResourceMonitor {
    crate::resource_monitor::unsupported::UnsupportedRuntimeResourceMonitor::default()
}
