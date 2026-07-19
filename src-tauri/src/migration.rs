//! One-shot migration of user data from the pre-v0.2 app data directory.
//!
//! History: v0.2.0 changed the Tauri identifier from `com.joaog.claudia-rh`
//! to `io.github.johngabie.claudia-rh` (commit 6693e56), silently moving
//! `app_data_dir()` to a brand-new empty directory — v0.1 (source-build)
//! users "lost" their profile on update. Since v0.2.0 is the only public
//! binary release, its identifier is the installed base's data path and was
//! kept — and is now frozen forever (see the identifier_is_frozen test
//! below). This migration rescues data from pre-v0.2 source builds.

use std::path::Path;

// The pre-v0.2 identifier (source builds only — v0.1 never shipped binaries).
const LEGACY_DIR_NAME: &str = "com.joaog.claudia-rh";

/// Plain data files copied only when missing at the destination.
const DATA_FILES: &[&str] = &[
    "candidate_base.yaml",
    "search_variants.yaml",
    "strategy.md",
    "disparo.json",
    "modo_autonomo.json",
    "notif.json",
];

/// Runs before db::init. Safe to call on every startup: it only acts when a
/// legacy directory exists next to the new one, and never overwrites data the
/// user already created in the new location.
pub fn migrate_legacy_data_dir(new_dir: &Path) {
    let Some(parent) = new_dir.parent() else { return };
    let old_dir = parent.join(LEGACY_DIR_NAME);
    if !old_dir.exists() || old_dir == new_dir {
        return;
    }

    for file in DATA_FILES {
        let src = old_dir.join(file);
        let dst = new_dir.join(file);
        if src.exists() && !dst.exists() {
            if let Err(e) = std::fs::copy(&src, &dst) {
                eprintln!("[migration] failed to copy {file}: {e}");
            } else {
                eprintln!("[migration] migrated {file} from legacy data dir");
            }
        }
    }

    migrate_database(&old_dir, new_dir);
}

/// The database needs more care than plain files: db::init may already have
/// created an empty DB in the new dir on a previous v0.2 run (or tests may
/// have written throwaway rows). We replace the new DB only when it has no
/// meaningful user data, backing it up first.
fn migrate_database(old_dir: &Path, new_dir: &Path) {
    let old_db = old_dir.join("claudia_rh.db");
    if !old_db.exists() {
        return;
    }
    let new_db = new_dir.join("claudia_rh.db");

    if new_db.exists() {
        let new_rows = count_user_rows(&new_db);
        let old_rows = count_user_rows(&old_db);
        // Keep the new DB when the user already has real data in it, or when
        // the legacy DB has nothing worth migrating.
        if new_rows > 0 || old_rows == 0 {
            return;
        }
        let backup = new_dir.join("claudia_rh.db.pre-migration");
        if let Err(e) = std::fs::rename(&new_db, &backup) {
            eprintln!("[migration] failed to back up new DB, aborting DB migration: {e}");
            return;
        }
        // Stale sidecar files from the renamed DB must not be reused.
        let _ = std::fs::remove_file(new_dir.join("claudia_rh.db-wal"));
        let _ = std::fs::remove_file(new_dir.join("claudia_rh.db-shm"));
    }

    if let Err(e) = std::fs::copy(&old_db, &new_db) {
        eprintln!("[migration] failed to copy database: {e}");
        return;
    }
    // Copy WAL/SHM so uncheckpointed writes from the legacy install survive.
    for ext in ["-wal", "-shm"] {
        let src = old_dir.join(format!("claudia_rh.db{ext}"));
        if src.exists() {
            let _ = std::fs::copy(&src, new_dir.join(format!("claudia_rh.db{ext}")));
        }
    }
    eprintln!("[migration] migrated database from legacy data dir");
}

