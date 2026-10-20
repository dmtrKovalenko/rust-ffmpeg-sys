//! Prebuilt FFmpeg binaries.
//!
//! Compiling FFmpeg from source (the `build` feature) takes minutes. For every release of this
//! crate, CI compiles FFmpeg for the common targets and feature sets and uploads the result to
//! GitHub releases. Before compiling from source, the build script tries to download the archive
//! matching the target and the enabled features and falls back to compiling when there is none.
//!
//! An archive is the `dist` directory of `make install` (`lib/`, `include/`) plus
//! `extralibs.txt`, the `EXTRALIBS` lines of FFmpeg's `ffbuild/config.mak`, which list the
//! system libraries FFmpeg has to be linked with.
//!
//! Environment variables:
//! - `FFMPEG_BINARIES_URL`: download URL template, `{tag}` and `{key}` are replaced. `file://`
//!   URLs work for testing.
//! - `FFMPEG_FORCE_BUILD`: always compile from source.
//! - `FFMPEG_BINARIES_CACHE`: where downloaded archives are kept (default: the user cache
//!   directory), so that every project on the machine downloads them once.
//! - `FFMPEG_BINARIES_EXPORT_DIR`: after compiling from source, write the archive and its key to
//!   this directory (used by CI to produce the binaries).
//! - `FFMPEG_BINARIES_BUNDLE_LIBS`: static libraries (a path list, like `PATH`) that are copied
//!   into the archive's `lib/` on export. CI bundles x264 this way: its ABI version is part of
//!   its symbol names (`x264_encoder_open_165`), so an FFmpeg compiled against one x264 doesn't
//!   link with another, and the system's x264 differs from machine to machine.
//!
//! Archives are compiled with the `build-portable` flags, so setting `FFMPEG_MARCH` or
//! `FFMPEG_MTUNE` compiles from source as well.

use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

const DEFAULT_URL: &str =
    "https://github.com/dmtrKovalenko/rust-ffmpeg-sys/releases/download/{tag}/ffmpeg-{key}.tar.gz";

/// Name of the file in the `dist` directory that keeps the `EXTRALIBS` of the build.
pub const EXTRALIBS_FILE: &str = "extralibs.txt";

pub fn rerun_if_env_changed() {
    for var in [
        "FFMPEG_BINARIES_URL",
        "FFMPEG_FORCE_BUILD",
        "FFMPEG_BINARIES_CACHE",
        "FFMPEG_BINARIES_EXPORT_DIR",
        "FFMPEG_BINARIES_BUNDLE_LIBS",
        "FFMPEG_MARCH",
        "FFMPEG_MTUNE",
    ] {
        println!("cargo:rerun-if-env-changed={var}");
    }
}

/// Release tag the binaries of this crate version are uploaded to.
pub fn tag() -> String {
    format!("binaries-{}", env::var("CARGO_PKG_VERSION").unwrap())
}

/// Identifies a build: the target and every enabled feature that changes what is compiled.
pub fn key() -> String {
    let mut features: Vec<String> = env::vars()
        .filter_map(|(var, _)| {
            let feature = var.strip_prefix("CARGO_FEATURE_")?.to_lowercase();
            match feature.as_str() {
                // implied by every prebuilt archive or irrelevant to the compiled code
                "build" | "static" | "build_portable" | "non_exhaustive_enums" | "default" => None,
                _ => {
                    let short = ["build_lib_", "build_license_", "build_"]
                        .iter()
                        .find_map(|prefix| feature.strip_prefix(prefix))
                        .unwrap_or(&feature);
                    Some(short.replace('_', "-"))
                }
            }
        })
        .collect();
    features.sort();

    let target = env::var("TARGET").unwrap();
    if features.is_empty() {
        target
    } else {
        format!("{target}-{}", features.join("-"))
    }
}

/// Tries to download and unpack prebuilt binaries into `dist`. Returns `false` when FFmpeg has to
/// be compiled from source.
pub fn try_install(dist: &Path) -> bool {
    if env::var_os("FFMPEG_FORCE_BUILD").is_some()
        || env::var_os("FFMPEG_MARCH").is_some()
        || env::var_os("FFMPEG_MTUNE").is_some()
        || env::var_os("FFMPEG_BINARIES_EXPORT_DIR").is_some()
    {
        return false;
    }

    let (tag, key) = (tag(), key());
    let url = env::var("FFMPEG_BINARIES_URL")
        .unwrap_or_else(|_| DEFAULT_URL.to_owned())
        .replace("{tag}", &tag)
        .replace("{key}", &key);

    match install(&url, &key, dist) {
        Ok(()) => {
            println!("cargo:warning=Using prebuilt FFmpeg binaries {tag}/{key}");
            true
        }
        Err(err) => {
            println!("No prebuilt FFmpeg binaries at {url} ({err}), compiling from source");
            let _ = fs::remove_dir_all(dist);
            false
        }
    }
}

