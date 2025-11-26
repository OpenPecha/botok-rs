//! Dialect pack downloading and management.
//!
//! This module handles automatic downloading of dialect packs from GitHub,
//! similar to how Python botok works.

use std::fs::{self, File};
use std::io::{self, Cursor};
use std::path::{Path, PathBuf};

/// Default dialect pack name
pub const DEFAULT_DIALECT_PACK: &str = "general";

/// GitHub repository for dialect packs
const BOTOK_DATA_REPO: &str = "Esukhia/botok-data";

/// Get the default base path for dialect packs
/// Returns ~/Documents/botok-rs/dialect_packs/
pub fn default_base_path() -> PathBuf {
    #[cfg(feature = "download")]
    {
        dirs::document_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("botok-rs")
            .join("dialect_packs")
    }
    #[cfg(not(feature = "download"))]
    {
        PathBuf::from(".")
    }
}

/// Get the path to a specific dialect pack
pub fn dialect_pack_path(dialect_name: &str, base_path: Option<&Path>) -> PathBuf {
    let base = base_path
        .map(PathBuf::from)
        .unwrap_or_else(default_base_path);
    base.join(dialect_name)
}

/// Check if a dialect pack exists locally
pub fn dialect_pack_exists(dialect_name: &str, base_path: Option<&Path>) -> bool {
    let path = dialect_pack_path(dialect_name, base_path);
    path.is_dir() && path.join("dictionary").is_dir()
}

/// Get the latest release version from GitHub
#[cfg(feature = "download")]
fn get_latest_version() -> Result<String, DialectPackError> {
    let url = format!("https://api.github.com/repos/{}/releases/latest", BOTOK_DATA_REPO);
    
    let client = reqwest::blocking::Client::builder()
        .user_agent("botok-rs")
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| DialectPackError::Network(e.to_string()))?;
    
    let response = client.get(&url)
        .send()
        .map_err(|e| DialectPackError::Network(e.to_string()))?;
    
    if !response.status().is_success() {
        return Err(DialectPackError::Network(format!(
            "GitHub API returned status: {}", response.status()
        )));
    }
    
    let json: serde_json::Value = response.json()
        .map_err(|e| DialectPackError::Network(e.to_string()))?;
    
    json["tag_name"]
        .as_str()
        .map(String::from)
        .ok_or_else(|| DialectPackError::Network("Could not find tag_name in response".into()))
}

/// Download a dialect pack from GitHub
#[cfg(feature = "download")]
pub fn download_dialect_pack(
    dialect_name: &str,
    base_path: Option<&Path>,
    version: Option<&str>,
) -> Result<PathBuf, DialectPackError> {
    let base = base_path
        .map(PathBuf::from)
        .unwrap_or_else(default_base_path);
    
    // Create base directory if it doesn't exist
    fs::create_dir_all(&base)
        .map_err(|e| DialectPackError::Io(e.to_string()))?;
    
    let pack_path = base.join(dialect_name);
    
    // If already exists, return the path
    if pack_path.is_dir() && pack_path.join("dictionary").is_dir() {
        return Ok(pack_path);
    }
    
    // Get version (latest if not specified)
    let version = match version {
        Some(v) => v.to_string(),
        None => get_latest_version()?,
    };
    
    let url = format!(
        "https://github.com/{}/releases/download/{}/{}.zip",
        BOTOK_DATA_REPO, version, dialect_name
    );
    
    eprintln!("[INFO] Downloading {} dialect pack (version {})...", dialect_name, version);
    
    // Download the zip file
    let client = reqwest::blocking::Client::builder()
        .user_agent("botok-rs")
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| DialectPackError::Network(e.to_string()))?;
    
    let response = client.get(&url)
        .send()
        .map_err(|e| DialectPackError::Network(e.to_string()))?;
    
    if !response.status().is_success() {
        return Err(DialectPackError::Network(format!(
            "Failed to download dialect pack: HTTP {}", response.status()
        )));
    }
    
    let bytes = response.bytes()
        .map_err(|e| DialectPackError::Network(e.to_string()))?;
    
    // Extract the zip file
    let cursor = Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor)
        .map_err(|e| DialectPackError::Zip(e.to_string()))?;
    
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)
            .map_err(|e| DialectPackError::Zip(e.to_string()))?;
        
        let outpath = match file.enclosed_name() {
            Some(path) => base.join(path),
            None => continue,
        };
        
        if file.name().ends_with('/') {
            fs::create_dir_all(&outpath)
                .map_err(|e| DialectPackError::Io(e.to_string()))?;
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(p)
                        .map_err(|e| DialectPackError::Io(e.to_string()))?;
                }
            }
            let mut outfile = File::create(&outpath)
                .map_err(|e| DialectPackError::Io(e.to_string()))?;
            io::copy(&mut file, &mut outfile)
                .map_err(|e| DialectPackError::Io(e.to_string()))?;
        }
    }
    
    eprintln!("[INFO] Download completed!");
    
    Ok(pack_path)
}

/// Get a dialect pack, downloading if necessary
#[cfg(feature = "download")]
pub fn get_dialect_pack(
    dialect_name: &str,
    base_path: Option<&Path>,
) -> Result<PathBuf, DialectPackError> {
    download_dialect_pack(dialect_name, base_path, None)
}

/// Get the default dialect pack (general), downloading if necessary
#[cfg(feature = "download")]
pub fn get_default_dialect_pack() -> Result<PathBuf, DialectPackError> {
    get_dialect_pack(DEFAULT_DIALECT_PACK, None)
}

/// List all TSV files in a dialect pack's dictionary
pub fn list_dictionary_files(dialect_pack_path: &Path) -> io::Result<Vec<PathBuf>> {
    let dict_path = dialect_pack_path.join("dictionary");
    if !dict_path.is_dir() {
        return Ok(Vec::new());
    }
    
    let mut files = Vec::new();
    collect_tsv_files(&dict_path, &mut files)?;
    Ok(files)
}

/// List all TSV files in a dialect pack's adjustments
pub fn list_adjustment_files(dialect_pack_path: &Path) -> io::Result<Vec<PathBuf>> {
    let adj_path = dialect_pack_path.join("adjustments");
    if !adj_path.is_dir() {
        return Ok(Vec::new());
    }
    
    let mut files = Vec::new();
    collect_tsv_files(&adj_path, &mut files)?;
    Ok(files)
}

fn collect_tsv_files(dir: &Path, files: &mut Vec<PathBuf>) -> io::Result<()> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                collect_tsv_files(&path, files)?;
            } else if path.extension().map_or(false, |ext| ext == "tsv") {
                files.push(path);
            }
        }
    }
    Ok(())
}

/// Errors that can occur when working with dialect packs
#[derive(Debug)]
pub enum DialectPackError {
    /// Network error during download
    Network(String),
    /// Error extracting zip file
    Zip(String),
    /// IO error
    Io(String),
    /// Dialect pack not found
    NotFound(String),
}

impl std::fmt::Display for DialectPackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DialectPackError::Network(msg) => write!(f, "Network error: {}", msg),
            DialectPackError::Zip(msg) => write!(f, "Zip error: {}", msg),
            DialectPackError::Io(msg) => write!(f, "IO error: {}", msg),
            DialectPackError::NotFound(msg) => write!(f, "Dialect pack not found: {}", msg),
        }
    }
}

impl std::error::Error for DialectPackError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_base_path() {
        let path = default_base_path();
        assert!(path.to_string_lossy().contains("botok-rs"));
    }

    #[test]
    fn test_dialect_pack_path() {
        let path = dialect_pack_path("general", None);
        assert!(path.to_string_lossy().contains("general"));
    }
}

