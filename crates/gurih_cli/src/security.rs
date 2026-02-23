use axum::{
    body::Body,
    http::{HeaderValue, Request},
    middleware::Next,
    response::Response,
};

fn apply_common_headers(headers: &mut axum::http::HeaderMap) {
    headers.insert("X-Content-Type-Options", HeaderValue::from_static("nosniff"));
    headers.insert("X-Frame-Options", HeaderValue::from_static("SAMEORIGIN"));
    headers.insert("X-XSS-Protection", HeaderValue::from_static("1; mode=block"));
    headers.insert(
        "Referrer-Policy",
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
}

pub async fn dev_security_headers(req: Request<Body>, next: Next) -> Response {
    let mut response = next.run(req).await;
    let headers = response.headers_mut();

    apply_common_headers(headers);

    // Sentinel: Dev Mode (Permissive) - Allow 'unsafe-inline' for Vue runtime/dev tools.
    headers.insert(
        "Content-Security-Policy",
        HeaderValue::from_static("default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; font-src 'self' https://fonts.gstatic.com; img-src 'self' data: blob: https:; connect-src 'self'; frame-ancestors 'self';"),
    );

    response
}

pub async fn prod_security_headers(req: Request<Body>, next: Next) -> Response {
    let mut response = next.run(req).await;
    let headers = response.headers_mut();

    apply_common_headers(headers);

    // Sentinel: Production Mode (Strict) - No 'unsafe-inline'.
    // Removed 'unsafe-inline' from script-src and style-src.
    // Added object-src 'none' and base-uri 'self'.
    headers.insert(
        "Content-Security-Policy",
        HeaderValue::from_static("default-src 'self'; script-src 'self'; style-src 'self' https://fonts.googleapis.com; font-src 'self' https://fonts.gstatic.com; img-src 'self' data: blob: https:; connect-src 'self'; frame-ancestors 'self'; object-src 'none'; base-uri 'self';"),
    );

    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use axum::{Router, middleware, routing::get};
    use tower::util::ServiceExt;

    #[tokio::test]
    async fn test_dev_security_headers() {
        let app = Router::new()
            .route("/", get(|| async { "Hello" }))
            .layer(middleware::from_fn(dev_security_headers));

        let response = app
            .oneshot(Request::builder().uri("/").body(Body::from("")).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let headers = response.headers();
        let csp = headers.get("Content-Security-Policy").unwrap().to_str().unwrap();

        // Dev headers MUST contain unsafe-inline
        assert!(csp.contains("script-src 'self' 'unsafe-inline'"));

        // Verify common headers are present
        assert_eq!(headers.get("X-Frame-Options").unwrap(), "SAMEORIGIN");
    }

    #[tokio::test]
    async fn test_prod_security_headers() {
        let app = Router::new()
            .route("/", get(|| async { "Hello" }))
            .layer(middleware::from_fn(prod_security_headers));

        let response = app
            .oneshot(Request::builder().uri("/").body(Body::from("")).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let headers = response.headers();
        let csp = headers.get("Content-Security-Policy").unwrap().to_str().unwrap();

        // Prod headers MUST NOT contain unsafe-inline
        assert!(!csp.contains("'unsafe-inline'"));
        assert!(csp.contains("script-src 'self'"));
        assert!(csp.contains("object-src 'none'"));

        // Verify common headers are present
        assert_eq!(headers.get("X-Frame-Options").unwrap(), "SAMEORIGIN");
    }
}
