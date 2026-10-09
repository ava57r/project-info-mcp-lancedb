//! Dashboard static assets served at `/`, `/dashboard`, and `/static/*`.
//!
//! The page markup, stylesheet, and client script live in `src/http/static/`
//! as real files (editable with proper HTML/CSS/JS tooling) and are baked
//! into the binary via `include_str!`.

/// Dashboard page markup (`static/dashboard.html` + `/static/*` asset links).
pub const DASHBOARD_HTML: &str = include_str!("static/dashboard.html");

/// Dashboard stylesheet (`static/style.css`).
pub const STYLE_CSS: &str = include_str!("static/style.css");

/// Dashboard client script (`static/app.js`).
pub const APP_JS: &str = include_str!("static/app.js");
