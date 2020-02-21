use chrono::{DateTime, Utc};
use directories::UserDirs;
use git2::Repository;
use std::fs;
use std::path::{Path, PathBuf};

pub fn get_output_path(label: &str) -> PathBuf {
    let ud = UserDirs::new().unwrap();
    let download_dir = ud.download_dir().unwrap();
    let utc: DateTime<Utc> = Utc::now();
    let filename = format!(
        "render-{0}-rev{1}-{2}.tiff",
        utc.format("%F-%H%M%S"),
        &git_sha_descriptor()[..7],
        label,
    );
    download_dir.join(filename)
}

fn git_sha_descriptor() -> String {
    let exepath = std::env::current_exe().unwrap();
    let repo = match Repository::discover(exepath.parent().unwrap()) {
        Ok(repo) => repo,
        Err(e) => panic!("failed to open: {}", e),
    };
    let head = match repo.head() {
        Ok(val) => val,
        Err(e) => panic!(e),
    };
    let commit = head.peel_to_commit().unwrap();
    commit.id().to_string()
}

pub fn open_preview(filename: impl AsRef<Path>) {
    use std::process::Command;

    Command::new("open")
        .arg(filename.as_ref().as_os_str())
        .spawn()
        .expect("Failed to start");
}

pub fn set_readonly(raw_preview_filename: impl AsRef<Path>) {
    let metadata = fs::metadata(&raw_preview_filename).unwrap();
    // Set readonly so that I don't accidentally save over it later.
    let mut p = metadata.permissions();
    p.set_readonly(true);
    fs::set_permissions(&raw_preview_filename, p).unwrap();
}
