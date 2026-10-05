//! Necessary host RAM ceilings. These are not available-memory measurements.

use pantograph_runtime_registry::RuntimeHostRamCapacitySource;

/// System RAM and, on Linux, every visible ancestor cgroup v2 hard memory limit.
///
/// Namespaces may hide ancestor limits. Configured budgets must still account for
/// those limits, external consumers, and the complete task envelope. Linux layouts
/// that cannot be resolved safely (including v1) return unavailable, never a cache.
#[derive(Debug, Default)]
pub struct NativeHostRamCapacitySource;

impl RuntimeHostRamCapacitySource for NativeHostRamCapacitySource {
    fn capacity_ceiling_bytes(&self) -> Option<u64> {
        let mut system = sysinfo::System::new();
        system.refresh_memory();
        let total = system.total_memory();
        if total == 0 {
            return None;
        }
        #[cfg(target_os = "linux")]
        {
            let cgroup = std::fs::read_to_string("/proc/self/cgroup").ok()?;
            let mounts = std::fs::read_to_string("/proc/self/mountinfo").ok()?;
            visible_cgroup_ceiling(total, &cgroup, &mounts, |path| {
                std::fs::read_to_string(path).ok()
            })
        }
        #[cfg(not(target_os = "linux"))]
        {
            Some(total)
        }
    }
}

#[cfg(target_os = "linux")]
fn visible_cgroup_ceiling(
    total: u64,
    cgroup: &str,
    mounts: &str,
    read: impl Fn(&std::path::Path) -> Option<String>,
) -> Option<u64> {
    use std::path::{Component, Path};
    let location = Path::new(cgroup.lines().find_map(|line| line.strip_prefix("0::"))?);
    if !location.is_absolute()
        || location
            .components()
            .any(|part| !matches!(part, Component::RootDir | Component::Normal(_)))
    {
        return None;
    }
    // A known mount layout avoids guessing namespace roots or decoding arbitrary mounts.
    let mounted = mounts.lines().any(|line| {
        let Some((fields, filesystem)) = line.split_once(" - ") else {
            return false;
        };
        let fields = fields.split_whitespace().collect::<Vec<_>>();
        fields.get(3) == Some(&"/")
            && fields.get(4) == Some(&"/sys/fs/cgroup")
            && filesystem.split_whitespace().next() == Some("cgroup2")
    });
    if !mounted {
        return None;
    }
    let root = Path::new("/sys/fs/cgroup");
    let mut current = root.join(location.strip_prefix("/").ok()?);
    let mut ceiling = total;
    loop {
        let limit = read(&current.join("memory.max"))?;
        let limit = limit.trim();
        if limit != "max" {
            ceiling = ceiling.min(limit.parse::<u64>().ok()?);
        }
        if current == root {
            break;
        }
        current = current.parent()?.to_path_buf();
    }
    Some(ceiling)
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    const MOUNT: &str = "10 1 0:1 / /sys/fs/cgroup rw - cgroup2 cgroup rw";

    #[test]
    fn host_ram_ceiling_checks_current_group_and_every_visible_ancestor() {
        let result = visible_cgroup_ceiling(100, "0::/parent/child\n", MOUNT, |path| {
            Some(
                match path.to_str().unwrap() {
                    "/sys/fs/cgroup/parent/child/memory.max" => "80",
                    "/sys/fs/cgroup/parent/memory.max" => "60",
                    "/sys/fs/cgroup/memory.max" => "max",
                    _ => panic!("unexpected path"),
                }
                .into(),
            )
        });
        assert_eq!(result, Some(60));
        assert_eq!(
            visible_cgroup_ceiling(100, "0::/", MOUNT, |_| Some("200".into())),
            Some(100)
        );
        assert_eq!(
            visible_cgroup_ceiling(100, "0::/", MOUNT, |_| Some("0".into())),
            Some(0)
        );
    }

    #[test]
    fn host_ram_ceiling_missing_malformed_or_unsupported_layout_is_unavailable() {
        for (group, mounts) in [("1:memory:/", MOUNT), ("0::/../other", MOUNT), ("0::/", "")] {
            assert_eq!(
                visible_cgroup_ceiling(100, group, mounts, |_| Some("60".into())),
                None
            );
        }
        assert_eq!(visible_cgroup_ceiling(100, "0::/", MOUNT, |_| None), None);
        assert_eq!(
            visible_cgroup_ceiling(100, "0::/", MOUNT, |_| Some("broken".into())),
            None
        );
    }

    #[test]
    fn native_host_ram_ceiling_drives_real_registry_admission() {
        use pantograph_runtime_registry::*;
        use std::sync::Arc;
        let source = Arc::new(NativeHostRamCapacitySource);
        let ceiling = source.capacity_ceiling_bytes();
        println!("native necessary RAM ceiling: {ceiling:?}");
        let registry = RuntimeRegistry::new();
        registry.register_runtime(RuntimeRegistration::new("pytorch", "PyTorch"));
        registry
            .configure_resource_domain(RuntimeResourceDomain {
                domain_id: "host.ram".into(),
                total_bytes: 100,
                safety_margin_bytes: 0,
                bindings: vec![RuntimeResourceDomainBinding {
                    runtime_id: "pytorch".into(),
                    resource_kind: RuntimeAdmissionResourceKind::RamBytes,
                }],
            })
            .unwrap();
        registry.bind_host_ram_capacity_source(source);
        let result = registry.acquire_reservation(RuntimeReservationRequest {
            runtime_id: "pytorch".into(),
            workflow_id: "native-probe".into(),
            reservation_owner_id: None,
            usage_profile: None,
            model_id: None,
            pin_runtime: false,
            retention_hint: RuntimeRetentionHint::Ephemeral,
            requirements: Some(RuntimeReservationRequirements::from_claims(vec![
                RuntimeReservationResourceClaim::ram_bytes(1),
            ])),
        });
        match ceiling {
            Some(bytes) if bytes > 0 => assert!(result.is_ok()),
            _ => {
                assert!(matches!(
                    result,
                    Err(RuntimeRegistryError::ResourceDomainAdmissionRejected {
                        available_bytes: 0,
                        ..
                    })
                ));
                assert!(registry.snapshot().reservations.is_empty());
            }
        }
    }
}
