//! Web views for FileKid.

pub mod browse;
pub mod oidc;
pub mod prelude;

use std::collections::HashMap;

use prelude::*;

use crate::ServerPath;

#[derive(Template)]
#[template(path = "index.html")]
pub(crate) struct HomePage {
    server_paths: HashMap<String, ServerPath>,
}

pub(crate) async fn home(State(state): State<WebState>) -> Result<HomePage, Error> {
    Ok(HomePage {
        server_paths: state.configuration.read().await.server_paths.clone(),
    })
}
