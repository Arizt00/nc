use super::*;

pub(super) fn handle_clean() -> Result<(), Box<dyn std::error::Error>> {
    let root = workspace_repo_root();
    for rel in ["target/nc/primitive", "reports/primitive", "var/primitive"] {
        let path = root.join(rel);
        if path.exists() {
            fs::remove_dir_all(&path)?;
        }
    }
    println!("OK: primitive clean");
    Ok(())
}