fn install(url: &str, key: &str, dist: &Path) -> io::Result<()> {
    let archive = match cache_dir() {
        Some(cache) => {
            let archive = cache.join(tag()).join(format!("ffmpeg-{key}.tar.gz"));
            if !archive.exists() {
                download(url, &archive)?;
            }
            archive
        }
        None => {
            let archive =
                PathBuf::from(env::var("OUT_DIR").unwrap()).join("ffmpeg-prebuilt.tar.gz");
            download(url, &archive)?;
            archive
        }
    };

    fs::create_dir_all(dist)?;
    let unpacked = run(Command::new("tar")
        .arg("-xzf")
        .arg(&archive)
        .arg("-C")
        .arg(dist));
    if let Err(err) = unpacked {
        // A broken cached archive must not break every later build.
        let _ = fs::remove_file(&archive);
        return Err(err);
    }

    for required in ["lib", "include", EXTRALIBS_FILE] {
        if !dist.join(required).exists() {
            return Err(io::Error::other(format!("archive has no {required}")));
        }
    }
    Ok(())
}

fn download(url: &str, destination: &Path) -> io::Result<()> {
    fs::create_dir_all(destination.parent().unwrap())?;
    // Written next to the destination and renamed, so parallel builds never see half a file.
    let partial = destination.with_extension(format!("partial-{}", std::process::id()));
    let result = run(Command::new("curl")
        .args([
            "--fail",
            "--location",
            "--silent",
            "--show-error",
            "--retry",
            "3",
        ])
        .arg("--output")
        .arg(&partial)
        .arg(url))
    .and_then(|()| fs::rename(&partial, destination));
    if result.is_err() {
        let _ = fs::remove_file(&partial);
    }
    result
}

fn cache_dir() -> Option<PathBuf> {
    if let Some(dir) = env::var_os("FFMPEG_BINARIES_CACHE") {
        return Some(PathBuf::from(dir));
    }
    let home = env::var_os("HOME").map(PathBuf::from);
    let base = if cfg!(target_os = "macos") {
        home.map(|home| home.join("Library/Caches"))
    } else if cfg!(windows) {
        env::var_os("LOCALAPPDATA").map(PathBuf::from)
    } else {
        env::var_os("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .or_else(|| home.map(|home| home.join(".cache")))
    };
    Some(base?.join("ffmpeg-sys-fframes"))
}

/// Keeps the `EXTRALIBS` lines of a source build next to the libraries, so that a prebuilt
/// archive knows which system libraries to link.
pub fn save_extralibs(config_mak: &Path, dist: &Path) -> io::Result<()> {
    let extralibs: String = fs::read_to_string(config_mak)?
        .lines()
        .filter(|line| line.starts_with("EXTRALIBS"))
        .map(|line| format!("{line}\n"))
        .collect();
    fs::write(dist.join(EXTRALIBS_FILE), extralibs)
}

/// Writes `ffmpeg-{key}.tar.gz` and `key.txt`/`tag.txt` to `FFMPEG_BINARIES_EXPORT_DIR`.
pub fn export(dist: &Path) -> io::Result<()> {
    let Some(export_dir) = env::var_os("FFMPEG_BINARIES_EXPORT_DIR").map(PathBuf::from) else {
        return Ok(());
    };
    fs::create_dir_all(&export_dir)?;
    // Next to FFmpeg's own libraries, the linker finds them before the system's.
    if let Some(libs) = env::var_os("FFMPEG_BINARIES_BUNDLE_LIBS") {
        for lib in env::split_paths(&libs) {
            let name = lib
                .file_name()
                .ok_or_else(|| io::Error::other(format!("{} is not a library", lib.display())))?;
            fs::copy(&lib, dist.join("lib").join(name))?;
        }
    }
    let key = key();
    run(Command::new("tar")
        // pkg-config files hold absolute paths of the build machine
        .arg("--exclude=lib/pkgconfig")
        .arg("-czf")
        .arg(export_dir.join(format!("ffmpeg-{key}.tar.gz")))
        .arg("-C")
        .arg(dist)
        .args(["lib", "include", EXTRALIBS_FILE]))?;
    fs::write(export_dir.join("key.txt"), &key)?;
    fs::write(export_dir.join("tag.txt"), tag())
}

fn run(command: &mut Command) -> io::Result<()> {
    let output = command.output()?;
    if output.status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "{command:?} failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )))
    }
}
