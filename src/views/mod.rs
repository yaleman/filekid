//! Web views for FileKid.

pub mod browse;

use dropshot::Body;

use crate::{prelude::*, FileKid};

#[endpoint {
    method = GET,
    path = "/",
    unpublished = true,

}]
pub async fn home(
    rqctx: RequestContext<FileKid>,
    // _path: Path<AllPath>,
) -> Result<Response<Body>, HttpError> {
    let mut body = "<head><html><h1>FileKid</h1>\n".to_string();

    for (server, server_config) in rqctx.context().config.server_paths.iter() {
        body.push_str(&format!(
            "<a href=\"/browse/{}\">{}</a> ({})<br>\n",
            server,
            server,
            server_config.path.display()
        ));
    }

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(http::header::CONTENT_TYPE, "text/html")
        .body(body.into())?)
}
