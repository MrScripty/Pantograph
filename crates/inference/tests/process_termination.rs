//! Actual Linux child lifecycle exercising Tauri's existing Tokio termination call.
#![cfg(unix)]

use tokio::process::Command;

#[tokio::test]
async fn reaped_child_shutdown_preserves_cached_termination_for_the_monitor() {
    let mut child = Command::new("/bin/sh")
        .args(["-c", "exit 0"])
        .spawn()
        .unwrap();
    let status = child.wait().await.unwrap();
    assert!(status.success());
    child
        .start_kill()
        .expect("a reaped child already satisfies the termination request");
    assert_eq!(child.try_wait().unwrap(), Some(status));
    child
        .start_kill()
        .expect("repeat stop preserves acknowledgment");
    assert_eq!(child.try_wait().unwrap(), Some(status));
}

#[tokio::test]
async fn live_child_shutdown_still_requires_observed_exit() {
    let mut child = Command::new("/bin/sleep")
        .arg("30")
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    assert!(child.try_wait().unwrap().is_none());
    child.start_kill().expect("signal accepted");
    let status = tokio::time::timeout(std::time::Duration::from_secs(2), child.wait())
        .await
        .expect("child must exit")
        .unwrap();
    assert!(!status.success());
    assert_eq!(child.try_wait().unwrap(), Some(status));
}