/// Total rows across the tables that represent real user activity.
fn count_user_rows(db: &Path) -> i64 {
    let Ok(conn) = rusqlite::Connection::open(db) else { return 0 };
    ["vagas", "candidaturas", "pendencias"]
        .iter()
        .map(|t| {
            conn.query_row(&format!("SELECT COUNT(*) FROM {t}"), [], |r| r.get::<_, i64>(0))
                .unwrap_or(0)
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_root(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "claudia-migration-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn make_db(path: &Path, vagas: i64) {
        let conn = rusqlite::Connection::open(path).unwrap();
        conn.execute_batch(include_str!("db/schema.sql")).unwrap();
        for i in 0..vagas {
            conn.execute(
                "INSERT INTO vagas (titulo, empresa, plataforma, url, descoberta_em, status)
                 VALUES ('t', 'e', 'p', 'https://x/' || ?1, datetime('now'), 'descoberta')",
                [i],
            )
            .unwrap();
        }
    }

    #[test]
    fn migrates_files_and_db_into_fresh_dir() {
        let root = temp_root("fresh");
        let old = root.join(LEGACY_DIR_NAME);
        let new = root.join("io.github.johngabie.claudia-rh");
        std::fs::create_dir_all(&old).unwrap();
        std::fs::create_dir_all(&new).unwrap();
        std::fs::write(old.join("candidate_base.yaml"), "dados_pessoais:\n  nome_completo: X\n").unwrap();
        make_db(&old.join("claudia_rh.db"), 3);

        migrate_legacy_data_dir(&new);

        assert!(new.join("candidate_base.yaml").exists());
        assert_eq!(count_user_rows(&new.join("claudia_rh.db")), 3);
    }

    #[test]
    fn never_overwrites_existing_new_data() {
        let root = temp_root("existing");
        let old = root.join(LEGACY_DIR_NAME);
        let new = root.join("io.github.johngabie.claudia-rh");
        std::fs::create_dir_all(&old).unwrap();
        std::fs::create_dir_all(&new).unwrap();
        std::fs::write(old.join("candidate_base.yaml"), "old profile").unwrap();
        std::fs::write(new.join("candidate_base.yaml"), "new profile").unwrap();
        make_db(&old.join("claudia_rh.db"), 5);
        make_db(&new.join("claudia_rh.db"), 2); // user already has real v0.2 data

        migrate_legacy_data_dir(&new);

        assert_eq!(std::fs::read_to_string(new.join("candidate_base.yaml")).unwrap(), "new profile");
        assert_eq!(count_user_rows(&new.join("claudia_rh.db")), 2);
    }

    #[test]
    fn replaces_empty_new_db_with_backup() {
        let root = temp_root("emptydb");
        let old = root.join(LEGACY_DIR_NAME);
        let new = root.join("io.github.johngabie.claudia-rh");
        std::fs::create_dir_all(&old).unwrap();
        std::fs::create_dir_all(&new).unwrap();
        make_db(&old.join("claudia_rh.db"), 4);
        make_db(&new.join("claudia_rh.db"), 0); // empty DB created by a prior v0.2 run

        migrate_legacy_data_dir(&new);

        assert_eq!(count_user_rows(&new.join("claudia_rh.db")), 4);
        assert!(new.join("claudia_rh.db.pre-migration").exists());
    }

    /// The app identifier determines where user data lives (app_data_dir)
    /// AND the Windows installer upgrade path. Changing it silently strands
    /// every existing user's profile/DB in the old directory and breaks
    /// in-place updates. It already happened once (v0.2.0, commit 6693e56).
    /// NEVER change it. If you believe you must, you are wrong — see
    /// src/migration.rs for the cleanup the last change required.
    #[test]
    fn identifier_is_frozen() {
        let conf: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        assert_eq!(
            conf["identifier"].as_str(),
            Some("io.github.johngabie.claudia-rh"),
            "The Tauri identifier must NEVER change — it is the user data path \
             and the installer upgrade identity. See src/migration.rs."
        );
    }

    #[test]
    fn noop_without_legacy_dir() {
        let root = temp_root("nolegacy");
        let new = root.join("io.github.johngabie.claudia-rh");
        std::fs::create_dir_all(&new).unwrap();
        migrate_legacy_data_dir(&new);
        assert!(!new.join("candidate_base.yaml").exists());
    }
}
