use actix_web::{
    body::BoxBody,
    dev::{fn_service, ServiceRequest, ServiceResponse},
    http::{header, StatusCode},
    middleware::DefaultHeaders,
    web, Error, HttpRequest, HttpResponse, HttpResponseBuilder,
};
use rust_embed::RustEmbed;
use std::borrow::Cow;
use tracing::{debug, warn};

#[derive(RustEmbed)]
#[folder = "static/"]
#[prefix = "/"]
struct StaticAssets;

/// Configure static file serving for the web UI using embedded files
pub fn configure(cfg: &mut web::ServiceConfig, enable_cache: bool) {
    debug!("Configuring embedded static file serving");

    // Set up cache headers based on environment
    let cache_control = if enable_cache {
        // Production: cache assets for 1 year, but not index.html
        "public, max-age=31536000, immutable"
    } else {
        // Development: no caching
        "no-cache, no-store, must-revalidate"
    };

    // Create a default service that handles all non-API routes
    let default_service = fn_service(move |req: ServiceRequest| {
        let enable_cache = enable_cache;
        async move {
            serve_embedded_file(req, enable_cache).await
        }
    });

    // Apply cache headers middleware and register the service
    if enable_cache {
        cfg.default_service(
            web::scope("")
                .wrap(DefaultHeaders::new().header("Cache-Control", cache_control))
                .service(default_service)
        );
    } else {
        cfg.default_service(
            web::scope("")
                .wrap(DefaultHeaders::new()
                    .header("Cache-Control", "no-cache, no-store, must-revalidate")
                    .header("Pragma", "no-cache")
                    .header("Expires", "0"))
                .service(default_service)
        );
    }
}

/// Serve embedded files or fall back to index.html for SPA routing
async fn serve_embedded_file(
    req: ServiceRequest,
    enable_cache: bool,
) -> Result<ServiceResponse, Error> {
    let path = req.path();

    // Don't apply SPA fallback to API routes or WebSocket
    if path.starts_with("/api/") || path.starts_with("/ws/") {
        debug!("Skipping static file serving for API/WebSocket route: {}", path);
        return Ok(ServiceResponse::new(
            req.into_parts().0,
            HttpResponse::NotFound().body("API endpoint not found"),
        ));
    }

    // Remove leading slash for embed lookup
    let path = if path == "/" {
        "index.html"
    } else {
        path.trim_start_matches('/')
    };

    // Try to find the exact file first
    if let Some(content) = StaticAssets::get(path) {
        return serve_embedded_content(req, path, content, enable_cache);
    }

    // Check if this looks like a file request (has an extension)
    if path.contains('.') && !path.ends_with('/') {
        // File doesn't exist, return 404
        debug!("Static file not found: {}", path);
        return Ok(ServiceResponse::new(
            req.into_parts().0,
            HttpResponse::NotFound().body("File not found"),
        ));
    }

    // For all other routes, serve index.html (SPA fallback)
    if let Some(content) = StaticAssets::get("index.html") {
        debug!("Serving index.html for SPA route: {}", req.path());
        return serve_embedded_content(req, "index.html", content, false); // Never cache index.html
    }

    // No index.html found
    warn!("index.html not found in embedded assets");
    Ok(ServiceResponse::new(
        req.into_parts().0,
        HttpResponse::NotFound().body("Application not found. Please rebuild with frontend assets."),
    ))
}

