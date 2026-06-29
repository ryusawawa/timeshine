use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::os::unix::fs::MetadataExt;
use chrono::Utc;
use sha2::{Sha256, Digest};
use rayon::prelude::*;
use serde::Serialize;

#[derive(Serialize)]
struct BackupIndex {
    timestamp: String,
    target_directory: String,
    system_mode: bool,
    files: Vec<FileMetadata>,
    directories: Vec<DirMetadata>,
}

#[derive(Serialize)]
struct FileMetadata {
    relative_path: String,
    hash: Option<String>,
    size: u64,
    modified: String,
    mode: u32,
    uid: u32,
    gid: u32,
    file_type: String, // "file", "symlink", "block", "char", "fifo", "socket"
    symlink_target: Option<String>,
    rdev: u64,
    xattrs: Vec<(String, Vec<u8>)>,
}

#[derive(Serialize)]
struct DirMetadata {
    relative_path: String,
    mode: u32,
    uid: u32,
    gid: u32,
    xattrs: Vec<(String, Vec<u8>)>,
}

pub fn run_backup(dir: &Path, system_mode: bool) -> io::Result<()> {
    let canonical_target = fs::canonicalize(dir)?;
    if !canonical_target.is_dir() {
        return Err(io::Error::new(io::ErrorKind::NotFound, "Target directory not found"));
    }

    let timestamp = Utc::now().format("%Y%m%d%H%M%S").to_string();
    let objects_dir = Path::new("objects");
    fs::create_dir_all(objects_dir)?;

    let mut file_paths = Vec::new();
    let mut dir_paths = Vec::new();
    scan_target_recursive(&canonical_target, &mut file_paths, &mut dir_paths, system_mode)?;
    println!("Scan summary - Items: {}, Directories: {}", file_paths.len(), dir_paths.len());

    let metadata_results: Vec<io::Result<FileMetadata>> = file_paths
        .into_par_iter()
        .map(|absolute_path| {
            let fs_meta = fs::symlink_metadata(&absolute_path)?;
            let raw_type = fs_meta.file_type();
            
            let mode = fs_meta.mode();
            let uid = fs_meta.uid();
            let gid = fs_meta.gid();
            let size = fs_meta.len();
            let rdev = fs_meta.rdev();
            
            let modified_time = fs_meta.modified()
                .map(|t| {
                    let datetime: chrono::DateTime<Utc> = t.into();
                    datetime.to_rfc3339()
                })
                .unwrap_or_else(|_| "".to_string());

            let relative = absolute_path.strip_prefix(&canonical_target)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            let relative_str = relative.to_string_lossy().replace("\\", "/");

            let mut xattrs = Vec::new();
            if let Ok(keys) = xattr::list(&absolute_path) {
                for key in keys {
                    if let Ok(Some(val)) = xattr::get(&absolute_path, &key) {
                        xattrs.push((key.to_string_lossy().into_owned(), val));
                    }
                }
            }

            if raw_type.is_symlink() {
                let target_path = fs::read_link(&absolute_path)?;
                return Ok(FileMetadata {
                    relative_path: relative_str,
                    hash: None,
                    size: 0,
                    modified: modified_time,
                    mode,
                    uid,
                    gid,
                    file_type: "symlink".to_string(),
                    symlink_target: Some(target_path.to_string_lossy().into_owned()),
                    rdev,
                    xattrs,
                });
            }

            use std::os::unix::fs::FileTypeExt;
            let f_type = if raw_type.is_block_device() {
                "block".to_string()
            } else if raw_type.is_char_device() {
                "char".to_string()
            } else if raw_type.is_fifo() {
                "fifo".to_string()
            } else if raw_type.is_socket() {
                "socket".to_string()
            } else {
                "file".to_string()
            };

            if f_type != "file" {
                return Ok(FileMetadata {
                    relative_path: relative_str,
                    hash: None,
                    size: 0,
                    modified: modified_time,
                    mode,
                    uid,
                    gid,
                    file_type: f_type,
                    symlink_target: None,
                    rdev,
                    xattrs,
                });
            }

            let hash = calculate_file_hash(&absolute_path)?;
            let object_path = Path::new("objects").join(&hash);
            if !object_path.exists() {
                let _ = fs::copy(&absolute_path, object_path);
            }

            Ok(FileMetadata {
                relative_path: relative_str,
                hash: Some(hash),
                size,
                modified: modified_time,
                mode,
                uid,
                gid,
                file_type: "file".to_string(),
                symlink_target: None,
                rdev,
                xattrs,
            })
        })
        .collect();

    let mut validated_files = Vec::new();
    for result in metadata_results {
        if let Ok(meta) = result {
            validated_files.push(meta);
        }
    }

    let mut validated_dirs = Vec::new();
    for d_path in dir_paths {
        if let Ok(fs_meta) = fs::metadata(&d_path) {
            if let Ok(relative) = d_path.strip_prefix(&canonical_target) {
                let mut xattrs = Vec::new();
                if let Ok(keys) = xattr::list(&d_path) {
                    for key in keys {
                        if let Ok(Some(val)) = xattr::get(&d_path, &key) {
                            xattrs.push((key.to_string_lossy().into_owned(), val));
                        }
                    }
                }
                validated_dirs.push(DirMetadata {
                    relative_path: relative.to_string_lossy().replace("\\", "/"),
                    mode: fs_meta.mode(),
                    uid: fs_meta.uid(),
                    gid: fs_meta.gid(),
                    xattrs,
                });
            }
        }
    }

    let index = BackupIndex {
        timestamp: timestamp.clone(),
        target_directory: canonical_target.to_string_lossy().into_owned(),
        system_mode,
        files: validated_files,
        directories: validated_dirs,
    };

    let snapshot_name = format!("snapshot_{}.json", timestamp);
    let mut snapshot_file = File::create(&snapshot_name)?;
    let json_data = serde_json::to_string_pretty(&index)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    snapshot_file.write_all(json_data.as_bytes())?;

    println!("Backup saved: {}", snapshot_name);
    Ok(())
}

fn scan_target_recursive(dir: &Path, files: &mut Vec<PathBuf>, dirs: &mut Vec<PathBuf>, system_mode: bool) -> io::Result<()> {
    if dir.is_dir() {
        dirs.push(dir.to_path_buf());
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if system_mode {
                let path_str = path.to_string_lossy();
                if path_str.contains("/proc") 
                    || path_str.contains("/sys") 
                    || path_str.contains("/dev") 
                    || path_str.contains("/run") 
                    || path_str.contains("/tmp")
                    || path_str.contains("/lost+found")
                    || path_str.contains("/timeshine/objects") {
                    continue;
                }
            }

            let file_type = entry.file_type()?;
            if file_type.is_dir() {
                let _ = scan_target_recursive(&path, files, dirs, system_mode);
            } else {
                files.push(path);
            }
        }
    }
    Ok(())
}

fn calculate_file_hash(path: &Path) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0; 65536];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 { break; }
        hasher.update(&buffer[..count]);
    }
    Ok(hex::encode(hasher.finalize()))
}
