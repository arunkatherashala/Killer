//! Tier 3 — minimal HTTP control plane for headless `killer_ui` JSON (shared by `killer_ui_serve` and `killer_ui` CLI).

use crate::http_server::HttpServer;
use crate::web_framework::{HttpRequest, HttpResponse, StatusCode};

use super::{headless_frame_json, KillerUiEngine};

/// Same JSON body as `GET /health` on the Tier 3 HTTP panel (VM: call builtin `ui_health()`).
#[inline]
pub fn killer_ui_health_json() -> &'static str {
    r#"{"ok":true,"service":"killer_ui"}"#
}

/// Bind `host:port` and serve JSON panel routes, `GET /health`, `GET /`, and CORS `OPTIONS`.
pub fn run_headless_panel_server(host: &str, port: u16) -> Result<(), String> {
    let mut server = HttpServer::new(host, port);

    server.on_route("GET", "/health", |_req: &HttpRequest| {
        HttpResponse::new(StatusCode::OK)
            .enable_cors()
            .json(killer_ui_health_json().to_string())
    })?;

    server.on_route("OPTIONS", "/health", |_req: &HttpRequest| {
        HttpResponse::new(StatusCode::OK)
            .enable_cors()
            .set_body(String::new())
    })?;

    server.on_route("GET", "/killer-ui/headless.json", |_req: &HttpRequest| {
        let engine = KillerUiEngine::example_parallel();
        let frame = engine.tick_headless();
        let json = headless_frame_json(engine.version, &frame);
        HttpResponse::new(StatusCode::OK)
            .enable_cors()
            .json(json)
    })?;

    server.on_route("GET", "/killer-ui/version.json", |_req: &HttpRequest| {
        let v = super::KILLER_UI_ENGINE_VERSION;
        let json = format!(r#"{{"killer_ui_engine_version":{}}}"#, v);
        HttpResponse::new(StatusCode::OK)
            .enable_cors()
            .json(json)
    })?;

    server.on_route("OPTIONS", "/killer-ui/headless.json", |_req: &HttpRequest| {
        HttpResponse::new(StatusCode::OK)
            .enable_cors()
            .set_body(String::new())
    })?;

    server.on_route("OPTIONS", "/killer-ui/version.json", |_req: &HttpRequest| {
        HttpResponse::new(StatusCode::OK)
            .enable_cors()
            .set_body(String::new())
    })?;

    server.on_route("GET", "/", |_req: &HttpRequest| {
        let html = concat!(
            "<!DOCTYPE html><html><head><meta charset=\"utf-8\">",
            "<title>killer_ui</title></head><body>",
            "<p>killer_ui HTTP — ",
            "<a href=\"/killer-ui/headless.json\">headless.json</a>",
            " · ",
            "<a href=\"/killer-ui/version.json\">version.json</a>",
            " · ",
            "<a href=\"/health\">health</a>",
            "</p></body></html>"
        );
        HttpResponse::new(StatusCode::OK)
            .enable_cors()
            .set_header("Content-Type".to_string(), "text/html; charset=utf-8".to_string())
            .set_body(html.to_string())
    })?;

    eprintln!(
        "killer_ui HTTP: http://{}:{}/killer-ui/headless.json (Ctrl+C to stop)",
        host, port
    );
    server.run()
}
