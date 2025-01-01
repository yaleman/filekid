//! Web session store things

use etcetera::app_strategy::Xdg;
use etcetera::{AppStrategy, AppStrategyArgs};
use tower_sessions::cookie::time::Duration;
use tower_sessions::cookie::SameSite;
use tower_sessions::{session_store::ExpiredDeletion, Expiry, SessionManagerLayer};

use tower_sessions_sqlx_store::sqlx::SqlitePool;
use tower_sessions_sqlx_store::SqliteStore;
use tracing::{debug, info};

use crate::error::Error;

/// Returns a path to the database file, creating the directory if it doesn't exist
async fn db_dir() -> Result<String, Error> {
    let app_strategy = Xdg::new(AppStrategyArgs {
        top_level_domain: "com".to_string(),
        author: "Terminal Outcomes".to_string(),
        app_name: "filekid".to_string(),
    })
    .map_err(|err| {
        Error::Configuration(format!(
            "Couldn't identify way of generating a database dir! Error was: {}",
            err
        ))
    })?;

    if !app_strategy.data_dir().exists() {
        info!(
            "Creating DB data dir: {}",
            app_strategy.data_dir().display()
        );
        tokio::fs::create_dir(app_strategy.data_dir())
            .await
            .map_err(|err| {
                Error::Configuration(format!(
                    "Couldn't create data dir {}! Error was: {}",
                    app_strategy.data_dir().display(),
                    err
                ))
            })?;
    }

    Ok(format!(
        "sqlite://{}/filekid.sqlite?mode=rwc",
        app_strategy.data_dir().to_string_lossy()
    ))
}
pub(crate) type DeletionTask =
    tokio::task::JoinHandle<Result<(), tower_sessions::session_store::Error>>;

#[cfg(test)]
pub(crate) const SQLITE_MEMORY: &str = "sqlite::memory:";

/// Returns a session store and a task that will delete expired sessions periodically
pub(crate) async fn build(
    database_path: Option<String>,
) -> Result<(DeletionTask, SessionManagerLayer<SqliteStore>), Error> {
    let database_path = match database_path {
        Some(val) => val,
        None => db_dir().await?,
    };
    debug!("Sqlite database path: {}", database_path);

    let pool = SqlitePool::connect(&database_path)
        .await
        .map_err(|err| Error::Database(err.to_string()))?;
    let session_store = SqliteStore::new(pool);
    session_store
        .migrate()
        .await
        .map_err(|err| Error::Database(err.to_string()))?;

    let deletion_task = tokio::task::spawn(
        session_store
            .clone()
            .continuously_delete_expired(tokio::time::Duration::from_secs(60)),
    );

    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(true)
        .with_http_only(true) // is the default but it's nice to explicitly call it out
        .with_path("/")
        .with_same_site(SameSite::Lax)
        .with_expiry(Expiry::OnInactivity(Duration::seconds(3600)));

    Ok((deletion_task, session_layer))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_build() {
        build(Some(SQLITE_MEMORY.to_string()))
            .await
            .expect("Failed to build session store");
    }
}