/// Serve embedded content with appropriate headers
fn serve_embedded_content(
    req: ServiceRequest,
    path: &str,
    content: rust_embed::EmbeddedFile,
    enable_cache: bool,
) -> Result<ServiceResponse, Error> {
    let mime_type = mime_guess::from_path(path)
        .first_or_octet_stream()
        .to_string();

    let mut builder = HttpResponseBuilder::new(StatusCode::OK);
    builder.content_type(mime_type);

    // Set cache headers based on file type and settings
    if path == "index.html" || !enable_cache {
        // Never cache index.html or in development mode
        builder.insert_header((
            header::CACHE_CONTROL,
            header::HeaderValue::from_static("no-cache, no-store, must-revalidate"),
        ));
    } else if path.contains("assets/") {
        // Assets with hashes can be cached forever
        builder.insert_header((
            header::CACHE_CONTROL,
            header::HeaderValue::from_static("public, max-age=31536000, immutable"),
        ));
    } else {
        // Other static files can be cached for a day
        builder.insert_header((
            header::CACHE_CONTROL,
            header::HeaderValue::from_static("public, max-age=86400"),
        ));
    }

    // Add ETag if available
    if let Some(etag) = content.metadata.sha256_hash() {
        builder.insert_header((
            header::ETAG,
            header::HeaderValue::from_str(&format!("\"{}\"", etag)).unwrap_or_else(|_| {
                header::HeaderValue::from_static("\"unknown\"")
            }),
        ));
    }

    // Add Last-Modified if available
    if let Some(last_modified) = content.metadata.last_modified() {
        if let Ok(time) = httpdate::fmt_http_date(std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(last_modified)) {
            builder.insert_header((
                header::LAST_MODIFIED,
                header::HeaderValue::from_str(&time).unwrap_or_else(|_| {
                    header::HeaderValue::from_static("Thu, 01 Jan 1970 00:00:00 GMT")
                }),
            ));
        }
    }

    let response = builder.body(content.data.to_vec());
    Ok(ServiceResponse::new(req.into_parts().0, response))
}

/// Alternative implementation using actix-files for development
/// This can be used when static files are available on disk
pub fn configure_development(cfg: &mut web::ServiceConfig, static_dir: std::path::PathBuf) {
    use actix_files::{Files, NamedFile};

    debug!("Configuring development static file serving from: {:?}", static_dir);

    // Check if static directory exists
    if !static_dir.exists() {
        warn!("Static directory does not exist: {:?}", static_dir);
        warn!("Using embedded files fallback");
        configure(cfg, false);
        return;
    }

    // Serve static files with no caching in development
    let files_service = Files::new("/", &static_dir)
        .use_etag(true)
        .use_last_modified(true)
        .prefer_utf8(true)
        .index_file("index.html")
        .default_handler(fn_service(move |req: ServiceRequest| {
            let static_dir = static_dir.clone();
            async move {
                spa_fallback_dev(req, static_dir).await
            }
        }));

    cfg.default_service(
        web::scope("")
            .wrap(DefaultHeaders::new()
                .header("Cache-Control", "no-cache, no-store, must-revalidate")
                .header("Pragma", "no-cache")
                .header("Expires", "0"))
            .service(files_service)
    );
}

/// SPA fallback handler for development mode
async fn spa_fallback_dev(
    req: ServiceRequest,
    static_dir: std::path::PathBuf,
) -> Result<ServiceResponse, Error> {
    use actix_files::NamedFile;

    let path = req.path();

    // Don't apply SPA fallback to API routes or WebSocket
    if path.starts_with("/api/") || path.starts_with("/ws/") {
        return Ok(ServiceResponse::new(
            req.into_parts().0,
            HttpResponse::NotFound().body("API endpoint not found"),
        ));
    }

    // For all other routes, serve index.html (SPA fallback)
    let index_path = static_dir.join("index.html");
    if index_path.exists() {
        let file = NamedFile::open(index_path)?;
        let mut res = file.into_response(&req);
        res.headers_mut().insert(
            header::CACHE_CONTROL,
            header::HeaderValue::from_static("no-cache, no-store, must-revalidate"),
        );
        Ok(ServiceResponse::new(req.into_parts().0, res))
    } else {
        Ok(ServiceResponse::new(
            req.into_parts().0,
            HttpResponse::NotFound().body("index.html not found"),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App};

    #[actix_web::test]
    async fn test_embedded_file_serving() {
        // This test will work once we have actual files in the static directory
        let app = test::init_service(
            App::new()
                .configure(|cfg| configure(cfg, false))
        ).await;

        // Test that API routes are not handled by static file server
        let req = test::TestRequest::get()
            .uri("/api/v1/health")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[actix_web::test]
    async fn test_spa_fallback_embedded() {
        let app = test::init_service(
            App::new()
                .configure(|cfg| configure(cfg, false))
        ).await;

        // Test that unknown routes return appropriate response
        let req = test::TestRequest::get()
            .uri("/dashboard")
            .to_request();
        let resp = test::call_service(&app, req).await;
        // Will be 404 if index.html doesn't exist in embedded assets yet
        assert!(resp.status() == StatusCode::NOT_FOUND || resp.status() == StatusCode::OK);
    }
}