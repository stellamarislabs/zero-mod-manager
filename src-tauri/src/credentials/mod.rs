//! One-time retirement of this application's old Nexus credential only.
use crate::error::{AppError, Result};
use rusqlite::Connection;

pub fn retire(conn: &Connection) -> Result<()> {
    if crate::database::get_setting(conn, "nexus_integration_retired")?.as_deref() == Some("true") {
        return Ok(());
    }
    for key in [
        "nexus_api_key",
        "nexus_account_name",
        "nexus_premium",
        "nexus_auto_update_check",
    ] {
        crate::database::delete_setting(conn, key)?;
    }
    // Never access upstream ZCOM's or Vortex's credential.
    let entry = keyring::Entry::new("app.zeromodmanager.desktop", "nexus-api-key")
        .map_err(|error| AppError::Other(format!("Retired credential cleanup: {error}")))?;
    match entry.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => (),
        Err(error) => {
            return Err(AppError::Other(format!(
                "Retired credential cleanup: {error}"
            )))
        }
    }
    crate::database::set_setting(conn, "nexus_integration_retired", "true")
}
