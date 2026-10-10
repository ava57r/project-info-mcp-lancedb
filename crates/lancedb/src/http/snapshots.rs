//! Filesystem snapshots (`.tar.gz`) of the LanceDB directory + restore.

use std::fs;
use std::path::{Path, PathBuf};

use chrono::Local;
use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use serde::{Deserialize, Serialize};

/// Snapshot metadata returned by the REST API / dashboard.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotInfo {
    /// File name (e.g. `snapshot-2026-10-08T12-00-00.tar.gz`).
    pub name: String,
    /// Size in bytes.
    pub size_bytes: u64,
    /// Unix mtime.
    pub modified_unix: i64,
    /// Human-readable timestamp.
    pub created: String,
}

/// Ensures the snapshot directory exists.
pub fn ensure_dir(dir: &str) -> std::io::Result<PathBuf> {
    let path = PathBuf::from(dir);
    fs::create_dir_all(&path)?;
    Ok(path)
}

/// Lists snapshots (`.tar.gz` / `.tgz`) newest-first.
pub fn list_snapshots(dir: &str) -> Result<Vec<SnapshotInfo>, String> {
    let path = ensure_dir(dir).map_err(|e| format!("Error opening snapshot dir: {e}"))?;
    let entries = fs::read_dir(&path).map_err(|e| format!("Error reading snapshots: {e}"))?;
    let mut out = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let p = entry.path();
        let name = p
            .file_name()
            .map_or(String::new(), |n| n.to_string_lossy().into_owned());
        if !(name.ends_with(".tar.gz") || name.ends_with(".tgz") || name.ends_with(".tar")) {
            continue;
        }
        let meta = entry.metadata().map_err(|e| e.to_string())?;
        let modified_unix = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map_or(0, |d| d.as_secs() as i64);
        out.push(SnapshotInfo {
            name,
            size_bytes: meta.len(),
            modified_unix,
            created: chrono::DateTime::from_timestamp(modified_unix, 0).map_or(
                "-".to_string(),
                |dt| {
                    dt.with_timezone(&Local)
                        .format("%Y-%m-%d %H:%M:%S")
                        .to_string()
                },
            ),
        });
    }
    out.sort_by_key(|b| std::cmp::Reverse(b.modified_unix));
    Ok(out)
}

/// Creates a `.tar.gz` snapshot of the whole `db_dir`.
pub fn create_snapshot(db_dir: &str, snapshot_dir: &str) -> Result<SnapshotInfo, String> {
    let snap_path =
        ensure_dir(snapshot_dir).map_err(|e| format!("Error opening snapshot dir: {e}"))?;
    if !Path::new(db_dir).exists() {
        return Err(format!("Database directory '{db_dir}' does not exist"));
    }
    let stamp = Local::now().format("%Y-%m-%dT%H-%M-%S").to_string();
    let name = format!("snapshot-{stamp}.tar.gz");
    let dest = snap_path.join(&name);
    let file = fs::File::create(&dest).map_err(|e| format!("Error creating snapshot: {e}"))?;
    let enc = GzEncoder::new(file, Compression::default());
    let mut tar = tar::Builder::new(enc);
    tar.append_dir_all(".", db_dir)
        .map_err(|e| format!("Error archiving DB dir: {e}"))?;
    let enc = tar
        .into_inner()
        .map_err(|e| format!("Error finalizing tar: {e}"))?;
    enc.finish()
        .map_err(|e| format!("Error finalizing gzip: {e}"))?;
    let meta = fs::metadata(&dest).map_err(|e| e.to_string())?;
    Ok(SnapshotInfo {
        name,
        size_bytes: meta.len(),
        modified_unix: chrono::Utc::now().timestamp(),
        created: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
    })
}

/// Deletes a snapshot file (rejects path traversal).
pub fn delete_snapshot(snapshot_dir: &str, name: &str) -> Result<(), String> {
    let target = safe_path(snapshot_dir, name)?;
    if !target.exists() {
        return Err(format!("Snapshot '{name}' not found"));
    }
    fs::remove_file(&target).map_err(|e| format!("Error deleting snapshot: {e}"))?;
    Ok(())
}

/// Restores a snapshot: wipes `db_dir` and extracts the archive into it.
pub fn restore_snapshot(db_dir: &str, snapshot_dir: &str, name: &str) -> Result<(), String> {
    let archive = safe_path(snapshot_dir, name)?;
    if !archive.exists() {
        return Err(format!("Snapshot '{name}' not found"));
    }
    if Path::new(db_dir).exists() {
        fs::remove_dir_all(db_dir).map_err(|e| format!("Error clearing DB dir: {e}"))?;
    }
    fs::create_dir_all(db_dir).map_err(|e| format!("Error recreating DB dir: {e}"))?;
    let file = fs::File::open(&archive).map_err(|e| format!("Error opening snapshot: {e}"))?;
    tar::Archive::new(GzDecoder::new(file))
        .unpack(db_dir)
        .map_err(|e| format!("Error extracting snapshot: {e}"))?;
    Ok(())
}

/// Joins `name` onto `dir`, rejecting absolute paths and `..` escapes.
fn safe_path(dir: &str, name: &str) -> Result<PathBuf, String> {
    if name.contains("..") || name.contains('/') || name.contains('\\') {
        return Err("Invalid snapshot name".to_string());
    }
    Ok(PathBuf::from(dir).join(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_traversal_names() {
        assert!(safe_path("./snaps", "../evil.tar.gz").is_err());
        assert!(safe_path("./snaps", "a/b.tar.gz").is_err());
        assert!(safe_path("./snaps", "ok.tar.gz").is_ok());
    }

    #[test]
    fn round_trips_snapshot() {
        let base = std::env::temp_dir().join(format!("piim-lance-snap-{}", std::process::id()));
        let db = base.join("db");
        let snaps = base.join("snaps");
        fs::create_dir_all(&db).expect("db dir");
        fs::write(db.join("hello.txt"), "hi").expect("seed file");
        let info = create_snapshot(db.to_str().expect("utf8"), snaps.to_str().expect("utf8"))
            .expect("create");
        assert!(info.name.ends_with(".tar.gz"));
        let listed = list_snapshots(snaps.to_str().expect("utf8")).expect("list");
        assert_eq!(listed.len(), 1);
        fs::remove_dir_all(&db).expect("wipe");
        restore_snapshot(
            db.to_str().expect("utf8"),
            snaps.to_str().expect("utf8"),
            &info.name,
        )
        .expect("restore");
        assert_eq!(
            fs::read_to_string(db.join("hello.txt")).expect("read"),
            "hi"
        );
        delete_snapshot(snaps.to_str().expect("utf8"), &info.name).expect("delete");
        assert!(
            list_snapshots(snaps.to_str().expect("utf8"))
                .expect("relist")
                .is_empty()
        );
        fs::remove_dir_all(&base).ok();
    }
}
