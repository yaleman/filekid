//! This module contains the browse endpoint, which allows users to browse the files on the server.

use dropshot::{Body, Path};

use crate::{prelude::*, FileKid};

#[derive(Deserialize, JsonSchema, Clone)]
/// The path to browse.
pub struct GetPath {
    /// The server path.
    pub server_path: String,
    /// The file path.
    pub filepath: Vec<String>,
}

#[endpoint {
    method = GET,
    path = r#"/browse/{server_path}/{filepath:.*}"#,
    unpublished = true
}]
pub async fn get_file(
    rqctx: RequestContext<FileKid>,
    path: Path<GetPath>,
) -> Result<Response<Body>, HttpError> {
    let path = path.into_inner();

    if path.filepath.is_empty() {
        return browse(rqctx, path).await;
    }

    let server_config = match rqctx.context().config.server_paths.get(&path.server_path) {
        None => {
            return Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::empty())?);
        }
        Some(p) => p,
    };

    let full_path = server_config
        .path
        .join(path.filepath.clone().join("/"))
        .canonicalize()
        .map_err(|e| {
            HttpError::for_internal_error(format!(
                "Failed to canonicalize path: {:?} - {}",
                path.filepath, e
            ))
        })?;

    if !full_path.starts_with(&server_config.path.canonicalize().map_err(|e| {
        HttpError::for_internal_error(format!(
            "Failed to canonicalize path: {:?} - {}",
            path.filepath, e
        ))
    })?) {
        return Ok(Response::builder()
            .status(StatusCode::FORBIDDEN)
            .body(Body::empty())?);
    }

    let file =
        std::fs::read(&full_path).map_err(|e| HttpError::for_internal_error(e.to_string()))?;

    // guess the content-type based on the filename
    let content_type = mime_guess::from_path(&full_path)
        .first_or_octet_stream()
        .to_string();

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(http::header::CONTENT_TYPE, content_type)
        // .header(
        //     http::header::CONTENT_DISPOSITION,
        //     format!(
        //         "attachment; filename={}",
        //         full_path.file_name().unwrap().to_string_lossy()
        //     ),
        // )
        .body(file.into())?)
}

/// Browse the files in a server path.
pub async fn browse(
    rqctx: RequestContext<FileKid>,
    path: GetPath,
) -> Result<Response<Body>, HttpError> {
    let path = path.server_path.clone();
    let filepath = match rqctx.context().config.server_paths.get(&path) {
        None => {
            return Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::empty())?);
        }
        Some(p) => p,
    };

    let mut body = format!("<head><html><h1>FileKid ({})</h1>\n", path);

    // get the list of files in the path
    let entries = std::fs::read_dir(&filepath.path)
        .map_err(|e| HttpError::for_internal_error(e.to_string()))?
        .map(|entry| {
            entry
                .map_err(|e| HttpError::for_internal_error(format!("failed to read path: {:?}", e)))
                .and_then(|entry| {
                    entry
                        .file_name()
                        .into_string()
                        .map_err(|_| HttpError::for_internal_error("Invalid filename".to_string()))
                })
        })
        .collect::<Result<Vec<String>, HttpError>>()?;

    for entry in entries {
        body.push_str(&format!(
            "<a href=\"/browse/{}/{}\">{}</a><br>\n",
            &path, entry, entry
        ));
    }

    body.push_str("</html></head>");

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(http::header::CONTENT_TYPE, "text/html")
        .body(body.into())?)
}
