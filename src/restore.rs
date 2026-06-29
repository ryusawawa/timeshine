use std::fs::{self, File};
use std::io::{self, Read};
use std::path::Path;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::os::unix::fs::chown;
use std::ffi::CString;
use serde::Deserialize;
use rayon::prelude::*;

#[derive(Deserialize)]
struct BackupIndex {
    files: Vec<FileMetadata>,
    directories: Vec<DirMetadata>,
}

#[derive(Deserialize)]
struct FileMetadata {
    relative_path: String,
    hash: Option<String>,
    mode: u32,
    uid: u32,
    gid: u32,
    file_type: String,
    symlink_target: Option<String>,
    rdev: u64,
    xattrs: Vec<(String, Vec<u8>)>,
}

#[derive(Deserialize)]
struct DirMetadata {
    relative_path: String,
    mode: u32,
    uid: u32,
    gid: u32,
    xattrs: Vec<(String, Vec<u8>)>,
}

pub fn run_restore(snapshot_path: &Path, dest_dir: &Path, dry_run: bool) -> io::Result<()> {
    if !snapshot_path.is_file() {
        return Err(io::Error::new(io::ErrorKind::NotFound, "Snapshot file specified not found"));
    }

    if dry_run {
        println!("--- DRY RUN MODE (No files will be modified) ---");
    }

    let mut file = File::open(snapshot_path)?;
    let mut json_str = String::new();
    file.read_to_string(&mut json_str)?;

    let index: BackupIndex = serde_json::from_str(&json_str)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    if !dry_run {
        fs::create_dir_all(dest_dir)?;
    }

    for dir_info in &index.directories {
        let clean_relative = dir_info.relative_path.replace("\\", "/");
        let target_path = dest_dir.join(clean_relative);
        if dry_run {
            println!("[Dry-Run] Create Directory: {:?}", target_path);
        } else {
            fs::create_dir_all(&target_path)?;
            let _ = fs::set_permissions(&target_path, fs::Permissions::from_mode(dir_info.mode));
            let _ = chown(&target_path, Some(dir_info.uid), Some(dir_info.gid));
            for (key, val) in &dir_info.xattrs {
                let _ = xattr::set(&target_path, key, val);
            }
        }
    }

    let restore_results: Vec<io::Result<()>> = index.files
        .into_par_iter()
        .map(|file_info| {
            let clean_relative = file_info.relative_path.replace("\\", "/");
            let target_file_path = dest_dir.join(clean_relative);

            if !dry_run && (target_file_path.exists() || target_file_path.is_symlink()) {
                let _ = fs::remove_file(&target_file_path);
            }

            if file_info.file_type == "symlink" {
                if let Some(ref link_target) = file_info.symlink_target {
                    if dry_run {
                        println!("[Dry-Run] Create Symlink: {:?} -> {}", target_file_path, link_target);
                    } else {
                        symlink(link_target, &target_file_path)?;
                        let _ = chown(&target_file_path, Some(file_info.uid), Some(file_info.gid));
                        for (key, val) in &file_info.xattrs {
                            let _ = xattr::set(&target_file_path, key, val);
                        }
                    }
                }
                return Ok(());
            }

            if file_info.file_type != "file" {
                if dry_run {
                    println!("[Dry-Run] Create Special Device ({}): {:?}", file_info.file_type, target_file_path);
                } else {
                    let path_str = target_file_path.to_string_lossy();
                    if let Ok(c_str) = CString::new(path_str.as_ref() as &str) {
                        unsafe {
                            libc::mknod(c_str.as_ptr(), file_info.mode as libc::mode_t, file_info.rdev as libc::dev_t);
                        }
                        let _ = chown(&target_file_path, Some(file_info.uid), Some(file_info.gid));
                        for (key, val) in &file_info.xattrs {
                            let _ = xattr::set(&target_file_path, key, val);
                        }
                    }
                }
                return Ok(());
            }

            if let Some(ref hash_str) = file_info.hash {
                let object_path = Path::new("objects").join(hash_str);
                if !object_path.is_file() {
                    return Err(io::Error::new(
                        io::ErrorKind::NotFound,
                        format!("Missing object data for hash: {}", hash_str),
                    ));
                }

                if dry_run {
                    println!("[Dry-Run] Restore File: {:?}", target_file_path);
                } else {
                    if let Some(parent) = target_file_path.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    fs::copy(&object_path, &target_file_path)?;
                    let _ = fs::set_permissions(&target_file_path, fs::Permissions::from_mode(file_info.mode));
                    let _ = chown(&target_file_path, Some(file_info.uid), Some(file_info.gid));
                    for (key, val) in &file_info.xattrs {
                        let _ = xattr::set(&target_file_path, key, val);
                    }
                }
            }
            Ok(())
        })
        .collect();

    for result in restore_results {
        result?;
    }

    println!("Restore processing finished.");
    Ok(())
}
