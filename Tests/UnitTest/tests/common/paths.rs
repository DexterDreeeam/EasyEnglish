use std::path::PathBuf;

pub(crate) fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("UnitTest has a parent Tests directory")
        .parent()
        .expect("Tests has a parent workspace directory")
        .to_path_buf()
}

pub(crate) fn dict_file(filename: &str) -> PathBuf {
    workspace_root().join("Dict").join(filename)
}
