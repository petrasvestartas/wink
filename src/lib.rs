use std::{iter, sync::Arc}; // Arc is a thread-safe reference-counted pointer
use anyhow::Result;

#[cfg(target_arch = "wasm32")]
use {
    wasm_bindgen_futures::spawn_local,
    wasm_bindgen::{JsCast, UnwrapThrowExt},
    wasm_bindgen::prelude::*,
};
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
pub mod error_handling;
pub mod buffer_factory;
pub mod geometry_loader;
pub mod scene_bounds;
use shader_geometry_pipeline::{GpuGeometryPipeline, PipeTransform, SphereTransform};
pub mod shader_color_pipeline;
pub mod shader_solid_pipeline;
pub mod shader_lights_pipeline;
pub mod shader_pointcloud_pipeline;
pub mod shader_geometry_pipeline;
use vertex::Vertex;
use shader_pointcloud_pipeline::{PointCloudInstance, QuadVertex};
use camera::{Camera, CameraUniform, CameraController};
use timing::Instant;
use instance::{Instance, InstanceRaw, DrawBatch, BatchDraw, BatchKind};
use buffer_factory::BufferFactory;
use geometry_loader::GeometryLoader;
use scene_bounds::SceneBounds;

// Platform-specific constants
#[cfg(target_arch = "wasm32")]
const LOCAL_GEOMETRY_HTTP_PATH: &str = "/geometry/all_geometry.json"; // served by docs dev server
#[cfg(target_arch = "wasm32")]
const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24Plus;
#[cfg(target_arch = "wasm32")]
const _MSAA_SAMPLE_COUNT: u32 = 4;

#[cfg(not(target_arch = "wasm32"))]
const LOCAL_GEOMETRY_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/src/openmodel/all_geometry.json");
#[cfg(not(target_arch = "wasm32"))]
const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

// Polling interval for change detection (ms)
const GEOMETRY_POLL_INTERVAL_MS: u64 = 100;

