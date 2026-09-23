use crate::{
    deployment,
    error::{AppError, Result},
    models::{ConfigChangePreview, ConfigDocument, ConfigPatchRecord},
};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
};
use uuid::Uuid;
use walkdir::WalkDir;

const MAX_EDITABLE_BYTES: u64 = 2 * 1024 * 1024;

fn sha256_bytes(bytes: &[u8]) -> String {
    let mut hash = Sha256::new();
    hash.update(bytes);
    hex::encode(hash.finalize())
}

fn format_for(path: &Path) -> Option<(&'static str, bool)> {
    match path
        .extension()?
        .to_string_lossy()
        .to_ascii_lowercase()
        .as_str()
    {
        "ini" => Some(("ini", true)),
        "json" => Some(("json", true)),
        "toml" => Some(("toml", true)),
        "lua" => Some(("lua", false)),
        _ => None,
    }
}

fn allowed_roots(game: &Path) -> Vec<PathBuf> {
    vec![
        deployment::config_root(game),
        game.join("SWZeroCompany/Binaries/Win64/ue4ss"),
    ]
}

fn allowed_path(game: &Path, path: &Path) -> Result<PathBuf> {
    let canonical = path
        .canonicalize()
        .map_err(|_| AppError::Other("That configuration file no longer exists.".into()))?;
    let allowed = allowed_roots(game)
        .into_iter()
        .filter_map(|root| root.canonicalize().ok())
        .any(|root| canonical.starts_with(root));
    if !allowed {
        return Err(AppError::Other(
            "Only Zero Company and UE4SS configuration files can be edited here.".into(),
        ));
    }
    Ok(canonical)
}

fn read_document(path: &Path) -> Result<ConfigDocument> {
    let metadata = fs::metadata(path)?;
    if metadata.len() > MAX_EDITABLE_BYTES {
        return Err(AppError::Other(
            "That configuration file is larger than the 2 MiB workbench limit.".into(),
        ));
    }
    let bytes = fs::read(path)?;
    let content = String::from_utf8(bytes.clone()).map_err(|_| {
        AppError::Other(
            "The configuration file is not UTF-8 text, so it is read-only outside the workbench."
                .into(),
        )
    })?;
    let (format, writable) = format_for(path)
        .ok_or_else(|| AppError::Other("That configuration format is not supported.".into()))?;
    Ok(ConfigDocument {
        path: path.display().to_string(),
        format: format.into(),
        content,
        sha256: sha256_bytes(&bytes),
        writable,
    })
}

pub fn list(game: &Path) -> Result<Vec<ConfigDocument>> {
    let mut result = Vec::new();
    for root in allowed_roots(game) {
        if !root.is_dir() {
            continue;
        }
        for entry in WalkDir::new(&root).max_depth(5).follow_links(false) {
            let Ok(entry) = entry else { continue };
            if !entry.file_type().is_file() || format_for(entry.path()).is_none() {
                continue;
            }
            if entry
                .metadata()
                .ok()
                .is_some_and(|metadata| metadata.len() <= MAX_EDITABLE_BYTES)
            {
                if let Ok(document) = read_document(entry.path()) {
                    result.push(document);
                }
            }
        }
    }
    result.sort_by(|a, b| {
        a.path
            .to_ascii_lowercase()
            .cmp(&b.path.to_ascii_lowercase())
    });
    Ok(result)
}

pub fn read(game: &Path, path: &Path) -> Result<ConfigDocument> {
    read_document(&allowed_path(game, path)?)
}

fn validate_ini(content: &str) -> std::result::Result<(), String> {
    let mut section_seen = false;
    for (index, line) in content.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with([';', '#']) {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') && line.len() > 2 {
            section_seen = true;
            continue;
        }
        if !line.contains('=') {
            return Err(format!(
                "Line {} is neither a section nor a key/value pair.",
                index + 1
            ));
        }
    }
    if !section_seen && !content.trim().is_empty() {
        return Err("No INI section header was found.".into());
    }
    Ok(())
}

