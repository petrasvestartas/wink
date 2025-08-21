use std::{iter, sync::Arc}; // Arc is a thread-safe reference-counted pointer
use anyhow::Result;
use winit::{
    application::ApplicationHandler, 
    event::{WindowEvent, KeyEvent, MouseButton, ElementState}, //* - import everythingi is skipped due to warnings
    event_loop::{ActiveEventLoop, EventLoop}, 
    keyboard::{KeyCode, PhysicalKey}, 
    window::Window
};
pub mod vertex;
pub mod camera;
pub mod timing;
pub mod instance;
pub mod pointcloud_vertex;
pub mod merged_geometry;
use gpu_geometry::{GpuGeometryPipeline, PipeTransform, SphereTransform};
pub mod shader_color_pipeline;
pub mod shader_solid_pipeline;
pub mod shader_lights_pipeline;
pub mod shader_pointcloud_pipeline;
pub mod gpu_geometry;
use vertex::Vertex;
use pointcloud_vertex::{PointCloudInstance, QuadVertex};
use camera::{Camera, CameraUniform, CameraController};
use timing::Instant;
use wgpu::util::DeviceExt;
use instance::Instance;
use instance::InstanceRaw;
use instance::DrawBatch;
use instance::BatchDraw;
use instance::BatchKind;
// OpenModel: JSON geometry + mesh utilities
use openmodel::AllGeometryData;
use openmodel::geometry::{Mesh, PointCloud};

/// Scene bounding box for orthographic camera framing

#[derive(Debug, Clone, Copy)]
pub struct SceneBounds {
    pub min: cgmath::Point3<f32>,
    pub max: cgmath::Point3<f32>,
}

impl SceneBounds {
    pub fn new() -> Self {
        Self {
            min: cgmath::Point3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY),
            max: cgmath::Point3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY),
        }
    }

    pub fn expand_point(&mut self, point: cgmath::Point3<f32>) {
        self.min.x = self.min.x.min(point.x);
        self.min.y = self.min.y.min(point.y);
        self.min.z = self.min.z.min(point.z);
        self.max.x = self.max.x.max(point.x);
        self.max.y = self.max.y.max(point.y);
        self.max.z = self.max.z.max(point.z);
    }

    pub fn center(&self) -> cgmath::Point3<f32> {
        cgmath::Point3::new(
            (self.min.x + self.max.x) * 0.5,
            (self.min.y + self.max.y) * 0.5,
            (self.min.z + self.max.z) * 0.5,
        )
    }

    pub fn size(&self) -> cgmath::Vector3<f32> {
        cgmath::Vector3::new(
            self.max.x - self.min.x,
            self.max.y - self.min.y,
            self.max.z - self.min.z,
        )
    }

    pub fn is_valid(&self) -> bool {
        self.min.x <= self.max.x && self.min.y <= self.max.y && self.min.z <= self.max.z
    }
}

#[cfg(target_arch = "wasm32")]
const LOCAL_GEOMETRY_HTTP_PATH: &str = "/geometry/all_geometry.json"; // served by docs dev server

// Native-only: absolute path to local JSON for fast runtime reloads (fallbacks to include_str! if not found)
#[cfg(not(target_arch = "wasm32"))]
const LOCAL_GEOMETRY_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/src/openmodel/all_geometry.json");

// Polling interval for change detection (ms)
const GEOMETRY_POLL_INTERVAL_MS: u64 = 1000;

// ADDED (depth): Depth buffer format used by pipelines and depth texture
#[cfg(target_arch = "wasm32")]
const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24Plus;
#[cfg(not(target_arch = "wasm32"))]
const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

// MSAA sample count (disable on WebGL path)
#[cfg(target_arch = "wasm32")]
const _MSAA_SAMPLE_COUNT: u32 = 4;
#[cfg(not(target_arch = "wasm32"))]
const _MSAA_SAMPLE_COUNT: u32 = 4;

#[derive(Copy, Clone, Debug, PartialEq)]
enum PipelineMode { Color, Solid, Lights }

#[cfg(target_arch = "wasm32")]
use std::cell::{Cell, RefCell};

#[cfg(target_arch = "wasm32")]
thread_local! {
    static PENDING_GEOMETRY: RefCell<Option<(Vec<Vertex>, Vec<u16>, Vec<DrawBatch>, Vec<PointCloudInstance>)>> = RefCell::new(None);
    static LOCAL_HASH: RefCell<Option<u64>> = RefCell::new(None);
    static LOCAL_FETCHING: Cell<bool> = Cell::new(false);
}


#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::JsFuture;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::spawn_local;

#[cfg(target_arch = "wasm32")]
use web_sys::{Request, RequestInit, RequestCache};

#[cfg(target_arch = "wasm32")]
async fn fetch_text(url: &str) -> Option<String> {
    let window = web_sys::window()?;
    // Cache-busting: append a timestamp to avoid stale caches
    let ts = window.performance()?.now() as u64;
    let sep = if url.contains('?') { "&" } else { "?" };
    let bust = format!("{}{}ts={}", url, sep, ts);

    // Prefer no-store to bypass intermediary caches in dev
    let mut init = RequestInit::new();
    init.set_method("GET");
    init.set_cache(RequestCache::NoStore);
    let req = Request::new_with_str_and_init(&bust, &init).ok()?;

    let resp_value = JsFuture::from(window.fetch_with_request(&req)).await.ok()?;
    let resp: web_sys::Response = resp_value.dyn_into().ok()?;
    if !resp.ok() {
        #[cfg(target_arch = "wasm32")]
        {
            web_sys::console::error_1(&format!("Fetch failed: {} for {}", resp.status(), bust).into());
        }
        return None;
    }
    let text_promise = resp.text().ok()?;
    let text = JsFuture::from(text_promise).await.ok()?;
    text.as_string()
}

// Tiny FNV-1a hash for quick change detection
#[cfg(target_arch = "wasm32")]
fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for b in bytes {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}
// Helper: push mesh faces as triangles using cached triangulation with per-vertex or default color
fn append_mesh_as_triangles(
    mesh: &mut Mesh,
    default_color: [f32; 3],
    vertices: &mut Vec<Vertex>,
    indices: &mut Vec<u16>,
) {
    // Collect face keys first to avoid holding an immutable borrow while mutably triangulating
    let face_keys: Vec<usize> = mesh.get_face_data().map(|(fk, _)| fk).collect();
    for face_key in face_keys {
        let triangles: Vec<[usize; 3]> = if let Some(tri_ref) = mesh.triangulate_face(face_key) {
            tri_ref.clone()
        } else {
            Vec::new()
        };

        log::debug!("Face {} triangulated into {} triangles", face_key, triangles.len());

        for tri in triangles {
            for &vk in &tri {
                if let Some(pos) = mesh.vertex_position(vk) {
                    let use_default = if let Some(vd) = mesh.vertex.get(&vk) {
                        !(vd.attributes.contains_key("r") && vd.attributes.contains_key("g") && vd.attributes.contains_key("b"))
                    } else { true };
                    let color = if use_default {
                        default_color
                    } else if let Some(vd) = mesh.vertex.get(&vk) {
                        let c = vd.color();
                        [c[0] as f32, c[1] as f32, c[2] as f32]
                    } else { default_color };

                    // Resolve vertex normal via OpenModel (authored -> computed -> face -> +Z)
                    let n = mesh.vertex_normal_resolved(vk, Some(face_key));
                    let normal = [n.x as f32, n.y as f32, n.z as f32];

                    if vertices.len() >= u16::MAX as usize { break; }
                    vertices.push(Vertex { position: [pos.x as f32, pos.y as f32, pos.z as f32], color, normal });
                    indices.push((vertices.len() - 1) as u16);
                }
            }
        }
    }
}

// Helper: convert openmodel Xform (column-major) to Instance (matrix pass-through)
fn xform_to_instance(xf: &openmodel::primitives::Xform) -> Instance {
    Instance::from_xform(xf)
}

// Helper: convert an OpenModel PointCloud to PointCloudInstance records used by the point-cloud pipeline
fn convert_pointcloud_to_instances(pointcloud: &PointCloud, out: &mut Vec<PointCloudInstance>) {
    // WEB OPTIMIZATION: Reduce point density by 75% for better performance
    #[cfg(target_arch = "wasm32")]
    let step = 4; // Render every 4th point on web
    #[cfg(not(target_arch = "wasm32"))]
    let step = 1; // Render all points on native
    
    for (i, p) in pointcloud.points.iter().enumerate().step_by(step) {
        let color = if i < pointcloud.colors.len() {
            let c = &pointcloud.colors[i];
            [c.r as f32 / 255.0, c.g as f32 / 255.0, c.b as f32 / 255.0]
        } else {
            [0.8, 0.8, 0.8]
        };
        out.push(PointCloudInstance {
            position: [p.x as f32, p.y as f32, p.z as f32],
            color,
            size: 1.0,
        });
    }
}

// Helper: generate a small test set of point cloud instances (3x3x3 grid)
fn create_test_pointcloud_instances() -> Vec<PointCloudInstance> {
    let mut out = Vec::new();
    // WEB OPTIMIZATION: Reduce test point cloud size
    #[cfg(target_arch = "wasm32")]
    let n = 2; // 2x2x2 = 8 points on web
    #[cfg(not(target_arch = "wasm32"))]
    let n = 3; // 3x3x3 = 27 points on native
    
    let spacing = 1.0f32;
    let offset = -((n as f32 - 1.0) * spacing * 0.5);
    for x in 0..n {
        for y in 0..n {
            for z in 0..n {
                out.push(PointCloudInstance {
                    position: [
                        offset + x as f32 * spacing,
                        offset + y as f32 * spacing,
                        offset + z as f32 * spacing,
                    ],
                    color: [1.0, 0.7, 0.2],
                    size: 1.0,
                });
            }
        }
    }
    out
}

