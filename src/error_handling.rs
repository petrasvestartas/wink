use openmodel::AllGeometryData;

/// Error handling utilities for geometry loading and processing
pub struct ErrorHandler;

impl ErrorHandler {
    /// Load geometry data with fallback to embedded JSON
    pub async fn load_geometry_with_fallback() -> AllGeometryData {
        let local_json = Self::load_local_geometry().await;
        let all_geom: AllGeometryData = match local_json {
            Some(ref s) => {
                println!("🔄 Loading geometry from runtime file");
                serde_json::from_str::<AllGeometryData>(s)
                    .unwrap_or_else(|e| {
                        println!("⚠️ Failed to parse runtime file, using embedded: {}", e);
                        Self::load_embedded_geometry()
                    })
            },
            None => {
                println!("📦 No runtime file found, using embedded geometry");
                Self::load_embedded_geometry()
            },
        };
        // Procedural augmentation removed - geometry processed directly in geometry_loader
        all_geom
    }

    /// Load embedded geometry as fallback
    fn load_embedded_geometry() -> AllGeometryData {
        serde_json::from_str(include_str!("../data/all_geometry.json"))
            .expect("embedded geometry JSON must be valid")
    }

    /// Handle file loading logic separately
    async fn load_local_geometry() -> Option<String> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let base = env!("CARGO_MANIFEST_DIR");
            let local_path = format!("{}/data/all_geometry.json", base);
            println!("🔍 Attempting to load geometry from: {}", local_path);
            match std::fs::read_to_string(&local_path) {
                Ok(content) => {
                    println!("✅ Successfully loaded {} bytes from file", content.len());
                    Some(content)
                },
                Err(e) => {
                    println!("❌ Failed to load file: {}", e);
                    None
                }
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            Self::fetch_text("/geometry/all_geometry.json").await
        }
    }

    #[cfg(target_arch = "wasm32")]
    async fn fetch_text(url: &str) -> Option<String> {
        use wasm_bindgen::JsCast;
        use wasm_bindgen_futures::JsFuture;
        use web_sys::{Request, RequestInit, RequestCache};

        let window = web_sys::window()?;
        // Cache-busting: append a timestamp to avoid stale caches
        let ts = window.performance()?.now() as u64;
        let sep = if url.contains('?') { "&" } else { "?" };
        let bust = format!("{}{}ts={}", url, sep, ts);

        // Prefer no-store to bypass intermediary caches in dev
        let init = RequestInit::new();
        init.set_method("GET");
        init.set_cache(RequestCache::NoStore);
        let req = Request::new_with_str_and_init(&bust, &init).ok()?;

        let resp_value = JsFuture::from(window.fetch_with_request(&req)).await.ok()?;
        let resp: web_sys::Response = resp_value.dyn_into().ok()?;
        if !resp.ok() {
            web_sys::console::error_1(&format!("Fetch failed: {} for {}", resp.status(), bust).into());
            return None;
        }
        let text_promise = resp.text().ok()?;
        let text = JsFuture::from(text_promise).await.ok()?;
        text.as_string()
    }

    /// Tiny FNV-1a hash for quick change detection
    #[cfg(target_arch = "wasm32")]
    pub fn fnv1a64(bytes: &[u8]) -> u64 {
        let mut hash: u64 = 0xcbf29ce484222325;
        for b in bytes {
            hash ^= *b as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash
    }
}