fn validate(format: &str, content: &str) -> std::result::Result<(), String> {
    match format {
        "ini" => validate_ini(content),
        "json" => serde_json::from_str::<serde_json::Value>(content)
            .map(|_| ())
            .map_err(|error| error.to_string()),
        "toml" => content
            .parse::<toml_edit::DocumentMut>()
            .map(|_| ())
            .map_err(|error| error.to_string()),
        "lua" => Err("Lua is read-only in Zero Mod Manager 0.7.".into()),
        _ => Err("Unsupported configuration format.".into()),
    }
}

fn line_diff(before: &str, after: &str) -> Vec<String> {
    let before = before.lines().collect::<Vec<_>>();
    let after = after.lines().collect::<Vec<_>>();
    let mut result = Vec::new();
    let count = before.len().max(after.len());
    for index in 0..count {
        match (before.get(index), after.get(index)) {
            (Some(left), Some(right)) if left == right => {}
            (Some(left), Some(right)) => {
                result.push(format!("- {:04} {left}", index + 1));
                result.push(format!("+ {:04} {right}", index + 1));
            }
            (Some(left), None) => result.push(format!("- {:04} {left}", index + 1)),
            (None, Some(right)) => result.push(format!("+ {:04} {right}", index + 1)),
            (None, None) => {}
        }
        if result.len() >= 400 {
            result.push("… diff truncated …".into());
            break;
        }
    }
    result
}

fn atomic_replace(path: &Path, format: &str, content: &[u8], token: &str) -> Result<()> {
    let temporary = path.with_extension(format!("{format}.zmm-new"));
    let displaced = path.with_extension(format!("{format}.zmm-old-{token}"));
    crate::package_transaction::protect_file(path)?;
    crate::package_transaction::protect_file(&temporary)?;
    crate::package_transaction::protect_file(&displaced)?;
    fs::write(&temporary, content)?;
    fs::rename(path, &displaced)?;
    if let Err(error) = fs::rename(&temporary, path) {
        let _ = fs::rename(&displaced, path);
        let _ = fs::remove_file(&temporary);
        return Err(error.into());
    }
    let _ = fs::remove_file(&displaced);
    Ok(())
}

/// Applies a stored profile layer after rechecking that its path, format and
/// contents remain inside the workbench contract. The caller keeps the returned
/// bytes to roll the entire profile switch back if a later operation fails.
pub fn apply_profile_content(game: &Path, path: &Path, content: &str) -> Result<Vec<u8>> {
    deployment::ensure_game_stopped()?;
    let canonical = allowed_path(game, path)?;
    let current = read_document(&canonical)?;
    if !current.writable {
        return Err(AppError::Other(
            "A stored profile patch points to a read-only format.".into(),
        ));
    }
    validate(&current.format, content).map_err(|problem| {
        AppError::Other(format!("A stored profile patch is invalid: {problem}"))
    })?;
    let original = fs::read(&canonical)?;
    if original != content.as_bytes() {
        atomic_replace(
            &canonical,
            &current.format,
            content.as_bytes(),
            &Uuid::new_v4().to_string(),
        )?;
    }
    Ok(original)
}

pub fn restore_profile_content(game: &Path, path: &Path, content: &[u8]) -> Result<()> {
    let canonical = allowed_path(game, path)?;
    let format = format_for(&canonical)
        .map(|value| value.0)
        .ok_or_else(|| AppError::Other("A profile rollback path is no longer supported.".into()))?;
    atomic_replace(&canonical, format, content, &Uuid::new_v4().to_string())
}

pub fn preview(game: &Path, path: &Path, content: &str) -> Result<ConfigChangePreview> {
    let current = read(game, path)?;
    let validation = validate(&current.format, content);
    Ok(ConfigChangePreview {
        path: current.path,
        format: current.format,
        before_sha256: current.sha256,
        after_sha256: sha256_bytes(content.as_bytes()),
        diff: line_diff(&current.content, content),
        valid: validation.is_ok(),
        problem: validation.err(),
    })
}

