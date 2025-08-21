#[cfg(target_arch = "wasm32")]
use {
    wasm_bindgen::{JsCast, UnwrapThrowExt},
    wasm_bindgen_futures::JsFuture,
    web_sys::{Request, RequestInit, RequestCache},
    std::cell::{Cell, RefCell},
};

/// WASM-specific utilities and thread-local storage
#[cfg(target_arch = "wasm32")]
pub struct WasmUtils;

#[cfg(target_arch = "wasm32")]
thread_local! {
    pub static PENDING_GEOMETRY: RefCell<Option<(Vec<crate::vertex::Vertex>, Vec<u16>, Vec<crate::instance::DrawBatch>, Vec<crate::pointcloud_vertex::PointCloudInstance>)>> = RefCell::new(None);
    pub static LOCAL_HASH: RefCell<Option<u64>> = RefCell::new(None);
    pub static LOCAL_FETCHING: Cell<bool> = Cell::new(false);
}

#[cfg(target_arch = "wasm32")]
impl WasmUtils {
    /// Set up WASM panic hook and run the main application
    pub fn run_web() -> Result<(), wasm_bindgen::JsValue> {
        console_error_panic_hook::set_once();
        crate::run().unwrap_throw();
        Ok(())
    }

    /// Check if geometry is currently being fetched
    pub fn is_fetching() -> bool {
        LOCAL_FETCHING.with(|f| f.get())
    }

    /// Set fetching state
    pub fn set_fetching(fetching: bool) {
        LOCAL_FETCHING.with(|f| f.set(fetching));
    }

    /// Get pending geometry data
    pub fn take_pending_geometry() -> Option<(Vec<crate::vertex::Vertex>, Vec<u16>, Vec<crate::instance::DrawBatch>, Vec<crate::pointcloud_vertex::PointCloudInstance>)> {
        PENDING_GEOMETRY.with(|pg| pg.borrow_mut().take())
    }

    /// Set pending geometry data
    pub fn set_pending_geometry(data: (Vec<crate::vertex::Vertex>, Vec<u16>, Vec<crate::instance::DrawBatch>, Vec<crate::pointcloud_vertex::PointCloudInstance>)) {
        PENDING_GEOMETRY.with(|pg| *pg.borrow_mut() = Some(data));
    }

    /// Get local hash for change detection
    pub fn get_local_hash() -> Option<u64> {
        LOCAL_HASH.with(|lh| *lh.borrow())
    }

    /// Set local hash for change detection
    pub fn set_local_hash(hash: u64) {
        LOCAL_HASH.with(|lh| *lh.borrow_mut() = Some(hash));
    }

    /// HTTP path for local geometry JSON
    pub const LOCAL_GEOMETRY_HTTP_PATH: &'static str = "all_geometry.json";

    /// Fetch text content from URL
    pub async fn fetch_text(url: &str) -> Option<String> {
        let opts = RequestInit::new();
        opts.set_method("GET");
        opts.set_cache(RequestCache::NoCache);

        let request = Request::new_with_str_and_init(url, &opts).ok()?;
        let window = web_sys::window()?;
        let resp_value = JsFuture::from(window.fetch_with_request(&request)).await.ok()?;
        let resp: web_sys::Response = resp_value.dyn_into().ok()?;
        
        if !resp.ok() {
            return None;
        }

        let text_promise = resp.text().ok()?;
        let text_value = JsFuture::from(text_promise).await.ok()?;
        text_value.as_string()
    }

    /// FNV-1a 64-bit hash function
    pub fn fnv1a64(data: &[u8]) -> u64 {
        const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
        const FNV_PRIME: u64 = 0x100000001b3;
        
        let mut hash = FNV_OFFSET_BASIS;
        for &byte in data {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(FNV_PRIME);
        }
        hash
    }
}
