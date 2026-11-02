mod generator;

use generator::Options;
use loki_sdk::panel::{Field, FieldKind, Panel, PanelResponse, Section};
use loki_sdk::{Request, Response, Result, Router};
use serde_json::json;

fn home(_: Request) -> Result<Response> {
    Ok(Response::html(include_str!("../static/index.html")))
}

fn health(_: Request) -> Result<Response> {
    Ok(Response::text("ready"))
}

fn generate(request: Request) -> Result<Response> {
    let result = Options::parse(|name| request.query(name)).and_then(|options| {
        let numbers = options.generate()?;
        Response::json(&json!({"numbers": numbers, "options": options}))
    });
    match result {
        Ok(response) => response,
        Err(error) => {
            Response::json(&json!({"error": error.message()}))?.with_status(error.status())
        }
    }
    .header("cache-control", "no-store")
}

fn panel(request: Request) -> Result<Response> {
    let action = request.context().panel_action.as_ref();
    let value = |key: &str, default: &str| {
        action
            .and_then(|a| a.fields.get(key))
            .cloned()
            .unwrap_or_else(|| default.into())
    };
    let mut panel = Panel::new().section(
        Section::new("Random Number Generator")
            .muted("Choose an inclusive range and draw 1–100 integers.")
            .form(
                "generate",
                [
                    Field::new("min", "Minimum", FieldKind::Number).value(value("min", "1")),
                    Field::new("max", "Maximum", FieldKind::Number).value(value("max", "100")),
                    Field::new("count", "Quantity", FieldKind::Number).value(value("count", "1")),
                    Field::new("unique", "Unique results", FieldKind::Bool)
                        .value(value("unique", "false") == "true"),
                ],
                "Generate",
            ),
    );
    if let Some(action) = action {
        if action.action != "generate" {
            return PanelResponse::error(panel, "Unknown generator action.").respond();
        }
        let result = Options::parse(|key| action.fields.get(key).cloned())
            .and_then(|options| options.generate());
        match result {
            Ok(numbers) => {
                let text = numbers
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ");
                panel = panel.section(Section::new("Your results").code("Numbers", text));
            }
            Err(error) => return PanelResponse::error(panel, error.message()).respond(),
        }
    }
    panel.respond()
}

fn stylesheet(_: Request) -> Result<Response> {
    loki_sdk::asset("static/style.css")
}
fn javascript(_: Request) -> Result<Response> {
    loki_sdk::asset("static/app.js")
}

pub fn handle(request: Request) -> Result<Response> {
    Router::new()
        .get("/", home)
        .get("/health", health)
        .get("/api/generate", generate)
        .get("/static/style.css", stylesheet)
        .get("/static/app.js", javascript)
        .panel(panel)
        .handle(request)
}

#[cfg(target_arch = "wasm32")]
loki_sdk::export!(handle);

#[cfg(test)]
mod tests {
    use super::*;
    use loki_sdk::{Context, Method, testing};

    fn context() -> Context {
        serde_json::from_value(json!({
            "app_id": "random-number-generator", "origin": "http://random-number-generator.localhost:7070",
            "issuer": "http://localhost:7070", "config": {}
        })).unwrap()
    }

    #[test]
    fn health_does_not_need_identity_or_host_services() {
        let response = handle(testing::request(Method::Get, "/health", context())).unwrap();
        assert_eq!(response.status, 200);
        assert_eq!(response.body, b"ready");
    }

    #[test]
    fn api_returns_uncached_json_and_validation_errors() {
        for (query, status) in [("min=-2&max=-2&count=3", 200), ("count=101", 400)] {
            let response = handle(testing::request(
                Method::Get,
                &format!("/api/generate?{query}"),
                context(),
            ))
            .unwrap();
            assert_eq!(response.status, status);
            assert_eq!(response.headers["cache-control"], "no-store");
            let body: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
            if status == 200 {
                assert_eq!(body["numbers"], json!([-2, -2, -2]));
            } else {
                assert!(body["error"].is_string());
            }
        }
    }

    #[test]
    fn dashboard_panel_supports_draws_and_errors_within_host_limits() {
        for (fields, error) in [
            (
                json!({"min":"-9007199254740991", "max":"9007199254740991", "count":"100", "unique":"true"}),
                false,
            ),
            (json!({"min":"5", "max":"1"}), true),
        ] {
            let mut ctx = context();
            ctx.panel_action = Some(
                serde_json::from_value(json!({"action":"generate", "fields":fields})).unwrap(),
            );
            let response = handle(testing::request(Method::Post, "/_loki/panel", ctx)).unwrap();
            let result: PanelResponse = serde_json::from_slice(&response.body).unwrap();
            assert!(result.valid());
            assert_eq!(!result.error.is_empty(), error);
        }
    }

    #[test]
    fn unknown_routes_and_methods_are_rejected() {
        assert_eq!(
            handle(testing::request(Method::Get, "/missing", context()))
                .unwrap()
                .status,
            404
        );
        assert_eq!(
            handle(testing::request(Method::Post, "/api/generate", context()))
                .unwrap()
                .status,
            405
        );
    }
}
