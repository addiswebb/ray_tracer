use std::path::Path;

const FILE: &str = concat!(env!("CARGO_MANIFEST_DIR"));

pub async fn load_string(path: &Path) -> anyhow::Result<String> {
    assert!(
        path.exists(),
        "Text file does not exist: {}",
        path.display()
    );

    Ok(std::fs::read_to_string(path)?)
}

pub async fn load_binary(path: &Path) -> anyhow::Result<Vec<u8>> {
    assert!(
        path.exists(),
        "Binary file does not exist: {}",
        path.display()
    );

    Ok(std::fs::read(path)?)
}