#[derive(Copy, Clone, Debug, PartialEq)]
enum PipelineMode { Color, Solid, Lights, PointCloud }




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
    geom_rx: std::sync::mpsc::Receiver<(Vec<Vertex>, Vec<u16>, Vec<DrawBatch>, Vec<PointCloudInstance>)>,
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
        let (tx_geom, rx_geom) = std::sync::mpsc::channel::<(Vec<Vertex>, Vec<u16>, Vec<DrawBatch>, Vec<PointCloudInstance>)>();

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
                        let (vertices, indices, batches, pointcloud_instances, _pipe_transforms, _sphere_transforms) = pollster::block_on(GeometryLoader::get_geometry());
                        let _ = tx_geom.send((vertices, indices, batches, pointcloud_instances));
                    }
                    std::thread::sleep(StdDuration::from_millis(GEOMETRY_POLL_INTERVAL_MS));
                }
            });
        }

        // Create GPU buffers from provided geometry
        let vertex_buffer = BufferFactory::create_vertex_buffer(&device, vertices);

        let index_buffer = BufferFactory::create_index_buffer(&device, indices);
        
        let num_indices = indices.len() as u32;
        
        // Create shared quad geometry buffer for point cloud rendering
        let pointcloud_quad_buffer = BufferFactory::create_quad_buffer(&device, QuadVertex::VERTICES);
        
        // Create point cloud instance buffer - ensure we have at least one dummy instance to avoid GPU crashes
        let buffer_data = if pointcloud_instances.is_empty() {
            vec![PointCloudInstance {
                position: [0.0, 0.0, 0.0],
                size: 0.0,
                color: [0.0, 0.0, 0.0],
            }]
        } else {
            pointcloud_instances.to_vec()
        };
        
        let pointcloud_instance_buffer = BufferFactory::create_pointcloud_buffer(&device, &buffer_data);
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
        let instance_buffer = BufferFactory::create_instance_buffer(&device, &instance_data);

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

        let camera_buffer = BufferFactory::create_uniform_buffer(&device, &[camera_uniform]);

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

    // Helper function to update camera uniform and write to buffer
    fn update_camera_uniform(&mut self) {
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
        self.write_camera_buffer();
    }

    // Helper function to write camera uniform to GPU buffer
    fn write_camera_buffer(&self) {
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[self.camera_uniform]),
        );
    }


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
    fn apply_scale_to_pipes(&self, pipes: Vec<PipeTransform>) -> Vec<PipeTransform> {
        // No scaling applied here - GPU shader handles all scaling based on pipe_px_radius
        // This prevents double scaling issues
        pipes
    }
    
    // Apply uniform scaling to sphere transformation matrices
    fn apply_scale_to_spheres(&self, spheres: Vec<SphereTransform>) -> Vec<SphereTransform> {
        // No scaling applied here - GPU shader handles all scaling based on pipe_px_radius
        // This prevents double scaling issues
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
        
        // Always update when new data is provided - don't assume identical data based on count alone
        // This ensures geometry changes are reflected even when instance count stays the same
        
        // Create new instance buffer only when needed
        let pointcloud_instance_buffer = BufferFactory::create_pointcloud_buffer(&self.device, pointcloud_instances);
        
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


    // Replace entire scene including point clouds
    fn replace_scene_with_pointclouds(&mut self, vertices: &[Vertex], indices: &[u16], batches_in: &[DrawBatch], pointcloud_instances: &[PointCloudInstance]) {
        #[cfg(target_arch = "wasm32")]
        web_sys::console::log_1(&format!("🔄 REPLACING SCENE: {} vertices, {} indices, {} batches, {} point cloud instances", vertices.len(), indices.len(), batches_in.len(), pointcloud_instances.len()).into());
        // Add a new line here to log the point cloud instances
        #[cfg(not(target_arch = "wasm32"))]
        println!("🔄 REPLACING SCENE: {} vertices, {} indices, {} batches, {} point cloud instances", vertices.len(), indices.len(), batches_in.len(), pointcloud_instances.len());
        // Replace vertex/index buffers
        let new_vertex_buffer = BufferFactory::create_vertex_buffer(&self.device, vertices);
        let new_index_buffer = BufferFactory::create_index_buffer(&self.device, indices);
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
        let new_instance_buffer = BufferFactory::create_instance_buffer(&self.device, &instance_data);
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

        // CRITICAL: Recreate GPU geometry bind group after buffer updates
        if let Some(pipeline) = &self.gpu_geometry_pipeline {
            let bind_group = pipeline.update_data(
                &self.device,
                self.apply_scale_to_pipes(self.gpu_pipes_data.clone()),
                self.apply_scale_to_spheres(self.gpu_spheres_data.clone()),
            );
            self.gpu_geometry_bind_group = Some(bind_group);
            
            #[cfg(target_arch = "wasm32")]
            web_sys::console::log_1(&"🔄 GPU bind group recreated after geometry update".into());
            #[cfg(not(target_arch = "wasm32"))]
            println!("🔄 GPU bind group recreated after geometry update");
        }

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
            while let Ok((vertices, indices, batches, pointcloud_instances)) = self.geom_rx.try_recv() {
                println!("🔄 Native geometry update - loading {} new point cloud instances", pointcloud_instances.len());
                self.replace_scene_with_pointclouds(&vertices, &indices, &batches, &pointcloud_instances);
            }
            return;
        }

        // WASM: throttled async polling and apply results prepared by the task
        #[cfg(target_arch = "wasm32")]
        {
            let now = Instant::now();
            let elapsed = now.duration_since(self.last_poll_time);
            let web_poll_interval = GEOMETRY_POLL_INTERVAL_MS; // Real-time polling on web
            if elapsed.as_millis() < web_poll_interval as u128 {
                return; // Skip this frame
            }
            self.last_poll_time = now;
            web_sys::console::log_1(&"🔍 Starting geometry poll cycle".into());
            // If a fetch is already running, just try to apply pending result
            let already_fetching = GeometryLoader::is_fetching();
            if !already_fetching {
                GeometryLoader::set_fetching(true);
                // Spawn async poll for local JSON; only apply if content hash changed
                spawn_local(async move {
                    // Fetch local-served JSON
                    web_sys::console::log_1(&format!("🔍 Fetching geometry from: {}", LOCAL_GEOMETRY_HTTP_PATH).into());
                    let local_text = GeometryLoader::fetch_text(LOCAL_GEOMETRY_HTTP_PATH).await;
                    let local_changed = if let Some(ref t) = local_text {
                        let new_hash = GeometryLoader::fnv1a64(t.as_bytes());
                        let old_hash = GeometryLoader::get_local_hash();
                        web_sys::console::log_1(&format!("🔍 Hash comparison - Old: {:?}, New: {}, Size: {} bytes", old_hash, new_hash, t.len()).into());
                        if old_hash.map_or(true, |old| old != new_hash) {
                            GeometryLoader::set_local_hash(new_hash);
                            web_sys::console::log_1(&"✅ Hash changed - will reload geometry".into());
                            true
                        } else {
                            web_sys::console::log_1(&"⏭️ Hash unchanged - skipping reload".into());
                            false
                        }
                    } else { 
                        web_sys::console::log_1(&"❌ Failed to fetch geometry file".into());
                        false 
                    };
                    
                    if local_changed {
                        // Rebuild full scene using the same logic as initial load
                        let (vertices, indices, batches, pointcloud_instances, pipe_transforms, sphere_transforms) = GeometryLoader::get_geometry().await;
                        #[cfg(target_arch = "wasm32")]
                        web_sys::console::log_1(&format!("🔄 FILE CHANGED - Reloading geometry: {} vertices, {} indices, {} batches, {} point cloud instances, {} pipes, {} spheres", vertices.len(), indices.len(), batches.len(), pointcloud_instances.len(), pipe_transforms.len(), sphere_transforms.len()).into());
                        GeometryLoader::set_pending_geometry_with_gpu_data((vertices, indices, batches, pointcloud_instances, pipe_transforms, sphere_transforms));
                        web_sys::console::log_1(&"Geometry changed; source: local".into());
                    }
                    GeometryLoader::set_fetching(false);
                });
            }

            // Apply any pending geometry prepared by the async task
            let pending = GeometryLoader::take_pending_geometry();
            if let Some((vertices, indices, batches, pointcloud_instances, pipe_transforms, sphere_transforms)) = pending {
                #[cfg(target_arch = "wasm32")]
                web_sys::console::log_1(&format!("🔄 APPLYING PENDING GEOMETRY: {} vertices, {} indices, {} batches, {} point cloud instances, {} pipes, {} spheres", vertices.len(), indices.len(), batches.len(), pointcloud_instances.len(), pipe_transforms.len(), sphere_transforms.len()).into());
                #[cfg(not(target_arch = "wasm32"))]
                println!("🔄 APPLYING PENDING GEOMETRY: {} vertices, {} indices, {} batches, {} point cloud instances, {} pipes, {} spheres", vertices.len(), indices.len(), batches.len(), pointcloud_instances.len(), pipe_transforms.len(), sphere_transforms.len());
                
                // Update GPU geometry data
                self.gpu_pipes_data = pipe_transforms;
                self.gpu_spheres_data = sphere_transforms;
                
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
        
        // Debug: Log that update is being called
        #[cfg(target_arch = "wasm32")]
        {
            static mut UPDATE_COUNTER: u32 = 0;
            unsafe {
                UPDATE_COUNTER += 1;
                if UPDATE_COUNTER % 300 == 0 { // Log every 5 seconds at 60fps
                    web_sys::console::log_1(&format!("🔄 Update loop running ({})", UPDATE_COUNTER).into());
                }
            }
        }
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
        // Update extended camera/pipe parameters each frame
        self.update_camera_uniform();
        self.write_camera_buffer();
    }

    /// Apply an external OpenModel world_from_camera transform to the camera
    /// and refresh the uniform buffer immediately.
    pub fn apply_camera_om_xform(&mut self, xf: openmodel::primitives::Xform) {
        self.camera.set_om_world_from_camera(xf);
        // Rebuild view-projection and extended fields from the updated camera
        self.update_camera_uniform();
        // Write updated uniform to GPU so the effect is visible this frame
        self.write_camera_buffer();
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
                            PipelineMode::PointCloud => render_pass.set_pipeline(&self.render_pipeline_pointcloud),
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
                            unsafe {
                                FRAME_COUNT += 1;
                                if FRAME_COUNT % 60 == 0 { // Log every 60 frames
                                }
                            }
                            
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
                
                
                if num_pipes > 0 {
                    pipeline.render_pipes(&mut render_pass, bind_group, &self.camera_bind_group, num_pipes);
                }
                
                // Render spheres using embedded geometry in shader
                if num_spheres > 0 {
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
                    PipelineMode::Lights => PipelineMode::PointCloud,
                    PipelineMode::PointCloud => PipelineMode::Color,
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
                self.update_camera_uniform();
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
            use wasm_bindgen::{JsCast, UnwrapThrowExt};
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
            let (vertices, indices, batches, pointcloud_vertices, pipe_transforms, sphere_transforms) = pollster::block_on(GeometryLoader::get_geometry());
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
                    &pointcloud_vertices, // Use loaded pointcloud instances
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
                    let (vertices, indices, batches, pointcloud_vertices, pipe_transforms, sphere_transforms) = GeometryLoader::get_geometry().await;
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
    let (vertices, indices, batches, _pointcloud_instances, _pipe_transforms, _sphere_transforms) = pollster::block_on(GeometryLoader::get_geometry());

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
// WASM entry point
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn run_web() -> Result<(), wasm_bindgen::JsValue> {
    console_error_panic_hook::set_once();
    run().unwrap_throw();
    Ok(())
}