pub fn apply(
    conn: &Connection,
    game: &Path,
    data_dir: &Path,
    path: &Path,
    content: &str,
    expected_sha256: &str,
) -> Result<ConfigPatchRecord> {
    deployment::ensure_game_stopped()?;
    let canonical = allowed_path(game, path)?;
    let current = read_document(&canonical)?;
    if !current.writable {
        return Err(AppError::Other(
            "That format is read-only in the workbench.".into(),
        ));
    }
    if current.sha256 != expected_sha256 {
        return Err(AppError::Other(
            "The file changed after the preview. Reload it before saving.".into(),
        ));
    }
    if let Err(problem) = validate(&current.format, content) {
        return Err(AppError::Other(format!(
            "The edited configuration is invalid: {problem}"
        )));
    }
    let profile_id = conn
        .query_row("SELECT id FROM profiles WHERE is_active=1", [], |row| {
            row.get::<_, String>(0)
        })
        .optional()?
        .ok_or_else(|| AppError::Other("No active profile exists.".into()))?;
    let id = Uuid::new_v4().to_string();
    let backup = data_dir
        .join("config-backups")
        .join(&id)
        .join(canonical.file_name().unwrap_or_default());
    if let Some(parent) = backup.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(&canonical, &backup)?;
    atomic_replace(&canonical, &current.format, content.as_bytes(), &id)?;
    let created_at = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO config_patches(id,profile_id,path,format,patch_json,backup_path,created_at) VALUES(?1,?2,?3,?4,?5,?6,?7)",
        params![id, profile_id, canonical.display().to_string(), current.format, serde_json::json!({"content": content}).to_string(), backup.display().to_string(), created_at],
    )?;
    Ok(ConfigPatchRecord {
        id,
        profile_id,
        path: canonical.display().to_string(),
        format: current.format,
        backup_path: Some(backup.display().to_string()),
        created_at,
    })
}

pub fn history(conn: &Connection) -> Result<Vec<ConfigPatchRecord>> {
    let mut statement = conn.prepare("SELECT id,profile_id,path,format,backup_path,created_at FROM config_patches ORDER BY created_at DESC LIMIT 100")?;
    let rows = statement
        .query_map([], |row| {
            Ok(ConfigPatchRecord {
                id: row.get(0)?,
                profile_id: row.get(1)?,
                path: row.get(2)?,
                format: row.get(3)?,
                backup_path: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn rollback(conn: &Connection, game: &Path, patch_id: &str) -> Result<()> {
    deployment::ensure_game_stopped()?;
    let (path, backup) = conn
        .query_row(
            "SELECT path,backup_path FROM config_patches WHERE id=?1",
            [patch_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)),
        )
        .optional()?
        .ok_or_else(|| AppError::Other("That configuration checkpoint no longer exists.".into()))?;
    let target = allowed_path(game, Path::new(&path))?;
    let backup = backup
        .map(PathBuf::from)
        .filter(|path| path.is_file())
        .ok_or_else(|| AppError::Other("The backup for that checkpoint is missing.".into()))?;
    fs::copy(backup, target)?;
    conn.execute("DELETE FROM config_patches WHERE id=?1", [patch_id])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_supported_text_formats() {
        assert!(validate("ini", "[SystemSettings]\nr.Test=1\n").is_ok());
        assert!(validate("json", "{\"enabled\":true}").is_ok());
        assert!(validate("toml", "enabled = true").is_ok());
        assert!(validate("lua", "return true").is_err());
    }

    #[test]
    fn diff_never_silently_hides_a_changed_line() {
        assert_eq!(line_diff("a\nb", "a\nc"), vec!["- 0002 b", "+ 0002 c"]);
    }
}