// Helper: convert point cloud to instances for instanced rendering
// Convert point cloud data to unified geometry (vertices + indices + instances)
fn create_pointcloud_geometry_from_data(pointclouds: &[openmodel::geometry::PointCloud]) -> (Vec<Vertex>, Vec<u16>, Vec<Instance>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut instances = Vec::new();
    
    // Create shared quad geometry for all point cloud instances
    let quad_vertices = [
        Vertex { position: [-0.5, -0.5, 0.0], color: [1.0, 1.0, 1.0], normal: [0.0, 0.0, 1.0] },
        Vertex { position: [ 0.5, -0.5, 0.0], color: [1.0, 1.0, 1.0], normal: [0.0, 0.0, 1.0] },
        Vertex { position: [ 0.5,  0.5, 0.0], color: [1.0, 1.0, 1.0], normal: [0.0, 0.0, 1.0] },
        Vertex { position: [-0.5,  0.5, 0.0], color: [1.0, 1.0, 1.0], normal: [0.0, 0.0, 1.0] },
    ];
    let quad_indices = [0, 1, 2, 2, 3, 0];
    
    vertices.extend_from_slice(&quad_vertices);
    indices.extend_from_slice(&quad_indices);
    
    // Create instances for each point
    for pointcloud in pointclouds {
        for (i, point) in pointcloud.points.iter().enumerate() {
            let _color = if i < pointcloud.colors.len() {
                let c = &pointcloud.colors[i];
                [c.r as f32, c.g as f32, c.b as f32]
            } else {
                [0.8, 0.8, 0.8] // Default gray
            };
            
            // Create transform matrix: translation to point position + scale for size
            let size = 0.1; // Point size
            let transform = [
                [size, 0.0, 0.0, 0.0],
                [0.0, size, 0.0, 0.0], 
                [0.0, 0.0, size, 0.0],
                [point.x as f32, point.y as f32, point.z as f32, 1.0],
            ];
            
            instances.push(Instance { model: transform });
        }
    }
    
    (vertices, indices, instances)
}
// Helper: create test point cloud geometry using unified system
fn create_test_pointcloud_geometry() -> (Vec<Vertex>, Vec<u16>, Vec<Instance>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut instances = Vec::new();
    
    // Create shared quad geometry
    let quad_vertices = [
        Vertex { position: [-0.5, -0.5, 0.0], color: [1.0, 1.0, 1.0], normal: [0.0, 0.0, 1.0] },
        Vertex { position: [ 0.5, -0.5, 0.0], color: [1.0, 1.0, 1.0], normal: [0.0, 0.0, 1.0] },
        Vertex { position: [ 0.5,  0.5, 0.0], color: [1.0, 1.0, 1.0], normal: [0.0, 0.0, 1.0] },
        Vertex { position: [-0.5,  0.5, 0.0], color: [1.0, 1.0, 1.0], normal: [0.0, 0.0, 1.0] },
    ];
    let quad_indices = [0, 1, 2, 2, 3, 0];
    
    vertices.extend_from_slice(&quad_vertices);
    indices.extend_from_slice(&quad_indices);
    
    let size = 3;
    let spacing = 1.0f32;
    let offset = -((size as f32 - 1.0) * spacing * 0.5);
    
    for x in 0..size {
        for y in 0..size {
            for z in 0..size {
                let position = [
                    offset + x as f32 * spacing,
                    offset + y as f32 * spacing,
                    offset + z as f32 * spacing,
                ];
                
                let point_size = 0.1;
                let transform = [
                    [point_size, 0.0, 0.0, 0.0],
                    [0.0, point_size, 0.0, 0.0], 
                    [0.0, 0.0, point_size, 0.0],
                    [position[0], position[1], position[2], 1.0],
                ];
                
                instances.push(Instance { model: transform });
            }
        }
    }
    
    // #[cfg(target_arch = "wasm32")]
    // web_sys::console::log_1(&format!("Created {} test point cloud instances", instances.len()).into());
    // #[cfg(not(target_arch = "wasm32"))]
    // println!("Created {} test point cloud instances", instances.len());
    
    (vertices, indices, instances)
}

pub struct State{
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    is_surface_configured: bool,
    // Shader pipelines
    render_pipeline_solid: wgpu::RenderPipeline, // First pipeline (one color)
    render_pipeline_color: wgpu::RenderPipeline, // Second pipeline (vertex colors)
    render_pipeline_lights: wgpu::RenderPipeline, // Lights pipeline (lit surfaces)
    render_pipeline_pointcloud: wgpu::RenderPipeline, // Point cloud glyph pipeline
    pipeline_mode: PipelineMode,                 // Active pipeline selection
    // Pipe rendering controls
    pipe_px_radius: f32,
    // Scene bounds for orthographic camera framing
    scene_bounds: Option<SceneBounds>,
    vertex_buffer: wgpu::Buffer, // We will store data of vertex.rs in this buffer
    index_buffer: wgpu::Buffer, // We will store data of vertex.rs in this buffer
    num_indices: u32,
    // Point cloud rendering buffers (instanced approach)
    pointcloud_quad_buffer: wgpu::Buffer,      // Shared quad geometry
    pointcloud_instance_buffer: wgpu::Buffer,  // Instance data (position, color, size)
    pointcloud_num_instances: u32,
    // Camera system - testing step by step
    camera: Camera,
    camera_uniform: CameraUniform,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    camera_bind_group_layout: wgpu::BindGroupLayout,
    camera_controller: CameraController,
    last_render_time: Instant,
    // Change detection throttle timestamp
    #[cfg(target_arch = "wasm32")]
    last_poll_time: Instant,
    mouse_pressed: bool,
    // default pointer to the window
    window: Arc<Window>,
    // Native-only: background poller delivers geometry here to avoid blocking the render thread
    #[cfg(not(target_arch = "wasm32"))]
    geom_rx: std::sync::mpsc::Receiver<(Vec<Vertex>, Vec<u16>, Vec<DrawBatch>)>,
    // Instance data
    instances: Vec<Instance>,
    instance_buffer: wgpu::Buffer,
    // Per-batch draws (CPU descriptor) and flattened mapping
    batches: Vec<BatchDraw>,
    // ADDED (depth): Depth buffer
    depth_texture: wgpu::Texture,
    depth_view: wgpu::TextureView,
    // ADDED (MSAA): Multisampled color buffer (resolved to surface) and sample count
    msaa_sample_count: u32,
    msaa_color_texture: wgpu::Texture,
    msaa_color_view: wgpu::TextureView,
    // GPU geometry pipeline for compute-based pipe and sphere generation
    gpu_geometry_pipeline: Option<GpuGeometryPipeline>,
    gpu_geometry_bind_group: Option<wgpu::BindGroup>,
    gpu_pipes_data: Vec<PipeTransform>,
    gpu_spheres_data: Vec<SphereTransform>,
    gpu_geometry_scale: f32,
}

impl State{
    // We don't need to be async right now, will implement later
    pub async fn new(window: Arc<Window>, vertices: &[Vertex], indices: &[u16], batches_in: &[DrawBatch], pointcloud_instances: &[PointCloudInstance], pipes: Vec<PipeTransform>, spheres: Vec<SphereTransform>) -> anyhow::Result<Self> {

        let size = window.inner_size();
        // Clamp initial surface size on Web (WebGL2 backend) to avoid exceeding max texture limit.
        // On Web, prefer the canvas's actual internal resolution (width/height attributes)
        // so surface configuration matches the true backing store size set by JS.
        #[allow(unused_variables)]
        let (init_width, init_height) = {
            #[cfg(target_arch = "wasm32")]
            {
                let mut w = size.width.max(1);
                let mut h = size.height.max(1);
                if let Some(win) = web_sys::window() {
                    if let Some(doc) = win.document() {
                        if let Some(el) = doc.get_element_by_id("canvas") {
                            if let Ok(canvas) = el.dyn_into::<web_sys::HtmlCanvasElement>() {
                                let cw = canvas.width();
                                let ch = canvas.height();
                                if cw > 0 && ch > 0 { w = cw; h = ch; }
                            }
                        }
                    }
                }
                const MAX_DIM_GL: u32 = 2048; // conservative safe minimum for WebGL2
                if w > MAX_DIM_GL || h > MAX_DIM_GL {
                    let scale_w = MAX_DIM_GL as f32 / w as f32;
                    let scale_h = MAX_DIM_GL as f32 / h as f32;
                    let s = scale_w.min(scale_h);
                    w = ((w as f32) * s).floor().max(1.0) as u32;
                    h = ((h as f32) * s).floor().max(1.0) as u32;
                }
                (w, h)
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                (size.width, size.height)
            }
        };
       

        //////////////////////////////////////////////////////////////////////////////////////////////////////
        // Normal geometry
        //////////////////////////////////////////////////////////////////////////////////////////////////////

        // The instance is a handle to our GPU
        // BackendBit::PRIMARY => Vulkan + Metal + DX12 + Browser WebGPU
        // The instance is the first thing you create.
        // Its main purpose is to create Adapter(s) and Surface(s).
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            #[cfg(not(target_arch = "wasm32"))]
            backends: wgpu::Backends::PRIMARY,
            #[cfg(target_arch = "wasm32")]
            backends: wgpu::Backends::BROWSER_WEBGPU | wgpu::Backends::GL,
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone()).unwrap();

