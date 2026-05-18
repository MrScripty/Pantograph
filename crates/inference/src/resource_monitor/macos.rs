use crate::resource_monitor::process_rss::ProcessRssResourceMonitor;

pub(crate) fn default_runtime_resource_monitor() -> ProcessRssResourceMonitor {
    ProcessRssResourceMonitor::default()
}
