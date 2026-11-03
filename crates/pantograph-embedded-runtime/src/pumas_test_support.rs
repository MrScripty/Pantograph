//! Shared startup preparation for Pumas owner API fixtures.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use pumas_library::registry::LibraryRegistry;

pub(crate) fn builder(launcher_root: impl Into<PathBuf>) -> pumas_library::PumasApiBuilder {
    static REGISTRY: OnceLock<LibraryRegistry> = OnceLock::new();

    // Fixture model directories are independent, but every owner API opens the
    // same platform registry. Finish its first WAL/schema initialization once
    // before parallel builders open additional connections, and keep it alive
    // for the test process. Initialization errors still fail the test.
    let db_path = pumas_library::platform::registry_db_path().expect("Pumas test registry path");
    prepare_registry(&REGISTRY, &db_path);
    pumas_library::PumasApi::builder(launcher_root)
}

fn prepare_registry(registry: &OnceLock<LibraryRegistry>, db_path: &Path) {
    registry
        .get_or_init(|| LibraryRegistry::open_at(db_path).expect("Pumas test registry startup"));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Barrier;

    #[test]
    fn concurrent_owner_registry_connections_follow_completed_cold_startup() {
        const OWNERS: usize = 32;

        for _ in 0..8 {
            let temp = tempfile::tempdir().expect("isolated registry directory");
            let db_path = temp.path().join("registry.db");
            let registry = OnceLock::new();
            let barrier = Barrier::new(OWNERS);

            std::thread::scope(|scope| {
                let mut owners = Vec::new();
                for owner in 0..OWNERS {
                    let library_path = temp.path().join(format!("owner-{owner}"));
                    std::fs::create_dir(&library_path).expect("owner library directory");
                    let db_path = &db_path;
                    let registry = &registry;
                    let barrier = &barrier;
                    owners.push(scope.spawn(move || {
                        barrier.wait();
                        prepare_registry(registry, db_path);
                        let connection =
                            LibraryRegistry::open_at(db_path).expect("owner registry connection");
                        connection
                            .register(&library_path, "test-owner")
                            .expect("owner registration");
                        assert!(matches!(
                            connection
                                .try_claim_instance(&library_path, std::process::id())
                                .expect("owner instance claim"),
                            pumas_library::registry::InstanceClaimResult::Claimed(_)
                        ));
                        connection
                            .unregister_instance(&library_path)
                            .expect("owner instance cleanup");
                    }));
                }
                for owner in owners {
                    owner.join().expect("owner startup should succeed");
                }
            });

            assert_eq!(
                registry
                    .get()
                    .expect("prepared registry")
                    .list()
                    .expect("registered owners")
                    .len(),
                OWNERS
            );
        }
    }
}
