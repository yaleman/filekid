//! log configuration and setup module

use std::env;

use env_logger::{Builder, Target};
use log::LevelFilter;

/// Sets up logging
pub fn setup_logging(debug: bool, db_debug: bool) -> Result<(), log::SetLoggerError> {
    let mut builder = Builder::from_default_env();

    let level = if debug && env::var("RUST_LOG").is_err() {
        LevelFilter::Debug
    } else {
        LevelFilter::Info
    };

    builder.filter_level(level);

    // h2 is pretty noisy
    builder.filter(Some("h2::codec"), LevelFilter::Warn);
    builder.filter(Some("h2::proto"), LevelFilter::Info);

    if !db_debug {
        // We don't always want to see the SQL queries in the logs
        builder.filter(Some("sea_orm::driver::sqlx_sqlite"), LevelFilter::Warn);
        builder.filter(Some("sqlx::query"), LevelFilter::Warn);
    }

    builder.filter(Some("tracing::span"), LevelFilter::Warn);
    builder.target(Target::Stdout);

    #[cfg(not(test))]
    {
        builder.try_init()
    }

    #[cfg(test)]
    {
        if let Err(err) = builder.try_init() {
            use tracing::debug;
            debug!("Error init logging: {:?}", err);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::setup_logging;

    #[test]
    fn test_setup_logging() {
        let test1 = setup_logging(false, true);
        dbg!(&test1);
        assert!(test1.is_ok());
        // it'll probably throw an error because we're trying to re-init the logger, but we're in test so it's OK.
        let test2 = setup_logging(true, true);
        dbg!(&test2);
        assert!(test2.is_ok());

        let test3 = setup_logging(true, false);
        dbg!(&test3);
        assert!(test3.is_ok());
    }
}
