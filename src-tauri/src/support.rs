use crate::{
    error::{AppError, Result},
    models::{SupportBundlePreview, SupportBundleReport},
};
use std::{fs, io::Write, path::Path};
use zip::write::SimpleFileOptions;

pub fn preview(has_profile: bool, has_ue4ss_log: bool) -> SupportBundlePreview {
    let mut sections = vec![
        "Application and platform version".into(),
        "Game, launcher and runtime fingerprint".into(),
        "Diagnostics and compatibility report".into(),
        "Recent Zero Mod Manager activity and logs".into(),
    ];
    if has_profile {
        sections.push("Active profile lockfile".into());
    }
    if has_ue4ss_log {
        sections.push("Recent UE4SS log excerpt".into());
    }
    SupportBundlePreview {
        estimated_files: sections.len() + 1,
        sections,
        redactions: vec![
            "Home and user directory replaced with ~".into(),
            "Nexus API keys and authorization values removed".into(),
            "Machine-specific environment data omitted".into(),
        ],
        includes_save_data: false,
    }
}

fn redact(mut text: String) -> (String, usize) {
    let mut count = 0;
    if let Some(home) = dirs::home_dir() {
        let value = home.display().to_string();
        if text.contains(&value) {
            text = text.replace(&value, "~");
            count += 1;
        }
        let slash = value.replace('\\', "/");
        if text.contains(&slash) {
            text = text.replace(&slash, "~");
            count += 1;
        }
    }
    for marker in ["apikey", "api_key", "authorization", "bearer"] {
        let mut output = Vec::new();
        for line in text.lines() {
            if line.to_ascii_lowercase().contains(marker) {
                output.push("[credential redacted]");
                count += 1;
            } else {
                output.push(line);
            }
        }
        text = output.join("\n");
    }
    (text, count)
}

fn limited_text(path: &Path, limit: usize) -> Option<String> {
    let bytes = fs::read(path).ok()?;
    let start = bytes.len().saturating_sub(limit);
    Some(String::from_utf8_lossy(&bytes[start..]).into_owned())
}

pub struct BundleContent<'a> {
    pub application: &'a serde_json::Value,
    pub diagnostics: &'a str,
    pub compatibility: &'a serde_json::Value,
    pub profile_lock: Option<&'a serde_json::Value>,
    pub activity: &'a serde_json::Value,
    pub sessions: &'a serde_json::Value,
    pub application_log: &'a Path,
    pub ue4ss_log: Option<&'a Path>,
}

pub fn create(path: &Path, content: BundleContent<'_>) -> Result<SupportBundleReport> {
    if path
        .extension()
        .is_none_or(|extension| !extension.eq_ignore_ascii_case("zip"))
    {
        return Err(AppError::Other(
            "Support bundles must be saved as a .zip file.".into(),
        ));
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension("zip.partial");
    let file = fs::File::create(&temporary)?;
    let mut writer = zip::ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let mut files = 0usize;
    let mut redactions = 0usize;
    let mut add = |name: &str, value: String| -> Result<()> {
        let (value, changed) = redact(value);
        redactions += changed;
        writer.start_file(name, options)?;
        writer.write_all(value.as_bytes())?;
        files += 1;
        Ok(())
    };
    add(
        "bundle-manifest.json",
        serde_json::to_string_pretty(&serde_json::json!({
            "schemaVersion": 1,
            "createdAt": chrono::Utc::now().to_rfc3339(),
            "saveDataIncluded": false,
            "uploadedAutomatically": false
        }))?,
    )?;
    add(
        "application.json",
        serde_json::to_string_pretty(content.application)?,
    )?;
    add("diagnostics.txt", content.diagnostics.to_string())?;
    add(
        "compatibility.json",
        serde_json::to_string_pretty(content.compatibility)?,
    )?;
    add(
        "activity.json",
        serde_json::to_string_pretty(content.activity)?,
    )?;
    add(
        "launch-sessions.json",
        serde_json::to_string_pretty(content.sessions)?,
    )?;
    if let Some(profile) = content.profile_lock {
        add(
            "active-profile.lock.json",
            serde_json::to_string_pretty(profile)?,
        )?;
    }
    if let Some(log) = limited_text(content.application_log, 512 * 1024) {
        add("logs/zero-mod-manager.jsonl", log)?;
    }
    if let Some(log) = content
        .ue4ss_log
        .and_then(|path| limited_text(path, 256 * 1024))
    {
        add("logs/ue4ss-excerpt.log", log)?;
    }
    writer.finish()?;
    if path.exists() {
        fs::remove_file(path)?;
    }
    fs::rename(&temporary, path)?;
    Ok(SupportBundleReport {
        path: path.display().to_string(),
        files,
        redactions_applied: redactions,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credentials_are_removed_from_support_text() {
        let (text, count) = redact("ok\napiKey=secret\nauthorization: bearer secret".into());
        assert!(!text.contains("secret"));
        assert!(count >= 2);
    }
}
