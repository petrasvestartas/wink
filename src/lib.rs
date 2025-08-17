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
pub mod shader_color_pipeline;
pub mod shader_solid_pipeline;
pub mod shader_pipe_pipeline;
pub mod shader_sphere_pipeline;
pub mod shader_lights_pipeline;
use vertex::Vertex;
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
use openmodel::geometry::{Mesh};

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
const MSAA_SAMPLE_COUNT: u32 = 4;
#[cfg(not(target_arch = "wasm32"))]
const MSAA_SAMPLE_COUNT: u32 = 4;

#[derive(Copy, Clone, Debug, PartialEq)]
enum PipelineMode { Solid, Color, Lights }

#[cfg(target_arch = "wasm32")]
use std::cell::{Cell, RefCell};

#[cfg(target_arch = "wasm32")]
thread_local! {
    static PENDING_GEOMETRY: RefCell<Option<(Vec<Vertex>, Vec<u16>, Vec<DrawBatch>)>> = RefCell::new(None);
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
// Helper: push mesh faces as triangles (fan) with per-vertex or default color
fn append_mesh_as_triangles(
    mesh: &Mesh,
    default_color: [f32; 3],
    vertices: &mut Vec<Vertex>,
    indices: &mut Vec<u16>,
) {
    for (face_key, face_vertices) in mesh.get_face_data() {
        if face_vertices.len() < 3 { continue; }
        for i in 1..(face_vertices.len() - 1) {
            let tri = [face_vertices[0], face_vertices[i], face_vertices[i + 1]];
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
                    let n = mesh.vertex_normal_resolved(vk, Some(*face_key));
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

// // Helper: 10x10 grid (11 lines per direction) + 1-unit Z axis as pipes
// fn make_grid_and_axis_meshes() -> Vec<(Mesh, [f32; 3])> {
//     let mut out = Vec::new();
//     let size: i32 = 5; // -5..=5 => 11 lines => 10x10 cells
//     let radius: f64 = 0.02;
//     let grid_color: [f32; 3] = [0.3, 0.3, 0.3];
//     let axis_color: [f32; 3] = [0.0, 0.0, 1.0];

//     for i in -size..=size {
//         let y = i as f64;
//         out.push((Mesh::create_pipe(Point::new(-(size as f64), y, 0.0), Point::new(size as f64, y, 0.0), radius), grid_color));
//     }
//     for i in -size..=size {
//         let x = i as f64;
//         out.push((Mesh::create_pipe(Point::new(x, -(size as f64), 0.0), Point::new(x, size as f64, 0.0), radius), grid_color));
//     }
//     out.push((Mesh::create_pipe(Point::new(0.0, 0.0, 0.0), Point::new(0.0, 0.0, 1.0), 0.03), axis_color));
//     out
// }

pub struct State{
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    is_surface_configured: bool,
    // Shader pipelines
    render_pipeline_solid: wgpu::RenderPipeline, // First pipeline (one color)
    render_pipeline_color: wgpu::RenderPipeline, // Second pipeline (vertex colors)
    render_pipeline_pipe: wgpu::RenderPipeline,  // Third pipeline (pipe-specific)
    render_pipeline_sphere: wgpu::RenderPipeline, // Sphere pipeline (vertex caps)
    render_pipeline_lights: wgpu::RenderPipeline, // Lights pipeline (lit surfaces)
    pipeline_mode: PipelineMode,                 // Active pipeline selection
    // Pipe rendering controls
    pipe_px_radius: f32,
    vertex_buffer: wgpu::Buffer, // We will store data of vertex.rs in this buffer
    index_buffer: wgpu::Buffer, // We will store data of vertex.rs in this buffer
    num_indices: u32,
    // Camera system - testing step by step
    camera: Camera,
    camera_uniform: CameraUniform,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
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
}

impl State{
    // We don't need to be async right now, will implement later
    pub async fn new(window: Arc<Window>, vertices: &[Vertex], indices: &[u16], batches_in: &[DrawBatch]) -> anyhow::Result<Self> {

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

        let render_pipeline_pipe = crate::shader_pipe_pipeline::create(
            &device, &config, &camera_bind_group_layout, DEPTH_FORMAT, msaa_sample_count,
        );

        let render_pipeline_sphere = crate::shader_sphere_pipeline::create(
            &device, &config, &camera_bind_group_layout, DEPTH_FORMAT, msaa_sample_count,
        );

        let render_pipeline_lights = crate::shader_lights_pipeline::create(
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
            render_pipeline_pipe,
            render_pipeline_sphere,
            render_pipeline_lights,
            pipeline_mode: PipelineMode::Color,  
            pipe_px_radius: default_pipe_px_radius,
            vertex_buffer,
            index_buffer,
            num_indices,
            // Camera system - testing step by step
            camera,
            camera_uniform,
            camera_buffer,
            camera_bind_group,
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
        };
        // Configure surface immediately to avoid first-frame issues
        state.resize(size.width, size.height);
        Ok(state)
    }

    // Replace entire scene: geometry + instance batches
    fn replace_scene(&mut self, vertices: &[Vertex], indices: &[u16], batches_in: &[DrawBatch]) {
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
        self.instances = flat_instances;
        self.instance_buffer = new_instance_buffer;
        self.batches = batch_draws;

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
                self.replace_scene(&vertices, &indices, &batches);
            }
            return;
        }

        // WASM: throttled async polling and apply results prepared by the task
        #[cfg(target_arch = "wasm32")]
        {
            let now = Instant::now();
            if (now - self.last_poll_time).as_millis() < (GEOMETRY_POLL_INTERVAL_MS as u128) {
                return;
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
                        let (vertices, indices, batches) = get_geometry().await;
                        PENDING_GEOMETRY.with(|p| *p.borrow_mut() = Some((vertices, indices, batches)));
                        web_sys::console::log_1(&"Geometry changed; source: local".into());
                    }
                    LOCAL_FETCHING.with(|f| f.set(false));
                });
            }

            // Apply any pending geometry prepared by the async task
            let pending = PENDING_GEOMETRY.with(|p| p.borrow_mut().take());
            if let Some((vertices, indices, batches)) = pending {
                self.replace_scene(&vertices, &indices, &batches);
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
            } => self.camera_controller.process_keyboard(*key, *state),
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

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        self.window.request_redraw(); // We ask the window to draw another frame

        // We cannot render unless the surface is configured
        if !self.is_surface_configured {
            return Ok(());
        }

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
            for d in &self.batches {
                // Integrated rendering:
                // - Pipe batches render in BOTH Solid and Color modes using the pipe pipeline.
                // - Surface batches render with Solid or Color depending on current mode.
                match d.kind {
                    BatchKind::Pipe => {
                        render_pass.set_pipeline(&self.render_pipeline_pipe);
                    }
                    BatchKind::Surface => {
                        match self.pipeline_mode {
                            PipelineMode::Color => render_pass.set_pipeline(&self.render_pipeline_color),
                            PipelineMode::Lights => render_pass.set_pipeline(&self.render_pipeline_lights),
                            PipelineMode::Solid => render_pass.set_pipeline(&self.render_pipeline_solid),
                        }
                    }
                    BatchKind::Sphere => {
                        render_pass.set_pipeline(&self.render_pipeline_sphere);
                    }
                }
                // Set the camera bind group (after pipeline)
                render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
                if d.index_count == 0 || d.instance_count == 0 { continue; }
                let start = d.instance_offset as u64 * stride;
                let end = start + d.instance_count as u64 * stride;
                render_pass.set_vertex_buffer(1, self.instance_buffer.slice(start..end));
                render_pass.draw_indexed(
                    d.first_index..(d.first_index + d.index_count),
                    d.base_vertex,
                    0..d.instance_count,
                );
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
            // Adjust pipe pixel radius (affects only Pipe shader)
            (KeyCode::BracketLeft, true) => {
                self.pipe_px_radius = (self.pipe_px_radius * 0.9).max(0.25);
                #[cfg(target_arch = "wasm32")]
                web_sys::console::log_1(&format!("pipe_px_radius = {:.2}", self.pipe_px_radius).into());
                #[cfg(not(target_arch = "wasm32"))]
                log::info!("pipe_px_radius = {:.2}", self.pipe_px_radius);
            }
            (KeyCode::BracketRight, true) => {
                self.pipe_px_radius = (self.pipe_px_radius * 1.1111).min(64.0);
                #[cfg(target_arch = "wasm32")]
                web_sys::console::log_1(&format!("pipe_px_radius = {:.2}", self.pipe_px_radius).into());
                #[cfg(not(target_arch = "wasm32"))]
                log::info!("pipe_px_radius = {:.2}", self.pipe_px_radius);
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
            // If we are not on web we can use pollster to
            // await the 
            self.state = Some(pollster::block_on(State::new(window, &self.vertices, &self.indices, &self.batches)).unwrap());
        }

        #[cfg(target_arch = "wasm32")]
        {
            // Run the future asynchronously and use the
            // proxy to send the results to the event loop
            if let Some(proxy) = self.proxy.take() {
                wasm_bindgen_futures::spawn_local(async move {
                    // Build geometry on WASM (embedded + grid/axis + local JSON if available)
                    let (vertices, indices, batches) = get_geometry().await;
                    assert!(proxy
                        .send_event(
                            State::new(window, &vertices, &indices, &batches)
                                .await
                                .expect("Unable to create canvas!!!")
                        )
                        .is_ok())
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

// Geometry loading: minimal single function using local-or-embedded JSON
pub async fn get_geometry() -> (Vec<Vertex>, Vec<u16>, Vec<DrawBatch>) {
    // Prefer local (native file or WASM fetch); fall back to embedded JSON.
    let local: Option<String> = {
        #[cfg(not(target_arch = "wasm32"))]
        { std::fs::read_to_string(LOCAL_GEOMETRY_PATH).ok() }
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

    let mut vertices: Vec<Vertex> = Vec::new();
    let mut indices: Vec<u16> = Vec::new();
    let mut batches: Vec<DrawBatch> = Vec::new();

    // Track which batch corresponds to which source mesh index (skip procedural unit pipe/sphere meshes)
    let mut mesh_to_batch: Vec<Option<usize>> = vec![None; all_geom.meshes.len()];
    for (i, m) in all_geom.meshes.iter().enumerate() {
        if Some(i) == all_geom.pipe_mesh_index || Some(i) == all_geom.sphere_mesh_index { continue; }
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
        }
    }

    // // Add procedural grid and axis once
    // for (m, color) in make_grid_and_axis_meshes() {
    //     let first_index = indices.len() as u32;
    //     append_mesh_as_triangles(&m, color, &mut vertices, &mut indices);
    //     let index_count = (indices.len() as u32) - first_index;
    //     if index_count > 0 {
    //         batches.push(DrawBatch {
    //             first_index,
    //             index_count,
    //             base_vertex: 0,
    //             instances: vec![],
    //             kind: BatchKind::Surface,
    //         });
    //     }
    // }

    // Pipe instancing from augmented mesh_instances (if any)
    if let Some(pipe_idx) = all_geom.pipe_mesh_index {
        if let Some(mi) = all_geom.mesh_instances.iter().find(|mi| mi.mesh_index == pipe_idx) {
            let mut pipe_instances: Vec<Instance> = mi
                .transforms
                .iter()
                .map(|xf| xform_to_instance(xf))
                .collect();
            if !pipe_instances.is_empty() {
                let unit_pipe = Mesh::create_unit_pipe_high_res();
                let first_index = indices.len() as u32;
                append_mesh_as_triangles(&unit_pipe, [0.3, 0.3, 0.3], &mut vertices, &mut indices);
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
                let unit_sphere = Mesh::create_unit_sphere_high_res();
                let first_index = indices.len() as u32;
                append_mesh_as_triangles(&unit_sphere, [0.3, 0.3, 0.3], &mut vertices, &mut indices);
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

    (vertices, indices, batches)
}
