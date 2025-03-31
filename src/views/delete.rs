//! Delete-file related things

use super::{check_login, prelude::*};

use crate::fs::fs_from_serverpath;
use askama::Template;
use axum::extract::{Query, State};
use axum::response::Redirect;
use axum::Form;

#[derive(Debug, Deserialize, Template)]
#[template(path = "delete_form.html")]
pub(crate) struct DeletePage {
    server_path: String,
    key: String,
    username: String,
}

impl DeletePage {
    fn parent_path(&self) -> String {
        let path = self.key.clone();
        let mut path = path.split('/').collect::<Vec<&str>>();
        path.pop();
        path.join("/")
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct DeleteQuery {
    server_path: String,
    key: String,
}
impl DeleteQuery {
    fn parent_path(&self) -> String {
        let path = self.key.clone();
        let mut path = path.split('/').collect::<Vec<&str>>();
        path.pop();
        path.join("/")
    }
}

pub(crate) async fn delete_file_get(
    State(state): State<WebState>,
    Query(query): Query<DeleteQuery>,
    claims: Option<OidcClaims<EmptyAdditionalClaims>>,
) -> Result<String, Error> {
    let user = check_login(claims)?;

    let server_reader = state.configuration.read().await;

    let server_path_object = match server_reader.server_paths.get(&query.server_path) {
        None => {
            error!("Couldn't find server path {}", query.server_path);
            return Err(Error::NotFound(query.server_path));
        }
        Some(p) => p,
    };

    let filekidfs = fs_from_serverpath(server_path_object)?;
    if !filekidfs.exists(&query.key)? {
        error!("Couldn't find file path {:?}", query.key);
        return Err(Error::NotFound(query.key));
    }

    Ok(DeletePage {
        server_path: query.server_path,
        key: query.key,
        username: user.username(),
    }
    .render()?)
}

pub(crate) async fn delete_file_post(
    State(state): State<WebState>,
    Form(form): Form<DeleteQuery>,
) -> Result<impl IntoResponse, Error> {
    let server_reader = state.configuration.read().await;

    let server_path_object = match server_reader.server_paths.get(&form.server_path) {
        None => {
            error!("Couldn't find server path {}", form.server_path);
            return Err(Error::NotFound(form.server_path));
        }
        Some(p) => p,
    };

    let filekidfs = fs_from_serverpath(server_path_object)?;

    if !filekidfs.exists(&form.key)? {
        error!("Couldn't find file path {:?}", form.key);
        return Err(Error::NotFound(form.key));
    }

    filekidfs.delete_file(&form.key)?;

    Ok(Redirect::to(&format!(
        "{}/{}/{}",
        Urls::Browse.as_ref(),
        form.server_path,
        form.parent_path()
    )))
}
