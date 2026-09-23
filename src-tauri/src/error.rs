use serde::Serialize;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Star Wars: Zero Company was not found. Locate the game in Settings.")]
    GameNotFound,
    #[error("That folder is not a valid Zero Company installation: {0}")]
    InvalidGamePath(String),
    #[error("The Steam manifest could not be read: {0}")]
    SteamManifestInvalid(String),
    #[error("The selected mod payload is not recognized.")]
    ModNotRecognized,
    #[error("The archive is unsafe: {0}")]
    UnsafeArchive(String),
    #[error("The mod is incomplete. Missing: {0}")]
    MissingIoStoreComponent(String),
    #[error("UE4SS is not installed or its layout is incomplete.")]
    Ue4ssNotFound,
    #[error("That archive does not contain a UE4SS runtime. Expected dwmapi.dll next to a ue4ss folder.")]
    Ue4ssPackageNotRecognized,
    #[error("A different file already exists at {0}. It was not overwritten.")]
    DeploymentConflict(PathBuf),
    /// The literal prefix is part of the contract with the interface, which
    /// recognises this failure to offer the override that resolves it.
    #[error("A managed file changed outside Zero Mod Manager: {0}")]
    ChecksumMismatch(PathBuf),
    #[error("The installation preview expired. Inspect the mod again.")]
    PreviewExpired,
    #[error("The proposed load order is invalid: {0}")]
    InvalidLoadOrder(String),
    #[error("Close Star Wars: Zero Company before changing managed files. The current operation was not started.")]
    GameRunning,
    #[error("The compatibility catalog could not be trusted: {0}")]
    CatalogUntrusted(String),
    #[error("This archive needs the 7-Zip command-line tool, and none was found. Install 7-Zip, or point Settings → Archive tool at your own 7z.exe.")]
    SevenZipNotFound,
    #[error("Network request failed: {0}")]
    Network(String),
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("File operation failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid data: {0}")]
    Json(#[from] serde_json::Error),
    #[error("ZIP archive error: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("{0}")]
    Other(String),
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

pub type Result<T> = std::result::Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;

    /// The interface recognises this failure by its opening words in order to
    /// offer the override that resolves it, so the prefix is part of the
    /// contract rather than incidental wording. `isChangedFileError` in
    /// `src/services/backend.ts` matches the same text.
    #[test]
    fn the_changed_file_message_keeps_the_prefix_the_interface_matches() {
        let message =
            AppError::ChecksumMismatch(PathBuf::from("/game/mod/registry.txt")).to_string();
        assert!(
            message.starts_with("A managed file changed outside Zero Mod Manager:"),
            "{message}"
        );
        assert!(message.contains("registry.txt"), "{message}");
    }
}
