use crate::{
    error::{AppError, Result},
    models::{CompatibilityIssue, CompatibilityReport, ManifestRelation, ModManifest},
};
use base64::{engine::general_purpose::STANDARD, Engine};
use chrono::Utc;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SignedEnvelope {
    payload: String,
    signature: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Catalog {
    schema_version: u32,
    version: String,
    issued_at: String,
    expires_at: String,
    rules: Vec<CatalogRule>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CatalogRule {
    id: String,
    subject_mod_id: String,
    target_mod_id: Option<String>,
    rule_type: String,
    severity: String,
    message: String,
    evidence_url: Option<String>,
}

fn verify(bytes: &[u8]) -> Result<(Catalog, String)> {
    let envelope: SignedEnvelope = serde_json::from_slice(bytes)?;
    let configured = option_env!("ZERO_MOD_MANAGER_CATALOG_PUBLIC_KEY")
        .ok_or_else(|| AppError::CatalogUntrusted("no release public key is configured".into()))?;
    let key_bytes = STANDARD
        .decode(configured)
        .map_err(|_| AppError::CatalogUntrusted("the embedded public key is invalid".into()))?;
    let key: [u8; 32] = key_bytes.try_into().map_err(|_| {
        AppError::CatalogUntrusted("the embedded public key has the wrong length".into())
    })?;
    let verifying_key = VerifyingKey::from_bytes(&key).map_err(|_| {
        AppError::CatalogUntrusted("the embedded public key could not be read".into())
    })?;
    let signature_bytes = STANDARD.decode(&envelope.signature).map_err(|_| {
        AppError::CatalogUntrusted("the catalog signature is invalid base64".into())
    })?;
    let signature = Signature::from_slice(&signature_bytes).map_err(|_| {
        AppError::CatalogUntrusted("the catalog signature has the wrong length".into())
    })?;
    verifying_key
        .verify(envelope.payload.as_bytes(), &signature)
        .map_err(|_| AppError::CatalogUntrusted("signature verification failed".into()))?;
    let catalog: Catalog = serde_json::from_str(&envelope.payload)?;
    if catalog.schema_version != 1 {
        return Err(AppError::CatalogUntrusted(format!(
            "schema {} is not supported",
            catalog.schema_version
        )));
    }
    let expires = chrono::DateTime::parse_from_rfc3339(&catalog.expires_at)
        .map_err(|_| AppError::CatalogUntrusted("expiry timestamp is invalid".into()))?;
    let issued = chrono::DateTime::parse_from_rfc3339(&catalog.issued_at)
        .map_err(|_| AppError::CatalogUntrusted("issue timestamp is invalid".into()))?;
    if expires <= issued {
        return Err(AppError::CatalogUntrusted(
            "expiry must be later than issue time".into(),
        ));
    }
    if issued > Utc::now() + chrono::Duration::hours(24) {
        return Err(AppError::CatalogUntrusted(
            "issue timestamp is implausibly far in the future".into(),
        ));
    }
    if expires < Utc::now() {
        return Err(AppError::CatalogUntrusted(
            "the signed catalog has expired".into(),
        ));
    }
    let version = catalog.version.clone();
    Ok((catalog, version))
}

fn version_parts(value: &str) -> Vec<u64> {
    value
        .trim_start_matches(['v', 'V'])
        .split(['.', '-'])
        .map(|part| part.parse::<u64>().unwrap_or(0))
        .collect()
}

fn older_version(candidate: &str, trusted: &str) -> bool {
    let mut candidate = version_parts(candidate);
    let mut trusted = version_parts(trusted);
    let width = candidate.len().max(trusted.len());
    candidate.resize(width, 0);
    trusted.resize(width, 0);
    candidate < trusted
}

pub fn install(conn: &Connection, bytes: &[u8], cache_path: &Path) -> Result<String> {
    let (catalog, version) = verify(bytes)?;
    let previous = crate::database::get_setting(conn, "catalog_version")?;
    if previous
        .as_deref()
        .is_some_and(|old| older_version(&version, old))
    {
        return Err(AppError::CatalogUntrusted(format!(
            "version {version} is older than the trusted version {}",
            previous.unwrap_or_default()
        )));
    }
    let transaction = conn.unchecked_transaction()?;
    transaction.execute(
        "DELETE FROM compatibility_rules WHERE source='community-catalog'",
        [],
    )?;
    for rule in catalog.rules {
        transaction.execute(
            "INSERT INTO compatibility_rules(id,subject_mod_id,target_mod_id,rule_type,severity,source,evidence_url,message)
             VALUES(?1,?2,?3,?4,?5,'community-catalog',?6,?7)",
            params![rule.id, rule.subject_mod_id, rule.target_mod_id, rule.rule_type, rule.severity, rule.evidence_url, rule.message],
        )?;
    }
    transaction.commit()?;
    if let Some(parent) = cache_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(cache_path, bytes)?;
    crate::database::set_setting(conn, "catalog_version", &version)?;
    crate::database::set_setting(conn, "catalog_updated_at", &Utc::now().to_rfc3339())?;
    Ok(version)
}

pub fn load_cached(conn: &Connection, cache_path: &Path) -> Result<Option<String>> {
    if !cache_path.is_file() {
        return Ok(None);
    }
    install(conn, &fs::read(cache_path)?, cache_path).map(Some)
}

fn issue_status(severity: &str) -> String {
    match severity {
        "blocked" | "error" => "blocked",
        "warning" => "warning",
        _ => "unverified",
    }
    .into()
}

pub fn install_author_manifest(
    conn: &Connection,
    local_mod_id: &str,
    manifest: &ModManifest,
) -> Result<()> {
    let json = serde_json::to_string(manifest)?;
    conn.execute(
        "UPDATE mods SET manifest_id=?2,manifest_json=?3 WHERE id=?1",
        params![local_mod_id, manifest.id, json],
    )?;
    conn.execute(
        "DELETE FROM compatibility_rules WHERE source='author-manifest' AND subject_mod_id=?1",
        [&manifest.id],
    )?;
    let write = |kind: &str, severity: &str, relation: &ManifestRelation| -> Result<()> {
        let reason = relation.reason.clone().unwrap_or_else(|| match kind {
            "requires" => format!("Requires {}.", relation.id),
            "incompatible" => format!("Cannot be used with {}.", relation.id),
            _ => format!("Should load after {}.", relation.id),
        });
        conn.execute(
            "INSERT OR REPLACE INTO compatibility_rules(id,subject_mod_id,target_mod_id,rule_type,severity,source,evidence_url,message)
             VALUES(?1,?2,?3,?4,?5,'author-manifest',?6,?7)",
            params![format!("manifest:{}:{kind}:{}", manifest.id, relation.id), manifest.id, relation.id, kind, severity, relation.evidence_url, reason],
        )?;
        Ok(())
    };
    for relation in &manifest.dependencies {
        write("requires", "blocked", relation)?;
    }
    for relation in &manifest.incompatibilities {
        write("incompatible", "blocked", relation)?;
    }
    for relation in &manifest.load_after {
        write("load-after", "warning", relation)?;
    }
    Ok(())
}

pub fn report(conn: &Connection) -> Result<CompatibilityReport> {
    let enabled = crate::database::list_mods(conn)?
        .into_iter()
        .filter(|item| item.enabled)
        .map(|item| (item.id.clone(), item))
        .collect::<BTreeMap<_, _>>();
    let mut identities = BTreeMap::<String, String>::new();
    for item in enabled.values() {
        identities.insert(item.id.clone(), item.id.clone());
        if let Some(nexus) = item.nexus_mod_id {
            identities.insert(format!("nexus:{nexus}"), item.id.clone());
        }
        let manifest_id = conn.query_row(
            "SELECT manifest_id FROM mods WHERE id=?1",
            [&item.id],
            |row| row.get::<_, Option<String>>(0),
        )?;
        if let Some(manifest_id) = manifest_id {
            identities.insert(manifest_id, item.id.clone());
        }
    }
    let mut issues = Vec::new();
    for item in enabled.values() {
        if item.container_verification.as_deref() == Some("unavailable") {
            issues.push(CompatibilityIssue {
                id: format!("container-unverified:{}", item.id),
                status: "warning".into(),
                rule_type: "container-verification".into(),
                title: format!("{}: container not verified", item.name),
                detail: "Installed with your consent without retoc. Container integrity and package overlaps are unknown.".into(),
                source: "local-analysis".into(),
                evidence_url: None,
                member_ids: vec![item.id.clone()],
            });
        }
    }
    let current_build = crate::database::get_setting(conn, "last_game_build")?;
    let current_platform = std::env::consts::OS.to_ascii_lowercase();
    for item in enabled.values() {
        let raw = conn.query_row(
            "SELECT manifest_json FROM mods WHERE id=?1",
            [&item.id],
            |row| row.get::<_, Option<String>>(0),
        )?;
        let Some(manifest) = raw.and_then(|raw| serde_json::from_str::<ModManifest>(&raw).ok())
        else {
            continue;
        };
        if !manifest.platforms.is_empty()
            && !manifest
                .platforms
                .iter()
                .any(|value| value.eq_ignore_ascii_case(&current_platform))
        {
            issues.push(CompatibilityIssue {
                id: format!("environment:{}:platform", manifest.id),
                status: "blocked".into(),
                rule_type: "platform".into(),
                title: "Mod does not declare support for this platform".into(),
                detail: format!(
                    "{} declares support for {}.",
                    item.name,
                    manifest.platforms.join(", ")
                ),
                source: "author-manifest".into(),
                evidence_url: None,
                member_ids: vec![item.id.clone()],
            });
        }
        if let (Some(game), Some(build)) = (&manifest.game, current_build.as_deref()) {
            if !game.tested_builds.is_empty()
                && !game.tested_builds.iter().any(|tested| tested == build)
            {
                issues.push(CompatibilityIssue {
                    id: format!("environment:{}:build", manifest.id),
                    status: if game.strict == Some(true) {
                        "blocked"
                    } else {
                        "unverified"
                    }
                    .into(),
                    rule_type: "game-build".into(),
                    title: "Current game build is not declared compatible".into(),
                    detail: format!(
                        "{} was declared for build(s) {}; this installation is build {}.",
                        item.name,
                        game.tested_builds.join(", "),
                        build
                    ),
                    source: "author-manifest".into(),
                    evidence_url: None,
                    member_ids: vec![item.id.clone()],
                });
            }
        }
    }

    let mut file_rows = conn.prepare(
        "SELECT f.destination,group_concat(m.id),group_concat(m.name)
         FROM mod_files f JOIN mods m ON m.id=f.mod_id WHERE m.enabled=1
         GROUP BY lower(f.destination) HAVING count(DISTINCT m.id)>1",
    )?;
    for row in file_rows.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    })? {
        let (destination, ids, names) = row?;
        issues.push(CompatibilityIssue {
            id: format!("file:{}", destination.to_ascii_lowercase()),
            status: "blocked".into(),
            rule_type: "file-overlap".into(),
            title: "Two enabled mods own the same file".into(),
            detail: format!("{names} both deploy to {destination}."),
            source: "local-analysis".into(),
            evidence_url: None,
            member_ids: ids.split(',').map(str::to_string).collect(),
        });
    }

    let mut package_members: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (package, mod_id) in crate::database::package_members(conn)? {
        if enabled.contains_key(&mod_id) {
            package_members.entry(package).or_default().push(mod_id);
        }
    }
    let mut grouped: BTreeMap<Vec<String>, usize> = BTreeMap::new();
    for mut members in package_members
        .into_values()
        .filter(|members| members.len() > 1)
    {
        members.sort();
        members.dedup();
        *grouped.entry(members).or_default() += 1;
    }
    for (members, count) in grouped {
        issues.push(CompatibilityIssue {
            id: format!("package:{}", members.join(":")),
            status: "warning".into(),
            rule_type: "package-overlap".into(),
            title: "Packaged assets overlap".into(),
            detail: format!(
                "{} package path{} overlap. Load order decides the winner.",
                count,
                if count == 1 { "" } else { "s" }
            ),
            source: "local-analysis".into(),
            evidence_url: None,
            member_ids: members,
        });
    }

    let mut statement = conn.prepare(
        "SELECT id,subject_mod_id,target_mod_id,rule_type,severity,source,evidence_url,message FROM compatibility_rules ORDER BY id",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, String>(5)?,
            row.get::<_, Option<String>>(6)?,
            row.get::<_, String>(7)?,
        ))
    })?;
    for row in rows {
        let (id, subject, target, rule_type, severity, source, evidence, message) = row?;
        let Some(subject_local) = identities.get(&subject).cloned() else {
            continue;
        };
        let applies = match rule_type.as_str() {
            "requires" => target.as_ref().is_some_and(|id| !identities.contains_key(id)),
            "incompatible" => target.as_ref().is_some_and(|id| identities.contains_key(id)),
            "load-after" => target.as_ref().is_some_and(|target_id| {
                let subject_priority = enabled.get(&subject_local).and_then(|m| m.load_priority);
                let target_priority = identities.get(target_id).and_then(|local| enabled.get(local)).and_then(|m| m.load_priority);
                matches!((subject_priority, target_priority), (Some(subject), Some(target)) if subject <= target)
            }),
            "known-issue" => true,
            _ => false,
        };
        if applies {
            let mut members = vec![subject_local];
            if let Some(target) = target.and_then(|identity| identities.get(&identity).cloned()) {
                members.push(target);
            }
            issues.push(CompatibilityIssue {
                id,
                status: issue_status(&severity),
                rule_type: rule_type.clone(),
                title: match rule_type.as_str() {
                    "requires" => "Required mod is not enabled",
                    "incompatible" => "Known incompatible mods are enabled",
                    "load-after" => "Load-order recommendation is not satisfied",
                    _ => "Known compatibility note",
                }
                .into(),
                detail: message,
                source,
                evidence_url: evidence,
                member_ids: members,
            });
        }
    }
    let states = issues
        .iter()
        .map(|issue| issue.status.as_str())
        .collect::<BTreeSet<_>>();
    let status = if states.contains("blocked") {
        "blocked"
    } else if states.contains("warning") {
        "warning"
    } else if states.contains("unverified") {
        "unverified"
    } else {
        "ready"
    };
    Ok(CompatibilityReport {
        status: status.into(),
        generated_at: Utc::now().to_rfc3339(),
        catalog_state: crate::database::get_setting(conn, "catalog_version")?
            .map(|v| format!("trusted {v}"))
            .unwrap_or_else(|| "not configured".into()),
        issues,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn issue_severity_has_stable_product_states() {
        assert_eq!(issue_status("error"), "blocked");
        assert_eq!(issue_status("warning"), "warning");
        assert_eq!(issue_status("note"), "unverified");
    }

    #[test]
    fn catalog_rollback_comparison_is_numeric() {
        assert!(older_version("1.9", "1.10"));
        assert!(!older_version("1.10", "1.9"));
        assert!(!older_version("v2.0.0", "2"));
    }

    #[test]
    fn author_manifest_uses_stable_identity_for_dependency_rules() {
        let directory = tempdir().unwrap();
        let conn = crate::database::open(&directory.path().join("db.sqlite")).unwrap();
        conn.execute(
            "INSERT INTO mods(id,name,mod_type,installed_at,enabled) VALUES('local-random','Core','ue4ss',datetime('now'),1)", [],
        ).unwrap();
        let manifest = ModManifest {
            schema_version: 2,
            id: "suite.core".into(),
            name: "Core".into(),
            version: None,
            author: None,
            description: None,
            game: None,
            mod_types: vec!["ue4ss".into()],
            nexus: None,
            platforms: Vec::new(),
            launchers: Vec::new(),
            runtime: None,
            dependencies: vec![ManifestRelation {
                id: "suite.framework".into(),
                version: None,
                reason: Some("Framework is required.".into()),
                evidence_url: None,
            }],
            incompatibilities: Vec::new(),
            load_after: Vec::new(),
            config_schema: None,
        };
        install_author_manifest(&conn, "local-random", &manifest).unwrap();
        let result = report(&conn).unwrap();
        assert_eq!(result.status, "blocked");
        assert_eq!(result.issues[0].source, "author-manifest");
        assert_eq!(result.issues[0].member_ids, vec!["local-random"]);
    }
}
