// Pantograph modification. Apache-2.0.
// Capacity arithmetic is qualified against this exact standard-library build.
// Other toolchains still compile ordinary Tokenizers, but this API refuses.
use std::{env, process::Command};

fn main() {
    println!("cargo:rustc-check-cfg=cfg(pantograph_snapshot_qualified)");
    println!("cargo:rerun-if-env-changed=RUSTC");
    let compiler = env::var_os("RUSTC").expect("Cargo provides RUSTC");
    let output = Command::new(compiler)
        .arg("--version")
        .arg("--verbose")
        .output();
    let target = env::var("TARGET").unwrap_or_default();
    let qualified = output.is_ok_and(|output| {
        output.status.success()
            && String::from_utf8(output.stdout).is_ok_and(|version| {
                version.lines().any(|line| line == "release: 1.92.0")
                    && version
                        .lines()
                        .any(|line| line == "commit-hash: ded5c06cf21d2b93bffd5d884aa6e96934ee4234")
                    && version
                        .lines()
                        .any(|line| line == "host: x86_64-unknown-linux-gnu")
            })
    }) && target == "x86_64-unknown-linux-gnu";
    if qualified {
        println!("cargo:rustc-cfg=pantograph_snapshot_qualified");
    } else {
        println!("cargo:warning=Pantograph bounded snapshot is unqualified on this toolchain/target and will refuse");
    }
}
