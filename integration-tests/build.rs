#[path = "src/get_workspace_dir.rs"]
mod get_workspace_dir;

use crate::get_workspace_dir::get_workspace_dir;

static CONTRACT_DIR: &str = "contract";

fn main() {
    let workspace_dir = get_workspace_dir();
    let contract_dir = workspace_dir.join(CONTRACT_DIR);
    // Tell Cargo to rerun this script if the `contract` source changes.
    println!("cargo:rerun-if-changed={}", contract_dir.to_str().expect("valid UTF-8 path"));
    // Run build_local.sh
    let status = std::process::Command::new("bash")
        .arg("build_local.sh")
        .current_dir(&workspace_dir)
        .status()
        .expect("failed to execute build_local.sh");
    assert!(status.success());
}
