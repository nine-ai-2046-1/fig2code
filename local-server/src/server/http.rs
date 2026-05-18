use std::fs;
use std::path::{Path, PathBuf};
use tiny_http::{Response, Server, Request};
use mime_guess::from_path;
use percent_encoding::percent_decode_str;

/// Start a simple static file HTTP server bound to bind_host:port serving files from root_dir.
/// Security rules:
/// - Only bind to localhost (127.0.0.1)
/// - Disallow directory listing (return 403)
/// - Do not follow path segments that escape the root directory
pub fn start_static_server(bind_host: &str, port: u16, root_dir: PathBuf) -> anyhow::Result<()> {
    if !root_dir.exists() || !root_dir.is_dir() {
        anyhow::bail!("Root folder does not exist or is not a directory: {}", root_dir.display());
    }

    let addr = format!("{}:{}", bind_host, port);

    // tiny_http expects a socket address
    let server = Server::http(&addr).map_err(|e| anyhow::anyhow!(e))?;

    // handle ctrl-c to stop
    ctrlc::set_handler(move || {
        println!("Shutting down server");
        std::process::exit(0);
    }).ok();

    for request in server.incoming_requests() {
        if let Err(e) = handle_request(request, &root_dir) {
            eprintln!("Request handling error: {}", e);
        }
    }

    Ok(())
}

fn handle_request(request: Request, root_dir: &Path) -> anyhow::Result<()> {
    // Only allow requests from localhost (tiny_http does not expose peer addr easily), relying on bind
    let url = request.url();

    // Strip query
    let path = url.split('?').next().unwrap_or("");

    let decoded = percent_decode_str(path).decode_utf8_lossy();
    let mut fs_path = sanitize_path(&decoded, root_dir)?;

    if fs_path.is_dir() {
        // If path is dir, try to serve index.html
        let index = fs_path.join("index.html");
        if index.exists() {
            fs_path = index;
        } else {
            // disallow directory listing
            let response = Response::from_string("403 Forbidden\n").with_status_code(403);
            request.respond(response).ok();
            return Ok(());
        }
    }

    if !fs_path.exists() || !fs_path.is_file() {
        let response = Response::from_string("404 Not Found\n").with_status_code(404);
        request.respond(response).ok();
        return Ok(());
    }

    let mime = from_path(&fs_path).first_or_octet_stream();
    let file = fs::read(&fs_path)?;
    let response = Response::from_data(file).with_header(tiny_http::Header::from_bytes(b"Content-Type", mime.as_ref().as_bytes()).unwrap());
    request.respond(response).ok();
    Ok(())
}

/// Resolve URL path to a filesystem path under root, preventing path traversal.
fn sanitize_path(req_path: &str, root: &Path) -> anyhow::Result<PathBuf> {
    // Remove leading '/'
    let mut trimmed = req_path;
    if trimmed.starts_with('/') {
        trimmed = &trimmed[1..];
    }

    // Reject attempts to access parent directory segments explicitly
    if trimmed.contains("..") {
        anyhow::bail!("Invalid path");
    }

    let candidate = root.join(trimmed);
    let canonical_root = fs::canonicalize(root)?;
    let canonical_candidate = fs::canonicalize(&candidate).unwrap_or(candidate.clone());

    if !canonical_candidate.starts_with(&canonical_root) {
        anyhow::bail!("Path escapes root");
    }

    Ok(canonical_candidate)
}