        // The adapter is a handle for the graphics card.
        // You can get information: graphics card name and adapter type.
        // https://sotrh.github.io/learn-wgpu/beginner/tutorial2-surface/#state-new
        let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }).await?;

        // Inspect chosen adapter and decide limits/MSAA based on backend
        let adapter_info = adapter.get_info();
        #[cfg(target_arch = "wasm32")]
        {
            web_sys::console::log_1(
                &format!("wgpu adapter backend: {:?}, name: {}", adapter_info.backend, adapter_info.name).into(),
            );
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            log::info!("wgpu adapter backend: {:?}, name: {}", adapter_info.backend, adapter_info.name);
        }

        let is_webgl_backend = adapter_info.backend == wgpu::Backend::Gl;
        // Decide MSAA sample count dynamically (WebGPU: 4x, WebGL: 1x)
        let msaa_sample_count: u32 = if cfg!(target_arch = "wasm32") {
            if is_webgl_backend { 1 } else { 4 }
        } else {
            4
        };
        #[cfg(target_arch = "wasm32")]
        web_sys::console::log_1(&format!("MSAA sample count: {}", msaa_sample_count).into());
        #[cfg(not(target_arch = "wasm32"))]
        log::info!("MSAA sample count: {}", msaa_sample_count);

        // Choose appropriate limits per backend
        let required_limits = if cfg!(target_arch = "wasm32") {
            if is_webgl_backend {
                wgpu::Limits::downlevel_webgl2_defaults()
            } else {
                wgpu::Limits::default()
            }
        } else {
            wgpu::Limits::default()
        };

        // Use adapter to create device and queue
        // This current example doesn't use any features.
        // Full list of features: https://docs.rs/wgpu/latest/wgpu/struct.Features.html
        // Full list of limits: https://docs.rs/wgpu/latest/wgpu/struct.Limits.html
        // The memory_hints field provides the adapter with a preferred memory allocation strategy.
        // Memory hints options: https://wgpu.rs/doc/wgpu/enum.MemoryHints.html
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits,
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
            })
            .await?;


        // Here we are defining a config for our surface.
        // This will define how the surface creates its underlying SurfaceTexture in render function.
        let surface_caps = surface.get_capabilities(&adapter);
        // Shader code in this tutorial assumes an sRGB surface texture. Using a different
        // one will result in all the colors coming out darker. If you want to support non
        // sRGB surfaces, you'll need to account for that when drawing to the frame.
        let surface_format = surface_caps
        .formats
        .iter()
        .copied()
        .find(|f| f.is_srgb())
        .unwrap_or(surface_caps.formats[0]);
        
        // The usage field describes how SurfaceTexture will be used.
        // RENDER_ATTACHMENT specifies that the textures will be use to write to the screen.
        // The format defines how SurfaceTexture will be stored on the GPU.
        // The width and the height are in pixels of a SurfaceTexture (width and height of the window and never 0).
        // Present mode determines how to sync the surface with the display.
        // If you do not want runtime selection, PresenModel::Fifo will cap the display rate at the display's framerate.
        // Or use other options: https://docs.rs/wgpu/latest/wgpu/enum.PresentMode.html
        // The alpha_mode field defines how the alpha channel of the surface will be handled.
        // view_formats is a list of TextureForms that you can use when creating TextureViews.
        let present_mode = {
            #[cfg(target_arch = "wasm32")]
            {
                if surface_caps.present_modes.contains(&wgpu::PresentMode::AutoVsync) {
                    wgpu::PresentMode::AutoVsync
                } else if surface_caps.present_modes.contains(&wgpu::PresentMode::Fifo) {
                    wgpu::PresentMode::Fifo
                } else {
                    surface_caps.present_modes[0]
                }
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                if surface_caps.present_modes.contains(&wgpu::PresentMode::Fifo) {
                    wgpu::PresentMode::Fifo
                } else {
                    surface_caps.present_modes[0]
                }
            }
        };
        // Prefer opaque alpha to prevent page background showing through during resize, when supported
        let alpha_mode = if surface_caps
            .alpha_modes
            .contains(&wgpu::CompositeAlphaMode::Opaque)
        {
            wgpu::CompositeAlphaMode::Opaque
        } else {
            surface_caps.alpha_modes[0]
        };
        let desired_latency = if cfg!(target_arch = "wasm32") { 1 } else { 2 };
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: init_width,
            height: init_height,
            present_mode,
            alpha_mode,
            view_formats: vec![],
            desired_maximum_frame_latency: desired_latency,
        };

        ////////////////////////////////////////////////////////////////////////////////////////////////////////////
        // SHADERS
        ////////////////////////////////////////////////////////////////////////////////////////////////////////////
        // Debug: capture validation errors during pipeline/build steps (especially useful on Web)
        // Push one scope before creating pipelines; we'll pop it after both are created.
        device.push_error_scope(wgpu::ErrorFilter::Validation);

        // Pipeline layout - testing camera bind group step by step
        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("camera_bind_group_layout"),
            });

        // Pipelines via modules (unified instancing preserved)
        let render_pipeline_solid = crate::shader_solid_pipeline::create(
            &device, &config, &camera_bind_group_layout, DEPTH_FORMAT, msaa_sample_count,
        );

        let render_pipeline_color = crate::shader_color_pipeline::create(
            &device, &config, &camera_bind_group_layout, DEPTH_FORMAT, msaa_sample_count,
        );

        let render_pipeline_lights = crate::shader_lights_pipeline::create(
            &device, &config, &camera_bind_group_layout, DEPTH_FORMAT, msaa_sample_count,
        );

        let render_pipeline_pointcloud = crate::shader_pointcloud_pipeline::create(
            &device, &config, &camera_bind_group_layout, DEPTH_FORMAT, msaa_sample_count,
        );

        // Pop and log any validation errors that might have occurred during pipeline creation
        #[cfg(target_arch = "wasm32")]
        if let Some(err) = device.pop_error_scope().await {
            web_sys::console::error_1(&format!("WGPU validation (pipeline): {:?}", err).into());
        }
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(err) = device.pop_error_scope().await {
            eprintln!("WGPU validation (pipeline): {:?}", err);
        }

        // Native: spawn a background poller thread that watches for local geometry file changes (mtime)
        // and sends rebuilt vertex/index buffers through a channel. This prevents UI freezes from blocking I/O.
        #[cfg(not(target_arch = "wasm32"))]
        let (tx_geom, rx_geom) = std::sync::mpsc::channel::<(Vec<Vertex>, Vec<u16>, Vec<DrawBatch>)>();

        #[cfg(not(target_arch = "wasm32"))]
        {
            use std::time::Duration as StdDuration;
            std::thread::spawn(move || {
                let mut last_local_mtime: Option<std::time::SystemTime> = None;
                loop {
                    let mut changed = false;

                    // Local file mtime check
                    if let Ok(meta) = std::fs::metadata(LOCAL_GEOMETRY_PATH) {
                        if let Ok(mtime) = meta.modified() {
                            if last_local_mtime.map_or(true, |prev| prev != mtime) {
                                last_local_mtime = Some(mtime);
                                changed = true;
                            }
                        }
                    }

                    if changed {
                        let (vertices, indices, batches) = pollster::block_on(get_geometry());
                        let _ = tx_geom.send((vertices, indices, batches));
                    }
                    std::thread::sleep(StdDuration::from_millis(GEOMETRY_POLL_INTERVAL_MS));
                }
            });
        }

        // Create GPU buffers from provided geometry
        let vertex_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(vertices),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );

        let index_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(indices),
                usage: wgpu::BufferUsages::INDEX,
            }
        );
        
        let num_indices = indices.len() as u32;
        
        // Create shared quad geometry buffer for point cloud rendering
        let pointcloud_quad_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Point Cloud Quad Buffer"),
            contents: bytemuck::cast_slice(QuadVertex::VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });
        
        // Create point cloud instance buffer
        let pointcloud_instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Point Cloud Instance Buffer"),
            contents: bytemuck::cast_slice(pointcloud_instances),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
        let pointcloud_num_instances = pointcloud_instances.len() as u32;
        
        #[cfg(target_arch = "wasm32")]
        web_sys::console::log_1(&format!("Initialized point cloud with {} instances", pointcloud_num_instances).into());
        #[cfg(not(target_arch = "wasm32"))]
        println!("Initialized point cloud with {} instances", pointcloud_num_instances);
        
        // Instances come from batches below (default identity per-batch if none provided)

        // Flatten instances and build per-batch draw info
        let mut flat_instances: Vec<Instance> = Vec::new();
        let mut batch_draws: Vec<BatchDraw> = Vec::new();

        for b in batches_in {
            // default: one identity if no transforms provided
            let insts: Vec<Instance> = if b.instances.is_empty() {
                vec![Instance::identity()]
            } else {
                b.instances.clone()
            };

            let instance_offset = flat_instances.len() as u32;
            let instance_count = insts.len() as u32;
            flat_instances.extend(insts.into_iter());

            batch_draws.push(BatchDraw {
                first_index: b.first_index,
                index_count: b.index_count,
                base_vertex: b.base_vertex,
                instance_offset,
                instance_count,
                kind: b.kind,
            });
        }

        let instance_data = flat_instances.iter().map(Instance::to_raw).collect::<Vec<_>>();
        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Instance Buffer"),
            contents: bytemuck::cast_slice(&instance_data),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        // Initialize camera system
        let camera = Camera::new(init_width as f32, init_height as f32);
        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_view_proj(&camera);
        // Initialize extended view/pipe parameters
        let default_pipe_px_radius: f32 = 0.5;
        camera_uniform.set_eye_dir(&camera);
        camera_uniform.set_view_params(
            config.width as f32,
            config.height as f32,
            camera.fovy,
            camera.aspect,
            default_pipe_px_radius,
            camera.is_ortho,
            camera.ortho_half_height,
        );

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Debug: log the first row of the view-proj on Web to ensure it isn't zeros/NaNs
        #[cfg(target_arch = "wasm32")]
        {
            web_sys::console::log_1(&format!("Camera VP row0: {:?}", camera_uniform.view_proj[0]).into());
        }

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        });

        let camera_controller = CameraController::new(4.0, 0.4);

        // ADDED (depth): Create depth texture matching the surface size, with MSAA
        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
            size: wgpu::Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: msaa_sample_count,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // ADDED (MSAA): Create multisampled color target (resolved to surface each frame)
        let msaa_color_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("MSAA Color Texture"),
            size: wgpu::Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: msaa_sample_count,
            dimension: wgpu::TextureDimension::D2,
            format: config.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let msaa_color_view = msaa_color_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Compute scene bounds from vertices for orthographic camera framing
        let scene_bounds = if !vertices.is_empty() {
            let mut bounds = SceneBounds::new();
            for vertex in vertices {
                bounds.expand_point(cgmath::Point3::new(
                    vertex.position[0],
                    vertex.position[1],
                    vertex.position[2],
                ));
            }
            if bounds.is_valid() {
                Some(bounds)
            } else {
                None
            }
        } else {
            None
        };

        // Now that we configured our render surface.
        // We can create the struct State with its arguments.
        let mut state = Self {
            surface,
            device,
            queue,
            config,
            is_surface_configured: false,
            // Pipelines
            render_pipeline_solid,
            render_pipeline_color,
            render_pipeline_lights,
            render_pipeline_pointcloud,
            pipeline_mode: PipelineMode::Color,
            pipe_px_radius: 10.5,
            scene_bounds,
            vertex_buffer,
            index_buffer,
            num_indices,
            pointcloud_quad_buffer,
            pointcloud_instance_buffer,
            pointcloud_num_instances,
            camera,
            camera_uniform,
            camera_buffer,
            camera_bind_group,
            camera_bind_group_layout,
            camera_controller,
            // Instance data
            instances: flat_instances,
            instance_buffer,
            batches: batch_draws,
            // Last render time
            last_render_time: Instant::now(),
            #[cfg(target_arch = "wasm32")]
            last_poll_time: Instant::now(),
            // Mouse state
            mouse_pressed: false,
            // Window
            window,
            // Geometry receiver
            #[cfg(not(target_arch = "wasm32"))]
            geom_rx: rx_geom,
            // ADDED (depth): Depth buffer fields (texture + view)
            depth_texture,
            depth_view,
            // ADDED (MSAA)
            msaa_sample_count: msaa_sample_count,
            msaa_color_texture,
            msaa_color_view,
            // GPU geometry pipeline (initially None, will be initialized later if needed)
            gpu_geometry_pipeline: None,
            gpu_geometry_bind_group: None,
            gpu_pipes_data: pipes,
            gpu_spheres_data: spheres,
            gpu_geometry_scale: 1.0,
        };
        // Configure surface immediately to avoid first-frame issues
        state.resize(size.width, size.height);
        Ok(state)
    }

    // Initialize GPU geometry pipeline for compute-based pipe and sphere generation
    fn init_gpu_geometry_pipeline(&mut self) {
        if self.gpu_geometry_pipeline.is_none() {
            let pipeline = GpuGeometryPipeline::new(
                &self.device, 
                &self.config, 
                &self.camera_bind_group_layout,
                DEPTH_FORMAT,
                self.msaa_sample_count
            );
            self.gpu_geometry_pipeline = Some(pipeline);
        }
    }

    // This function is no longer needed as we use transformation matrices directly

    // Update GPU geometry data and recreate buffers if needed
    fn update_gpu_geometry_data(&mut self, pipes: Vec<PipeTransform>, spheres: Vec<SphereTransform>) {
        self.gpu_pipes_data = pipes;
        self.gpu_spheres_data = spheres;
        
        if let Some(pipeline) = &self.gpu_geometry_pipeline {
            let bind_group = pipeline.update_data(
                &self.device,
                self.apply_scale_to_pipes(self.gpu_pipes_data.clone()),
                self.apply_scale_to_spheres(self.gpu_spheres_data.clone()),
            );
            self.gpu_geometry_bind_group = Some(bind_group);
        }
    }
    
    // Apply uniform scaling to pipe transformation matrices
    fn apply_scale_to_pipes(&self, mut pipes: Vec<PipeTransform>) -> Vec<PipeTransform> {
        for pipe in &mut pipes {
            // Apply radial scaling to the transformation matrix
            // Scale only X, Y components (radius), not Z (length)
            pipe.transform[0][0] *= self.gpu_geometry_scale;
            pipe.transform[1][1] *= self.gpu_geometry_scale;
            // Do NOT scale Z component: pipe.transform[2][2] *= self.gpu_geometry_scale;
        }
        pipes
    }
    
    // Apply uniform scaling to sphere transformation matrices
    fn apply_scale_to_spheres(&self, mut spheres: Vec<SphereTransform>) -> Vec<SphereTransform> {
        for sphere in &mut spheres {
            // Apply uniform scaling to the transformation matrix
            // Scale the X, Y, Z components (diagonal elements)
            sphere.transform[0][0] *= self.gpu_geometry_scale;
            sphere.transform[1][1] *= self.gpu_geometry_scale;
            sphere.transform[2][2] *= self.gpu_geometry_scale;
        }
        spheres
    }
    
    // Update GPU geometry scale and refresh buffers
    fn update_gpu_geometry_scale(&mut self) {
        if let Some(pipeline) = &self.gpu_geometry_pipeline {
            let bind_group = pipeline.update_data(
                &self.device,
                self.apply_scale_to_pipes(self.gpu_pipes_data.clone()),
                self.apply_scale_to_spheres(self.gpu_spheres_data.clone()),
            );
            self.gpu_geometry_bind_group = Some(bind_group);
        }
    }

    // Update point cloud instance buffer with new data (only if different)
    fn update_pointcloud_instances(&mut self, pointcloud_instances: &[PointCloudInstance]) {
        let new_count = pointcloud_instances.len() as u32;
        
        // Skip update if count is the same (assume data is identical)
        if new_count == self.pointcloud_num_instances {
            // Reduce logging frequency on web to improve performance
            #[cfg(target_arch = "wasm32")]
            {
                static mut LOG_COUNTER: u32 = 0;
                unsafe {
                    LOG_COUNTER += 1;
                    if LOG_COUNTER % 100 == 0 { // Log every 100th skip
                        web_sys::console::log_1(&format!("⏭️ SKIPPED {} point cloud buffer updates: {} instances unchanged", LOG_COUNTER, new_count).into());
                    }
                }
            }
            #[cfg(not(target_arch = "wasm32"))]
            println!("⏭️ SKIPPING point cloud buffer update: {} instances unchanged", new_count);
            return;
        }
        
        // Create new instance buffer only when needed
        let pointcloud_instance_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Point Cloud Instance Buffer"),
            contents: bytemuck::cast_slice(pointcloud_instances),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
        
        #[cfg(target_arch = "wasm32")]
        web_sys::console::log_1(&format!("🔄 UPDATING point cloud buffer: {} -> {} instances", self.pointcloud_num_instances, new_count).into());
        #[cfg(not(target_arch = "wasm32"))]
        println!("🔄 UPDATING point cloud buffer: {} -> {} instances", self.pointcloud_num_instances, new_count);

        self.pointcloud_instance_buffer = pointcloud_instance_buffer;
        self.pointcloud_num_instances = new_count;
        
        #[cfg(target_arch = "wasm32")]
        web_sys::console::log_1(&format!("✅ Point cloud buffer updated successfully: {} instances", self.pointcloud_num_instances).into());
        #[cfg(not(target_arch = "wasm32"))]
        println!("✅ Point cloud buffer updated successfully: {} instances", self.pointcloud_num_instances);
    }

    // Replace entire scene: geometry + instance batches
    fn replace_scene(&mut self, vertices: &[Vertex], indices: &[u16], batches_in: &[DrawBatch]) {
        // When replace_scene is called without point clouds, we need to regenerate them
        // This happens when the scene is updated from sources that don't include point cloud data
        #[cfg(target_arch = "wasm32")]
        web_sys::console::log_1(&"⚠️ replace_scene called without point clouds - this will clear them!".into());
        #[cfg(not(target_arch = "wasm32"))]
        println!("⚠️ replace_scene called without point clouds - this will clear them!");
        
        self.replace_scene_with_pointclouds(vertices, indices, batches_in, &[]);
    }

    // Replace entire scene including point clouds
    fn replace_scene_with_pointclouds(&mut self, vertices: &[Vertex], indices: &[u16], batches_in: &[DrawBatch], pointcloud_instances: &[PointCloudInstance]) {
        #[cfg(target_arch = "wasm32")]
        web_sys::console::log_1(&format!("🔄 REPLACING SCENE: {} vertices, {} indices, {} batches, {} point cloud instances", vertices.len(), indices.len(), batches_in.len(), pointcloud_instances.len()).into());
        #[cfg(not(target_arch = "wasm32"))]
        println!("🔄 REPLACING SCENE: {} vertices, {} indices, {} batches, {} point cloud instances", vertices.len(), indices.len(), batches_in.len(), pointcloud_instances.len());
        // Replace vertex/index buffers
        let new_vertex_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let new_index_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        self.vertex_buffer = new_vertex_buffer;
        self.index_buffer = new_index_buffer;
        self.num_indices = indices.len() as u32;

        // Rebuild flattened instances and batch draws
        let mut flat_instances: Vec<Instance> = Vec::new();
        let mut batch_draws: Vec<BatchDraw> = Vec::new();

        for b in batches_in {
            // default: one identity if no transforms provided
            let insts: Vec<Instance> = if b.instances.is_empty() {
                vec![Instance::identity()]
            } else {
                b.instances.clone()
            };

            let instance_offset = flat_instances.len() as u32;
            let instance_count = insts.len() as u32;
            flat_instances.extend(insts.into_iter());

            batch_draws.push(BatchDraw {
                first_index: b.first_index,
                index_count: b.index_count,
                base_vertex: b.base_vertex,
                instance_offset,
                instance_count,
                kind: b.kind,
            });
        }

        let instance_data = flat_instances.iter().map(Instance::to_raw).collect::<Vec<_>>();
        let new_instance_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Instance Buffer"),
            contents: bytemuck::cast_slice(&instance_data),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
        // Update point cloud buffer only if new data is provided; otherwise preserve existing
        if !pointcloud_instances.is_empty() {
            #[cfg(target_arch = "wasm32")]
            web_sys::console::log_1(&format!("📊 Point cloud instances provided: {}", pointcloud_instances.len()).into());
            #[cfg(not(target_arch = "wasm32"))]
            println!("📊 Point cloud instances provided: {}", pointcloud_instances.len());
            
            self.update_pointcloud_instances(pointcloud_instances);
        } else {
            #[cfg(target_arch = "wasm32")]
            web_sys::console::log_1(&format!("⚠️ NO point cloud instances provided - keeping existing {} instances", self.pointcloud_num_instances).into());
            #[cfg(not(target_arch = "wasm32"))]
            println!("⚠️ NO point cloud instances provided - keeping existing {} instances", self.pointcloud_num_instances);
            
            // CRITICAL: Add point cloud batch back to batches if it doesn't exist
            let has_pointcloud_batch = batch_draws.iter().any(|b| matches!(b.kind, BatchKind::PointCloud));
            if !has_pointcloud_batch && self.pointcloud_num_instances > 0 {
                #[cfg(target_arch = "wasm32")]
                web_sys::console::log_1(&"🔧 Adding missing point cloud batch to preserve rendering".into());
                #[cfg(not(target_arch = "wasm32"))]
                println!("🔧 Adding missing point cloud batch to preserve rendering");
                
                batch_draws.push(BatchDraw {
                    first_index: 0,
                    index_count: 0,
                    base_vertex: 0,
                    instance_offset: 0,
                    instance_count: 0,
                    kind: BatchKind::PointCloud,
                });
            }
        }

        self.instances = flat_instances;
        self.instance_buffer = new_instance_buffer;
        self.batches = batch_draws;

        // Recompute scene bounds after geometry update
        self.scene_bounds = if !vertices.is_empty() {
            let mut bounds = SceneBounds::new();
            for vertex in vertices {
                bounds.expand_point(cgmath::Point3::new(
                    vertex.position[0],
                    vertex.position[1],
                    vertex.position[2],
                ));
            }
            if bounds.is_valid() {
                Some(bounds)
            } else {
                None
            }
        } else {
            None
        };

        #[cfg(target_arch = "wasm32")]
        {
            web_sys::console::log_1(&"Scene reloaded".into());
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            log::info!("Scene reloaded");
        }
    }

    // Check for geometry changes and reload if needed
    fn poll_geometry_changes(&mut self) {
        // Native: drain background updates without throttling or blocking the render thread
        #[cfg(not(target_arch = "wasm32"))]
        {
            while let Ok((vertices, indices, batches)) = self.geom_rx.try_recv() {
                // Preserve existing point cloud instances during native updates
                // Don't regenerate - keep the current point cloud data
                println!("🔄 Native geometry update - preserving {} existing point cloud instances", self.pointcloud_num_instances);
                
                // Create empty vec to preserve existing point clouds
                let empty_pointcloud_instances = Vec::new();
                self.replace_scene_with_pointclouds(&vertices, &indices, &batches, &empty_pointcloud_instances);
            }
            return;
        }

        // WASM: throttled async polling and apply results prepared by the task
        #[cfg(target_arch = "wasm32")]
        {
            let now = Instant::now();
            let elapsed = now.duration_since(self.last_poll_time);
            let web_poll_interval = GEOMETRY_POLL_INTERVAL_MS * 3; // 3x slower polling on web
            if elapsed.as_millis() < web_poll_interval as u128 {
                return; // Skip this frame
            }
            self.last_poll_time = now;
            // If a fetch is already running, just try to apply pending result
            let already_fetching = LOCAL_FETCHING.with(|f| f.get());
            if !already_fetching {
                LOCAL_FETCHING.with(|f| f.set(true));
                // Spawn async poll for local JSON; only apply if content hash changed
                spawn_local(async move {
                    // Fetch local-served JSON
                    let local_text = fetch_text(LOCAL_GEOMETRY_HTTP_PATH).await;
                    let local_changed = if let Some(ref t) = local_text {
                        let new_hash = fnv1a64(t.as_bytes());
                        LOCAL_HASH.with(|h| {
                            let mut hb = h.borrow_mut();
                            if hb.map_or(true, |old| old != new_hash) { *hb = Some(new_hash); true } else { false }
                        })
                    } else { false };
                    
                    if local_changed {
                        // Rebuild full scene using the same logic as initial load
                        let (vertices, indices, batches, pointcloud_instances) = get_geometry_with_pointclouds().await;
                        #[cfg(target_arch = "wasm32")]
                        web_sys::console::log_1(&format!("🔄 FILE CHANGED - Reloading geometry: {} vertices, {} indices, {} batches, {} point cloud instances", vertices.len(), indices.len(), batches.len(), pointcloud_instances.len()).into());
                        PENDING_GEOMETRY.with(|p| *p.borrow_mut() = Some((vertices, indices, batches, pointcloud_instances)));
                        web_sys::console::log_1(&"Geometry changed; source: local".into());
                    }
                    LOCAL_FETCHING.with(|f| f.set(false));
                });
            }

            // Apply any pending geometry prepared by the async task
            let pending = PENDING_GEOMETRY.with(|p| p.borrow_mut().take());
            if let Some((vertices, indices, batches, pointcloud_instances)) = pending {
                #[cfg(target_arch = "wasm32")]
                web_sys::console::log_1(&format!("🔄 APPLYING PENDING GEOMETRY: {} vertices, {} indices, {} batches, {} point cloud instances", vertices.len(), indices.len(), batches.len(), pointcloud_instances.len()).into());
                #[cfg(not(target_arch = "wasm32"))]
                println!("🔄 APPLYING PENDING GEOMETRY: {} vertices, {} indices, {} batches, {} point cloud instances", vertices.len(), indices.len(), batches.len(), pointcloud_instances.len());
                
                self.replace_scene_with_pointclouds(&vertices, &indices, &batches, &pointcloud_instances);
            }
        }
    }

    pub fn resize(&mut self, width: u32, height: u32){
        // If we want to resize the window, we need to update the surface,
        // every time we resize the window.
        // This was the reason we store size and config to configure the surface.
        if width > 0 && height > 0 {
            // Clamp for WebGL2 backend to avoid creating oversized textures on high-DPR/fullscreen
            #[cfg(target_arch = "wasm32")]
            let (w_clamped, h_clamped) = {
                let mut w = width;
                let mut h = height;
                const MAX_DIM_GL: u32 = 2048; // conservative safe minimum across WebGL2
                if w > MAX_DIM_GL || h > MAX_DIM_GL {
                    let scale_w = MAX_DIM_GL as f32 / w as f32;
                    let scale_h = MAX_DIM_GL as f32 / h as f32;
                    let s = scale_w.min(scale_h);
                    w = ((w as f32) * s).floor().max(1.0) as u32;
                    h = ((h as f32) * s).floor().max(1.0) as u32;
                }
                (w, h)
            };
            #[cfg(not(target_arch = "wasm32"))]
            let (w_clamped, h_clamped) = (width, height);

            self.config.width = w_clamped;
            self.config.height = h_clamped;
            self.surface.configure(&self.device, &self.config);
            // Keep camera projection in sync with the surface size (important on Web)
            self.camera.aspect = self.config.width as f32 / self.config.height as f32;
            // ADDED (depth): Recreate depth texture to match new size (respect MSAA)
            self.depth_texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Depth Texture"),
                size: wgpu::Extent3d {
                    width: self.config.width,
                    height: self.config.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: self.msaa_sample_count,
                dimension: wgpu::TextureDimension::D2,
                format: DEPTH_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            self.depth_view = self.depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

            // ADDED (MSAA): Recreate multisampled color target
            self.msaa_color_texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("MSAA Color Texture"),
                size: wgpu::Extent3d {
                    width: self.config.width,
                    height: self.config.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: self.msaa_sample_count,
                dimension: wgpu::TextureDimension::D2,
                format: self.config.format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            self.msaa_color_view = self.msaa_color_texture.create_view(&wgpu::TextureViewDescriptor::default());
            self.is_surface_configured = true;
        }
    }

    fn input(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(key),
                        state,
                        ..
                    },
                ..
            } => {
                // Intercept 'N' and 'M' to adjust pipe thickness (in pixels)
                if *state == ElementState::Pressed {
                    match key {
                        KeyCode::KeyN => {
                            // Decrease pipe pixel radius, clamp to a sensible minimum
                            self.pipe_px_radius = self.pipe_px_radius * 0.9;
                            // Update camera uniform immediately; the render loop also updates each frame
                            self.camera_uniform.set_eye_dir(&self.camera);
                            self.camera_uniform.set_view_params(
                                self.config.width as f32,
                                self.config.height as f32,
                                self.camera.fovy,
                                self.camera.aspect,
                                self.pipe_px_radius,
                                self.camera.is_ortho,
                                self.camera.ortho_half_height,
                            );
                            self.queue.write_buffer(
                                &self.camera_buffer,
                                0,
                                bytemuck::cast_slice(&[self.camera_uniform]),
                            );
                            true
                        }
                        KeyCode::KeyM => {
                            // Increase pipe pixel radius, clamp to a sensible maximum
                            self.pipe_px_radius = self.pipe_px_radius * 1.1111;
                            // Update camera uniform immediately; the render loop also updates each frame
                            self.camera_uniform.set_eye_dir(&self.camera);
                            self.camera_uniform.set_view_params(
                                self.config.width as f32,
                                self.config.height as f32,
                                self.camera.fovy,
                                self.camera.aspect,
                                self.pipe_px_radius,
                                self.camera.is_ortho,
                                self.camera.ortho_half_height,
                            );
                            self.queue.write_buffer(
                                &self.camera_buffer,
                                0,
                                bytemuck::cast_slice(&[self.camera_uniform]),
                            );
                            true
                        }
                        _ => self.camera_controller.process_keyboard(*key, *state),
                    }
                } else {
                    self.camera_controller.process_keyboard(*key, *state)
                }
            },
            WindowEvent::MouseWheel { delta, .. } => {
                self.camera_controller.process_scroll(delta);
                true
            }
            WindowEvent::MouseInput { button, state, .. } => {
                // Handle right mouse for general mouse_pressed tracking
                if *button == MouseButton::Right {
                    self.mouse_pressed = *state == ElementState::Pressed;
                }
                // Use the camera controller's proper mouse button handler
                self.camera_controller.process_mouse_button(*state, *button)
            }
            _ => false,
        }
    }

    fn update(&mut self) {
        // Poll for geometry changes periodically and hot-reload buffers if needed
        self.poll_geometry_changes();
        // WASM: keep surface config synced with actual canvas backing size set by JS
        #[cfg(target_arch = "wasm32")]
        {
            if let Some(win) = web_sys::window() {
                if let Some(doc) = win.document() {
                    if let Some(el) = doc.get_element_by_id("canvas") {
                        if let Ok(canvas) = el.dyn_into::<web_sys::HtmlCanvasElement>() {
                            let cw = canvas.width();
                            let ch = canvas.height();
                            if cw > 0 && ch > 0 && (cw != self.config.width || ch != self.config.height) {
                                self.resize(cw, ch);
                            }
                        }
                    }
                }
            }
        }
        let now = Instant::now();
        let dt = now - self.last_render_time;
        self.last_render_time = now;
        self.camera_controller.update_camera(&mut self.camera, dt);
        self.camera_uniform.update_view_proj(&self.camera);
        // Update extended camera/pipe parameters each frame
        self.camera_uniform.set_eye_dir(&self.camera);
        self.camera_uniform.set_view_params(
            self.config.width as f32,
            self.config.height as f32,
            self.camera.fovy,
            self.camera.aspect,
            self.pipe_px_radius,
            self.camera.is_ortho,
            self.camera.ortho_half_height,
        );
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[self.camera_uniform]),
        );
    }

    /// Apply an external OpenModel world_from_camera transform to the camera
    /// and refresh the uniform buffer immediately.
    pub fn apply_camera_om_xform(&mut self, xf: openmodel::primitives::Xform) {
        self.camera.set_om_world_from_camera(xf);
        // Rebuild view-projection and extended fields from the updated camera
        self.camera_uniform.update_view_proj(&self.camera);
        self.camera_uniform.set_eye_dir(&self.camera);
        self.camera_uniform.set_view_params(
            self.config.width as f32,
            self.config.height as f32,
            self.camera.fovy,
            self.camera.aspect,
            self.pipe_px_radius,
            self.camera.is_ortho,
            self.camera.ortho_half_height,
        );
        // Write updated uniform to GPU so the effect is visible this frame
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[self.camera_uniform]),
        );
    }

    /// Get scene bounds for camera framing
    pub fn get_scene_bounds(&self) -> Option<(cgmath::Point3<f32>, cgmath::Vector3<f32>)> {
        self.scene_bounds.map(|bounds| (bounds.center(), bounds.size()))
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        self.window.request_redraw(); // We ask the window to draw another frame

        // We cannot render unless the surface is configured
        if !self.is_surface_configured {
            return Ok(());
        }

        // Check for geometry changes and reload if needed
        self.poll_geometry_changes();

        // The get_current_texture() function will wait for the surface to provide a new surface texture. 
        // Will store it in the output variable for later use.
        let output = self.surface.get_current_texture()?;

        // This creates a TextureView with default settings.
        // We need to do this because we want to control how the rende code interacts with the texture.
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        // We also need a CommandEncoder to create the actual commands to send to GPU.
        // Most modern graphics frameworks expect commands to to be stored in a command buffer before sending to GPU.
        // The encoder builds a command buffer that we can then send to the GPU.
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        // Initialize GPU geometry pipeline if needed
        self.init_gpu_geometry_pipeline();

        // Use cached GPU geometry data from the state
        let pipes = self.gpu_pipes_data.clone();
        let spheres = self.gpu_spheres_data.clone();
        
        #[cfg(target_arch = "wasm32")]
        web_sys::console::log_1(&format!("📊 Creating GPU geometry: {} pipes, {} spheres", pipes.len(), spheres.len()).into());
        #[cfg(not(target_arch = "wasm32"))]
        println!("📊 Creating GPU geometry: {} pipes, {} spheres", pipes.len(), spheres.len());
        
        self.update_gpu_geometry_data(pipes, spheres);

        // GPU geometry pipeline now generates geometry directly in vertex shader - no compute dispatch needed

        // Clearing the screen.
        // We need to use the encoder to create a RenderPass.
        // The RenderPass has all the methods for the actual drawing.
        // The render method via shaders will draw the geometry.
        {
            // Always clear to a stable gray to avoid undefined loads during rapid resize
            let color_load_op = wgpu::LoadOp::Clear(wgpu::Color { r: 0.9, g: 0.9, b: 0.9, a: 1.0 });
            // Choose color attachment and optional resolve target based on MSAA
            let (color_view, resolve_target): (&wgpu::TextureView, Option<&wgpu::TextureView>) = if self.msaa_sample_count > 1 {
                (&self.msaa_color_view, Some(&view))
            } else {
                (&view, None)
            };

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: color_view,
                    resolve_target,
                    ops: wgpu::Operations {
                        load: color_load_op,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                // ADDED (depth): Attach depth buffer with clear=1.0
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });


            // Bind shared vertex buffer (slot 0) once
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            // We'll set the instance buffer (slot 1) per-batch below

            // You can only have one index buffer set at a time.
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16); // 1.

            // When using an index buffer, we need to use draw_indexed instead of draw.
            // First argument is the range of indices to draw.
            // Second argument is the base vertex.
            // Third argument is the instance count.
            let stride = std::mem::size_of::<InstanceRaw>() as u64;
            
            #[cfg(not(target_arch = "wasm32"))]
            {
                // let pointcloud_batches = self.batches.iter().filter(|b| matches!(b.kind, BatchKind::PointCloud)).count();
                // if pointcloud_batches > 0 {
                //     println!("Found {} point cloud batches in render loop", pointcloud_batches);
                // }
            }
            
            for d in &self.batches {
                // Integrated rendering:
                // - Pipe batches render in BOTH Solid and Color modes using the pipe pipeline.
                // - Surface batches render with Solid or Color depending on current mode.
                match d.kind {
                    BatchKind::Pipe => {
                        // Skip CPU-generated pipe instances - using GPU geometry instead
                        continue;
                    }
                    BatchKind::Surface => {
                        match self.pipeline_mode {
                            PipelineMode::Color => render_pass.set_pipeline(&self.render_pipeline_color),
                            PipelineMode::Lights => render_pass.set_pipeline(&self.render_pipeline_lights),
                            PipelineMode::Solid => render_pass.set_pipeline(&self.render_pipeline_solid),
                        }
                    }
                    BatchKind::Sphere => {
                        // Skip CPU-generated sphere instances - using GPU geometry instead
                        continue;
                    }
                    BatchKind::PointCloud => {
                        render_pass.set_pipeline(&self.render_pipeline_pointcloud);
                    }
                }
                // Set the camera bind group (after pipeline)
                render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
                
                match d.kind {
                    BatchKind::PointCloud => {
                        // Point clouds use instanced rendering with shared quad geometry
                        render_pass.set_vertex_buffer(0, self.pointcloud_quad_buffer.slice(..));
                        render_pass.set_vertex_buffer(1, self.pointcloud_instance_buffer.slice(..));
                        if self.pointcloud_num_instances > 0 {
                            // Only log occasionally to avoid spam
                            static mut FRAME_COUNT: u32 = 0;
                            // unsafe {
                            //     FRAME_COUNT += 1;
                            //     if FRAME_COUNT % 60 == 0 { // Log every 60 frames
                            //         #[cfg(target_arch = "wasm32")]
                            //         web_sys::console::log_1(&format!("🎨 Frame {}: RENDERING {} point cloud instances", FRAME_COUNT, self.pointcloud_num_instances).into());
                            //         #[cfg(not(target_arch = "wasm32"))]
                            //         println!("🎨 Frame {}: RENDERING {} point cloud instances", FRAME_COUNT, self.pointcloud_num_instances);
                            //     }
                            // }
                            
                            // Draw 6 vertices (quad) for each instance
                            render_pass.draw(0..6, 0..self.pointcloud_num_instances);
                        } else {
                            #[cfg(target_arch = "wasm32")]
                            web_sys::console::log_1(&"❌ Point cloud batch found but NO INSTANCES to render!".into());
                            #[cfg(not(target_arch = "wasm32"))]
                            println!("❌ Point cloud batch found but NO INSTANCES to render!");
                        }
                        continue; // Skip normal instance rendering for point clouds
                    }
                    _ => {
                        // Regular indexed drawing for meshes
                        if d.instance_count == 0 { continue; }
                        let start = d.instance_offset as u64 * stride;
                        let end = start + d.instance_count as u64 * stride;
                        render_pass.set_vertex_buffer(1, self.instance_buffer.slice(start..end));
                        
                        if d.index_count > 0 {
                            render_pass.draw_indexed(
                                d.first_index..(d.first_index + d.index_count),
                                d.base_vertex,
                                0..d.instance_count,
                            );
                        }
                    }
                }
            }

            // Render GPU geometry (pipes and spheres) with embedded vertex data
            if let (Some(pipeline), Some(bind_group)) = (&self.gpu_geometry_pipeline, &self.gpu_geometry_bind_group) {
                let num_pipes = self.gpu_pipes_data.len() as u32;
                let num_spheres = self.gpu_spheres_data.len() as u32;
                
                #[cfg(target_arch = "wasm32")]
                web_sys::console::log_1(&format!("🔧 GPU Geometry: {} pipes, {} spheres", num_pipes, num_spheres).into());
                #[cfg(not(target_arch = "wasm32"))]
                println!("🔧 GPU Geometry: {} pipes, {} spheres", num_pipes, num_spheres);
                
                if num_pipes > 0 {
                    #[cfg(target_arch = "wasm32")]
                    web_sys::console::log_1(&format!("🔵 Rendering {} pipes", num_pipes).into());
                    #[cfg(not(target_arch = "wasm32"))]
                    println!("🔵 Rendering {} pipes", num_pipes);
                    pipeline.render_pipes(&mut render_pass, bind_group, &self.camera_bind_group, num_pipes);
                }
                
                // Render spheres using embedded geometry in shader
                if num_spheres > 0 {
                    #[cfg(target_arch = "wasm32")]
                    web_sys::console::log_1(&format!("🟡 Rendering {} spheres", num_spheres).into());
                    #[cfg(not(target_arch = "wasm32"))]
                    println!("🟡 Rendering {} spheres", num_spheres);
                    pipeline.render_spheres(&mut render_pass, bind_group, &self.camera_bind_group, num_spheres);
                }
            }

        }
        self.queue.submit(iter::once(encoder.finish()));
        output.present();
    
        Ok(())
    }
    


    // Handle key events.
    // Escape - to exit the app
    // Space - cycle pipeline (Color -> Solid -> Lights)
    fn handle_key(&mut self, event_loop: &ActiveEventLoop, code: KeyCode, is_pressed: bool) {
        match (code, is_pressed) {
            (KeyCode::Escape, true) => event_loop.exit(),
            (KeyCode::Space, true) => {
                self.pipeline_mode = match self.pipeline_mode {
                    PipelineMode::Solid => PipelineMode::Lights,
                    PipelineMode::Color => PipelineMode::Solid,
                    PipelineMode::Lights => PipelineMode::Color,
                };
                #[cfg(target_arch = "wasm32")]
                {
                    web_sys::console::log_1(&format!("Pipeline mode: {:?}", self.pipeline_mode).into());
                }
                #[cfg(not(target_arch = "wasm32"))]
                {
                    log::info!("Pipeline mode: {:?}", self.pipeline_mode);
                }
            }
            // Toggle projection: Perspective <-> Orthographic (O)
            (KeyCode::KeyO, true) => {
                // Toggle flag
                let to_ortho = !self.camera.is_ortho;
                let fovy_rad = self.camera.fovy.to_radians();
                let half_tan = (0.5f32 * fovy_rad).tan().max(1e-6);
                if to_ortho {
                    // Match current perspective apparent scale at the target distance
                    self.camera.ortho_half_height = self.camera.distance * half_tan;
                    self.camera.is_ortho = true;
                } else {
                    // Switch to perspective; adjust distance to preserve vertical world span
                    let target_dist = self.camera.ortho_half_height / half_tan;
                    if target_dist.is_finite() && target_dist > 1e-4 {
                        self.camera.distance = target_dist;
                    }
                    self.camera.is_ortho = false;
                    // Update camera position to reflect distance change
                    self.camera.update_position();
                }
                // Immediately refresh camera uniform and push to GPU so the first frame after the toggle
                // uses the correct view-projection and ortho parameters.
                self.camera_uniform.update_view_proj(&self.camera);
                self.camera_uniform.set_eye_dir(&self.camera);
                self.camera_uniform.set_view_params(
                    self.config.width as f32,
                    self.config.height as f32,
                    self.camera.fovy,
                    self.camera.aspect,
                    self.pipe_px_radius,
                    self.camera.is_ortho,
                    self.camera.ortho_half_height,
                );
                self.queue.write_buffer(
                    &self.camera_buffer,
                    0,
                    bytemuck::cast_slice(&[self.camera_uniform]),
                );
                // Ensure a redraw is scheduled immediately
                self.window.request_redraw();
                #[cfg(target_arch = "wasm32")]
                web_sys::console::log_1(&format!(
                    "Projection mode: {} (ortho_half_height={:.3})",
                    if self.camera.is_ortho { "Orthographic" } else { "Perspective" },
                    self.camera.ortho_half_height
                ).into());
                #[cfg(not(target_arch = "wasm32"))]
                log::info!(
                    "Projection mode: {} (ortho_half_height={:.3})",
                    if self.camera.is_ortho { "Orthographic" } else { "Perspective" },
                    self.camera.ortho_half_height
                );
            }
            // Adjust GPU geometry scale (affects pipes and spheres in GPU geometry shader)
            (KeyCode::KeyN, true) => {
                self.gpu_geometry_scale = (self.gpu_geometry_scale * 0.9).max(0.1);
                self.update_gpu_geometry_scale();
                #[cfg(target_arch = "wasm32")]
                web_sys::console::log_1(&format!("GPU geometry scale = {:.2}", self.gpu_geometry_scale).into());
                #[cfg(not(target_arch = "wasm32"))]
                log::info!("GPU geometry scale = {:.2}", self.gpu_geometry_scale);
            }
            (KeyCode::KeyM, true) => {
                self.gpu_geometry_scale = (self.gpu_geometry_scale * 1.1111).min(10.0);
                self.update_gpu_geometry_scale();
                #[cfg(target_arch = "wasm32")]
                web_sys::console::log_1(&format!("GPU geometry scale = {:.2}", self.gpu_geometry_scale).into());
                #[cfg(not(target_arch = "wasm32"))]
                log::info!("GPU geometry scale = {:.2}", self.gpu_geometry_scale);
            }
            _ => {}
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////
// We need to tell winit how to use it, for this an App is created
////////////////////////////////////////////////////////////////////////////////////////////
pub struct App {
    #[cfg(target_arch = "wasm32")]
    proxy: Option<winit::event_loop::EventLoopProxy<State>>,
    state: Option<State>,
    vertices: Vec<Vertex>, // User geometry
    indices: Vec<u16>, // User geometry,
    batches: Vec<DrawBatch>,
}

impl App {
    pub fn new(
        #[cfg(target_arch = "wasm32")]
        event_loop: &EventLoop<State>,
        vertices: Vec<Vertex>, // User geometry
        indices: Vec<u16>, // User geometry
        batches: Vec<DrawBatch>,
    ) -> Self {      

        // Create the proxy for wasm
        #[cfg(target_arch = "wasm32")]
        let proxy = Some(event_loop.create_proxy());
        Self {
            state: None,
            vertices, // User geometry
            indices, // User geometry
            batches,
            #[cfg(target_arch = "wasm32")]
            proxy,
        }
    
    }


}

// This gives a variety of functions: key press, mouse movements, lifecycle events.
impl ApplicationHandler<State> for App {

    // Define attributes about the window including web attributes
    // We use those attributes to create the window
    // We create a future that creates our State struct
    // On native we use pollster to get await the future
    // On web we we run the future asynchronously which sned the results to the user_event function
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        #[allow(unused_mut)]
        let mut window_attributes = Window::default_attributes();

        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::JsCast;
            use winit::platform::web::WindowAttributesExtWebSys;
            
            const CANVAS_ID: &str = "canvas";

            let window = wgpu::web_sys::window().unwrap_throw();
            let document = window.document().unwrap_throw();
            let canvas = document.get_element_by_id(CANVAS_ID).unwrap_throw();
            let html_canvas_element = canvas.unchecked_into();
            window_attributes = window_attributes.with_canvas(Some(html_canvas_element));
        }

        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        #[cfg(not(target_arch = "wasm32"))]
        {
            // Native: load full geometry including point clouds so we can render glyphs
            // and ensure a PointCloud batch exists.
            let (vertices, indices, batches, pointcloud_vertices, pipe_transforms, sphere_transforms) = pollster::block_on(get_geometry_with_pointclouds_and_gpu());
            // Keep App copies in sync (may be used later)
            self.vertices = vertices;
            self.indices = indices;
            self.batches = batches;
            // Create State with point cloud vertices
            self.state = Some(
                pollster::block_on(State::new(
                    window,
                    &self.vertices,
                    &self.indices,
                    &self.batches,
                    &Vec::new(), // Empty pointcloud instances
                    pipe_transforms,
                    sphere_transforms
                ))
                .expect("Unable to create state")
            );
        }
        #[cfg(target_arch = "wasm32")]
        {
            // Run the future asynchronously and use the
            // proxy to send the results to the event loop
            if let Some(proxy) = self.proxy.take() {
                wasm_bindgen_futures::spawn_local(async move {
                    // Build geometry on WASM (embedded + grid/axis + local JSON if available)
                    let (vertices, indices, batches, pointcloud_vertices, pipe_transforms, sphere_transforms) = get_geometry_with_pointclouds_and_gpu().await;
                    assert!(proxy
                        .send_event(
                            State::new(window, &vertices, &indices, &batches, &pointcloud_vertices, pipe_transforms, sphere_transforms)
                                .await
                                .expect("Unable to create canvas!!!")
                        )
                        .is_ok());
                });
            }
        }
    }

    // This servers as a landing point four our State future. 
    // Resumed isnt aync so we need to offload the future and send the results somewhere
    #[allow(unused_mut)]
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, mut event: State) {
        // This is where proxy.send_event() ends up
        #[cfg(target_arch = "wasm32")]
        {
            event.window.request_redraw();
            event.resize(
                event.window.inner_size().width,
                event.window.inner_size().height,
            );
        }
        self.state = Some(event);
    }

    // Windows event
    // This is where we can process events such as keyboard inputs, and mouse movements
    // Other events such as when the window wants to draw or it is resized.
    // handle_key() function is used in window_event()
    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let state = match &mut self.state {
            Some(canvas) => canvas,
            None => return,
        };

        if state.input(&event) {
            return;
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(new_size) => {
                // Native: resize immediately from OS event. WASM: skip, we poll canvas size in update().
                #[cfg(not(target_arch = "wasm32"))]
                {
                    state.resize(new_size.width, new_size.height);
                }
                #[cfg(target_arch = "wasm32")]
                {
                    let _ = new_size; // suppress unused variable warning on wasm
                }
                // Ensure a new frame is scheduled after resize (important on Web)
                state.window.request_redraw();
            }
            // Redraw method to render the geometry
            WindowEvent::RedrawRequested => {
                state.update();
                match state.render() {
                    Ok(_) => {}
                    // Reconfigure the surface if it's lost or outdated
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        #[cfg(not(target_arch = "wasm32"))]
                        {
                            let size = state.window.inner_size();
                            state.resize(size.width, size.height);
                        }
                        #[cfg(target_arch = "wasm32")]
                        {
                            if let Some(win) = web_sys::window() {
                                if let Some(doc) = win.document() {
                                    if let Some(el) = doc.get_element_by_id("canvas") {
                                        if let Ok(canvas) = el.dyn_into::<web_sys::HtmlCanvasElement>() {
                                            let cw = canvas.width();
                                            let ch = canvas.height();
                                            if cw > 0 && ch > 0 {
                                                state.resize(cw, ch);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        // Schedule another frame so rendering continues after reconfig
                        state.window.request_redraw();
                    }
                    Err(e) => {
                        log::error!("Unable to render {}", e);
                        // Try to continue rendering next frame on transient errors
                        state.window.request_redraw();
                    }
                }
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state: key_state,
                        ..
                    },
                ..
            } => state.handle_key(event_loop, code, key_state.is_pressed()),
            _ => {}
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        if let Some(state) = &mut self.state {
             match event {
                 winit::event::DeviceEvent::MouseMotion { delta } => {
                    // Always forward mouse motion; controller decides based on active mode (orbit/pan)
                    state.camera_controller.process_mouse(delta.0, delta.1);
                 }
                 _ => {}
             }
        }
    }
}


// Now we actually need to run our code
// This function sets up the logger as well as creates the event loop and our app
// THen runs our app to completeion
pub fn run() -> anyhow::Result<()> {

    #[cfg(not(target_arch = "wasm32"))]
    {
        env_logger::init();
    }
    #[cfg(target_arch = "wasm32")]
    {
        console_log::init_with_level(log::Level::Info).unwrap_throw();
    }

    let event_loop = EventLoop::with_user_event().build()?;



    #[cfg(not(target_arch = "wasm32"))]
    let (vertices, indices, batches) = pollster::block_on(get_geometry());

    #[cfg(not(target_arch = "wasm32"))]
    let mut app = App::new(
        vertices,
        indices,
        batches,
    );

    #[cfg(target_arch = "wasm32")]
    let mut app = App::new(
        &event_loop,
        Vec::new(),
        Vec::new(),
        Vec::new(),
    );
    event_loop.run_app(&mut app)?;

    Ok(())
}

// Function to run code on the web.
// This will set up the panic hook so that when our code panics, we will see in browser console.
// Then it will run our code.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn run_web() -> Result<(), wasm_bindgen::JsValue> {
    console_error_panic_hook::set_once();
    run().unwrap_throw();

    Ok(())
}

// Enhanced geometry loading with point cloud and GPU geometry support
pub async fn get_geometry_with_pointclouds_and_gpu() -> (Vec<Vertex>, Vec<u16>, Vec<DrawBatch>, Vec<PointCloudInstance>, Vec<PipeTransform>, Vec<SphereTransform>) {
    let (vertices, indices, batches, pipe_transforms, sphere_transforms) = get_geometry_with_gpu_data().await;
    
    // Load point cloud data from AllGeometryData
    let mut pointcloud_instances: Vec<PointCloudInstance> = Vec::new();
    
    // Parse geometry data again to extract point clouds
    let local: Option<String> = {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let base = env!("CARGO_MANIFEST_DIR");
            let primary = format!("{}/all_geometry.json", base);
            std::fs::read_to_string(&primary)
                .or_else(|_| std::fs::read_to_string(LOCAL_GEOMETRY_PATH))
                .ok()
        }
        #[cfg(target_arch = "wasm32")]
        {
            None // Will be handled by fetch below
        }
    };

    #[cfg(target_arch = "wasm32")]
    let wasm_fetched = {
        use wasm_bindgen_futures::JsFuture;
        use web_sys::{Request, RequestInit, RequestMode, Response};
        
        let mut opts = RequestInit::new();
        opts.method("GET");
        opts.mode(RequestMode::Cors);
        
        let url = "./all_geometry.json";
        let request = Request::new_with_str_and_init(url, &opts).unwrap();
        
        match JsFuture::from(web_sys::window().unwrap().fetch_with_request(&request)).await {
            Ok(resp_value) => {
                let resp: Response = resp_value.dyn_into().unwrap();
                if resp.ok() {
                    match JsFuture::from(resp.text().unwrap()).await {
                        Ok(text) => Some(text.as_string().unwrap()),
                        Err(_) => None,
                    }
                } else {
                    None
                }
            }
            Err(_) => None,
        }
    };

    let json_str = {
        #[cfg(not(target_arch = "wasm32"))]
        { local.unwrap_or_else(|| include_str!("openmodel/all_geometry.json").to_string()) }
        #[cfg(target_arch = "wasm32")]
        { wasm_fetched.unwrap_or_else(|| include_str!("openmodel/all_geometry.json").to_string()) }
    };

    if let Ok(all_geom) = serde_json::from_str::<AllGeometryData>(&json_str) {
        // Extract point cloud data
        for pc in &all_geom.point_clouds {
            for _point in &pc.points {
                let instance = PointCloudInstance {
                    position: [0.0, 0.0, 0.0], // Default position
                    size: 1.0, // Default size
                    color: [1.0, 1.0, 1.0], // Default white color
                };
                pointcloud_instances.push(instance);
            }
        }
    }

    (vertices, indices, batches, pointcloud_instances, pipe_transforms, sphere_transforms)
}

// Enhanced geometry loading with point cloud support
pub async fn get_geometry_with_pointclouds() -> (Vec<Vertex>, Vec<u16>, Vec<DrawBatch>, Vec<PointCloudInstance>) {
    let (vertices, indices, batches, pointcloud_instances, _pipe_transforms, _sphere_transforms) = get_geometry_with_pointclouds_and_gpu().await;
    (vertices, indices, batches, pointcloud_instances)
}

// Geometry loading: minimal single function using local-or-embedded JSON
pub async fn get_geometry() -> (Vec<Vertex>, Vec<u16>, Vec<DrawBatch>) {
    let (vertices, indices, batches, _pointcloud_instances, _pipe_transforms, _sphere_transforms) = get_geometry_with_pointclouds_and_gpu().await;
    (vertices, indices, batches)
}

// Enhanced geometry loading that also returns GPU geometry data
pub async fn get_geometry_with_gpu_data() -> (Vec<Vertex>, Vec<u16>, Vec<DrawBatch>, Vec<PipeTransform>, Vec<SphereTransform>) {
    // Prefer local (native file or WASM fetch); fall back to embedded JSON.
    let local: Option<String> = {
        #[cfg(not(target_arch = "wasm32"))]
        {
            // Try top-level all_geometry.json first, then src/openmodel/all_geometry.json
            let base = env!("CARGO_MANIFEST_DIR");
            let primary = format!("{}/all_geometry.json", base);
            let openmodel_path = format!("{}/src/openmodel/all_geometry.json", base);
            
            // Try to copy from openmodel directory if it's newer
            if let (Ok(openmodel_meta), Ok(primary_meta)) = (
                std::fs::metadata(&openmodel_path),
                std::fs::metadata(&primary)
            ) {
                if openmodel_meta.modified().unwrap_or(std::time::UNIX_EPOCH) > 
                   primary_meta.modified().unwrap_or(std::time::UNIX_EPOCH) {
                    let _ = std::fs::copy(&openmodel_path, &primary);
                    log::info!("Auto-copied newer geometry from {}", openmodel_path);
                }
            } else if std::fs::metadata(&openmodel_path).is_ok() {
                // Primary doesn't exist but openmodel does, copy it
                let _ = std::fs::copy(&openmodel_path, &primary);
                log::info!("Auto-copied geometry from {}", openmodel_path);
            }
            
            std::fs::read_to_string(&primary)
                .or_else(|_| std::fs::read_to_string(LOCAL_GEOMETRY_PATH))
                .ok()
        }
        #[cfg(target_arch = "wasm32")]
        { fetch_text(LOCAL_GEOMETRY_HTTP_PATH).await }
    };

    let mut all_geom: AllGeometryData = match local {
        Some(ref s) => serde_json::from_str::<AllGeometryData>(s).unwrap_or_else(|_| {
            serde_json::from_str(include_str!("openmodel/all_geometry.json"))
                .expect("embedded geometry JSON must be valid")
        }),
        None => serde_json::from_str(include_str!("openmodel/all_geometry.json"))
            .expect("embedded geometry JSON must be valid"),
    };

    // Augment with procedural unit pipe/sphere meshes and corresponding mesh_instances.
    // augment_with_procedural() also records their mesh indices for convenience.
    all_geom.augment_with_procedural();

    // Build vertices, indices, and batches from parsed geometry

    let mut pipe_transforms: Vec<PipeTransform> = Vec::new();
    let mut sphere_transforms: Vec<SphereTransform> = Vec::new();
    let mut vertices: Vec<Vertex> = Vec::new();
    let mut indices: Vec<u16> = Vec::new();
    let mut batches: Vec<DrawBatch> = Vec::new();

    // Track which batch corresponds to which source mesh index (skip procedural unit pipe/sphere meshes)
    let mut mesh_to_batch: Vec<Option<usize>> = vec![None; all_geom.meshes.len()];
    // Cache procedural indices before borrowing meshes mutably
    let pipe_idx = all_geom.pipe_mesh_index;
    let sphere_idx = all_geom.sphere_mesh_index;
    for (i, m) in all_geom.meshes.iter_mut().enumerate() {
        if Some(i) == pipe_idx || Some(i) == sphere_idx { continue; }
        let first_index = indices.len() as u32;
        append_mesh_as_triangles(m, [0.8, 0.8, 0.8], &mut vertices, &mut indices);
        let index_count = (indices.len() as u32) - first_index;
        if index_count > 0 {
            // base_vertex must be 0 because indices are global
            batches.push(DrawBatch {
                first_index,
                index_count,
                base_vertex: 0,
                instances: vec![],
                kind: BatchKind::Surface,
            });
            mesh_to_batch[i] = Some(batches.len() - 1);
            log::info!(
                "Created surface batch for mesh {}: vertices={}, faces={}, index_count={}",
                i, m.number_of_vertices(), m.number_of_faces(), index_count
            );
        } else {
            log::warn!("Mesh {} produced no triangles (vertices={}, faces={})", i, m.number_of_vertices(), m.number_of_faces());
        }
    }

    // Pipe instancing from augmented mesh_instances (if any)
    if let Some(pipe_idx) = all_geom.pipe_mesh_index {
        if let Some(mi) = all_geom.mesh_instances.iter().find(|mi| mi.mesh_index == pipe_idx) {
            let mut pipe_instances: Vec<Instance> = mi
                .transforms
                .iter()
                .map(|xf| xform_to_instance(xf))
                .collect();
            if !pipe_instances.is_empty() {
                let mut unit_pipe = Mesh::create_unit_pipe_high_res();
                let first_index = indices.len() as u32;
                append_mesh_as_triangles(&mut unit_pipe, [0.3, 0.3, 0.3], &mut vertices, &mut indices);
                let index_count = (indices.len() as u32) - first_index;
                if index_count > 0 {
                    batches.push(DrawBatch {
                        first_index,
                        index_count,
                        base_vertex: 0,
                        instances: pipe_instances.drain(..).collect(),
                        kind: BatchKind::Pipe,
                    });
                    log::info!(
                        "Created pipe batch (augmented): instances={}, index_count={}",
                        batches.last().unwrap().instances.len(),
                        index_count
                    );
                }
            }
        }
    }

    // Sphere instancing from augmented mesh_instances (if any)
    if let Some(sphere_idx) = all_geom.sphere_mesh_index {
        if let Some(mi) = all_geom.mesh_instances.iter().find(|mi| mi.mesh_index == sphere_idx) {
            let sphere_instances: Vec<Instance> = mi
                .transforms
                .iter()
                .map(|xf| xform_to_instance(xf))
                .collect();
            if !sphere_instances.is_empty() {
                let mut unit_sphere = Mesh::create_unit_sphere_high_res();
                let first_index = indices.len() as u32;
                append_mesh_as_triangles(&mut unit_sphere, [0.3, 0.3, 0.3], &mut vertices, &mut indices);
                let index_count = (indices.len() as u32) - first_index;
                if index_count > 0 {
                    batches.push(DrawBatch {
                        first_index,
                        index_count,
                        base_vertex: 0,
                        instances: sphere_instances,
                        kind: BatchKind::Sphere,
                    });
                    log::info!(
                        "Created sphere batch (augmented): instances={}, index_count={}",
                        batches.last().unwrap().instances.len(),
                        index_count
                    );
                }
            }
        }
    }

    // Populate per-mesh instances into batches
    for mi in &all_geom.mesh_instances {
        if let Some(Some(bi)) = mesh_to_batch.get(mi.mesh_index) {
            let insts = mi
                .transforms
                .iter()
                .map(|xf| xform_to_instance(xf))
                .collect::<Vec<_>>();
            batches[*bi].instances = insts;
        }
    }
    
    // Extract GPU geometry data from pipe and sphere transforms
    let mut pipes = Vec::new();
    let mut spheres = Vec::new();
    
    // Extract pipe transforms directly - no conversion needed!
    if let Some(pipe_idx) = all_geom.pipe_mesh_index {
        if let Some(mi) = all_geom.mesh_instances.iter().find(|mi| mi.mesh_index == pipe_idx) {
            for xf in &mi.transforms {
                // Use transformation matrix directly - preserves all rotation, scale, and position data
                // Convert f64 matrix to f32 and reshape from [f64; 16] to [[f32; 4]; 4]
                let matrix_f32: [[f32; 4]; 4] = [
                    [xf.m[0] as f32, xf.m[1] as f32, xf.m[2] as f32, xf.m[3] as f32],
                    [xf.m[4] as f32, xf.m[5] as f32, xf.m[6] as f32, xf.m[7] as f32],
                    [xf.m[8] as f32, xf.m[9] as f32, xf.m[10] as f32, xf.m[11] as f32],
                    [xf.m[12] as f32, xf.m[13] as f32, xf.m[14] as f32, xf.m[15] as f32],
                ];
                pipes.push(PipeTransform {
                    transform: matrix_f32,
                });
            }
        }
    }
    
    // Extract sphere transforms directly - no conversion needed!
    if let Some(sphere_idx) = all_geom.sphere_mesh_index {
        if let Some(mi) = all_geom.mesh_instances.iter().find(|mi| mi.mesh_index == sphere_idx) {
            for xf in &mi.transforms {
                // Use transformation matrix directly - preserves all rotation, scale, and position data
                // Convert f64 matrix to f32 and reshape from [f64; 16] to [[f32; 4]; 4]
                let matrix_f32: [[f32; 4]; 4] = [
                    [xf.m[0] as f32, xf.m[1] as f32, xf.m[2] as f32, xf.m[3] as f32],
                    [xf.m[4] as f32, xf.m[5] as f32, xf.m[6] as f32, xf.m[7] as f32],
                    [xf.m[8] as f32, xf.m[9] as f32, xf.m[10] as f32, xf.m[11] as f32],
                    [xf.m[12] as f32, xf.m[13] as f32, xf.m[14] as f32, xf.m[15] as f32],
                ];
                spheres.push(SphereTransform {
                    transform: matrix_f32,
                });
            }
        }
    }

    (vertices, indices, batches, pipes, spheres)
}
