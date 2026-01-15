//! Utility functions for the Yew UI

/// Format a number with appropriate units
pub fn format_distance(meters: f32) -> String {
    if meters >= 1000.0 {
        format!("{:.2} km", meters / 1000.0)
    } else if meters >= 1.0 {
        format!("{:.2} m", meters)
    } else if meters >= 0.01 {
        format!("{:.1} cm", meters * 100.0)
    } else {
        format!("{:.1} mm", meters * 1000.0)
    }
}

/// Format file size
pub fn format_file_size(bytes: usize) -> String {
    if bytes >= 1_000_000_000 {
        format!("{:.2} GB", bytes as f64 / 1_000_000_000.0)
    } else if bytes >= 1_000_000 {
        format!("{:.2} MB", bytes as f64 / 1_000_000.0)
    } else if bytes >= 1000 {
        format!("{:.2} KB", bytes as f64 / 1000.0)
    } else {
        format!("{} bytes", bytes)
    }
}

/// Get entity type display name
pub fn get_entity_display_name(entity_type: &str) -> &str {
    // Strip "Ifc" prefix and split camel case
    entity_type.strip_prefix("Ifc").unwrap_or(entity_type)
}

/// Get icon name for entity type
pub fn get_entity_icon(entity_type: &str) -> &'static str {
    match entity_type {
        s if s.contains("Wall") => "square",
        s if s.contains("Slab") || s.contains("Floor") => "layers",
        s if s.contains("Roof") => "home",
        s if s.contains("Beam") => "minus",
        s if s.contains("Column") => "pillar",
        s if s.contains("Door") => "door-open",
        s if s.contains("Window") => "app-window",
        s if s.contains("Stair") => "stairs",
        s if s.contains("Railing") => "fence",
        s if s.contains("Furniture") => "sofa",
        s if s.contains("Space") => "box",
        s if s.contains("Building") => "building",
        s if s.contains("Storey") => "layers",
        s if s.contains("Site") => "map",
        s if s.contains("Project") => "folder",
        s if s.contains("Pipe") => "cylinder",
        s if s.contains("Duct") => "wind",
        s if s.contains("Cable") => "cable",
        _ => "cube",
    }
}

/// Debounce helper - returns true if enough time has passed
pub struct Debounce {
    last_call: f64,
    delay_ms: f64,
}

impl Debounce {
    pub fn new(delay_ms: f64) -> Self {
        Self {
            last_call: 0.0,
            delay_ms,
        }
    }

    pub fn should_call(&mut self) -> bool {
        let now = js_sys::Date::now();
        if now - self.last_call >= self.delay_ms {
            self.last_call = now;
            true
        } else {
            false
        }
    }
}

/// Throttle helper - limits calls to max frequency
pub struct Throttle {
    last_call: f64,
    interval_ms: f64,
}

impl Throttle {
    pub fn new(interval_ms: f64) -> Self {
        Self {
            last_call: 0.0,
            interval_ms,
        }
    }

    pub fn should_call(&mut self) -> bool {
        let now = js_sys::Date::now();
        if now - self.last_call >= self.interval_ms {
            self.last_call = now;
            true
        } else {
            false
        }
    }
}

/// Get the `file` URL query parameter if present.
/// Example: `?file=house.ifc` returns `Some("house.ifc")`
pub fn get_file_param() -> Option<String> {
    let window = web_sys::window()?;
    let location = window.location();
    let search = location.search().ok()?;
    if search.is_empty() {
        return None;
    }
    let params = web_sys::UrlSearchParams::new_with_str(&search).ok()?;
    params.get("file")
}

/// Build the full URL to fetch an IFC file from the server's /ifc directory.
/// If the file parameter is a relative path, it's resolved relative to /ifc/.
/// If it's already an absolute URL, it's returned as-is.
pub fn build_ifc_url(file_param: &str) -> String {
    // If already an absolute URL (http:// or https://), use as-is
    if file_param.starts_with("http://") || file_param.starts_with("https://") {
        return file_param.to_string();
    }

    // Otherwise, treat as relative to /ifc/
    let base = web_sys::window()
        .and_then(|w| w.location().origin().ok())
        .unwrap_or_default();

    // Strip leading slash if present to avoid double slashes
    let clean_file = file_param.trim_start_matches('/');

    format!("{}/ifc/{}", base, clean_file)
}

/// Fetch IFC file content from a URL.
/// Returns the file content as a String, or an error message.
pub async fn fetch_ifc_file(url: &str) -> Result<String, String> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;

    let window = web_sys::window().ok_or("No window object")?;

    // Create request
    let request = web_sys::Request::new_with_str(url)
        .map_err(|e| format!("Failed to create request: {:?}", e))?;

    // Fetch
    let resp_value = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("Fetch failed: {:?}", e))?;

    let resp: web_sys::Response = resp_value
        .dyn_into()
        .map_err(|_| "Response is not a Response object")?;

    if !resp.ok() {
        return Err(format!(
            "HTTP error: {} {}",
            resp.status(),
            resp.status_text()
        ));
    }

    // Get text body
    let text_promise = resp
        .text()
        .map_err(|e| format!("Failed to get text: {:?}", e))?;
    let text_value = JsFuture::from(text_promise)
        .await
        .map_err(|e| format!("Failed to read response: {:?}", e))?;

    text_value
        .as_string()
        .ok_or_else(|| "Response is not a string".to_string())
}
