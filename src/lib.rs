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
use vertex::Vertex;
use camera::{Camera, CameraUniform, CameraController};
use timing::Instant;
use wgpu::util::DeviceExt;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

////////////////////////////////////////////////////////////////////////////////////////////
// This will store the state of our application related to the window
////////////////////////////////////////////////////////////////////////////////////////////
pub struct State {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    is_surface_configured: bool,
    // Shader pipelines
    render_pipeline_solid: wgpu::RenderPipeline, // First pipeline (one color)
    render_pipeline_color: wgpu::RenderPipeline, // Second pipeline (vertex colors)
    use_color_pipeline: bool,                    // Whether to use the second pipeline
    vertex_buffer: wgpu::Buffer, // We will store data of vertex.rs in this buffer
    index_buffer: wgpu::Buffer, // We will store data of vertex.rs in this buffer
    // Camera system - testing step by step
    camera: Camera,
    camera_uniform: CameraUniform,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    camera_controller: CameraController,
    last_render_time: Instant,
    mouse_pressed: bool,
    // default pointer to the window
    window: Arc<Window>,
}

impl State{
    // We don't need to be async right now, will implement later
    pub async fn new(window: Arc<Window>, vertices: &[Vertex], indices: &[u16]) -> anyhow::Result<Self> {

        let size = window.inner_size();

        // The instance is a handle to our GPU
        // BackendBit::PRIMARY => Vulkan + Metal + DX12 + Browser WebGPU
        // The instance is the first thing you create.
        // Its main purpose is to create Adapter(s) and Surface(s).
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            #[cfg(not(target_arch = "wasm32"))]
            backends: wgpu::Backends::PRIMARY,
            #[cfg(target_arch = "wasm32")]
            backends: wgpu::Backends::GL,
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

        // Use adapter to create device and queue
        // This current example doesn't use any features.
        // Full list of features: https://docs.rs/wgpu/latest/wgpu/struct.Features.html
        // Full list of limits: https://docs.rs/wgpu/latest/wgpu/struct.Limits.html
        // The mmemory_hints field provides the adapter with a preferred memory allocation strategy.
        // Memory hints options: https://wgpu.rs/doc/wgpu/enum.MemoryHints.html
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                // WebGL doesn't support all of wgpu's features, so if
                // we're building for the web we'll have to disable some.
                required_limits: if cfg!(target_arch = "wasm32") {
                    wgpu::Limits::downlevel_webgl2_defaults()
                } else {
                    wgpu::Limits::default()
                },
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
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        ////////////////////////////////////////////////////////////////////////////////////////////////////////////
        // SHADERS
        ////////////////////////////////////////////////////////////////////////////////////////////////////////////

        // Pipeline. We will have to load shaders, as the render pipeline require them.
        let shader_solid = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Solid Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader_solid.wgsl").into()),
        });

        let shader_color = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Color Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader_color.wgsl").into()),
        });

        // Pipeline layout - testing camera bind group step by step
        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("camera_bind_group_layout"),
            });

        let render_pipeline_layout =
        device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&camera_bind_group_layout],
            push_constant_ranges: &[],
        });

        // Pipeline for rendering
        let render_pipeline_solid = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Solid Pipeline"),
            layout: Some(&render_pipeline_layout),
            
            vertex: wgpu::VertexState {
                module: &shader_solid, // <-- Change the shader
                entry_point: Some("vs_main"), // 1. vertex entry point
                buffers: &[
                    Vertex::desc(), // The implementation of the vertex struct
                ], // 2. tells wgpu that type of vetices we want to pass to vertex shader
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },

            fragment: Some(wgpu::FragmentState { // 3. This is optional so we wrap to Some(), we need it for colors
                module: &shader_solid,  // <-- Change the shader
                entry_point: Some("fs_main"), // 1. fragment entry point
                targets: &[Some(wgpu::ColorTargetState { // 4. tells wgpu what color outputs it should set up
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),

            // The primitive field describes how to interpret our vertices when converting them into triangles.
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList, // Means that every three verties will correspond to one triangle.
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw, // CounterClockWise is facing forward, cw are culled
                cull_mode: Some(wgpu::Face::Back), // Cull back faces
                // Setting this to anything other than Fill requires Features::POLYGON_MODE_LINE
                // or Features::POLYGON_MODE_POINT
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },

            // We are not using a depth/stencil buffer currently
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1, // Determines how many samples the pipeline will use
                mask: !0, // Specifies which samples should be active, here we use all
                alpha_to_coverage_enabled: false, // related to multisampling which is not used here
            },
            // If the pipeline will be used with a multiview render pass, this
            // indicates how many array layers the attachments will have.
            multiview: None,
            // Useful for optimizing shader compilation on Android
            cache: None,
        });


         // Pipeline for rendering
         let render_pipeline_color = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Color Pipeline"),
            layout: Some(&render_pipeline_layout),
            
            vertex: wgpu::VertexState {
                module: &shader_color, // <-- Change the shader
                entry_point: Some("vs_main"), // 1. vertex entry point
                buffers: &[
                    Vertex::desc(),
                ], // 2. tells wgpu that type of vetices we want to pass to vertex shader
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },

            fragment: Some(wgpu::FragmentState { // 3. This is optional so we wrap to Some(), we need it for colors
                module: &shader_color, // <-- Change the shader
                entry_point: Some("fs_main"), // 1. fragment entry point
                targets: &[Some(wgpu::ColorTargetState { // 4. tells wgpu what color outputs it should set up
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),

            // The primitive field describes how to interpret our vertices when converting them into triangles.
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList, // Means that every three verties will correspond to one triangle.
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw, // CounterClockWise is facing forward, cw are culled
                cull_mode: Some(wgpu::Face::Back), // Cull back faces
                // Setting this to anything other than Fill requires Features::POLYGON_MODE_LINE
                // or Features::POLYGON_MODE_POINT
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },

            // We are not using a depth/stencil buffer currently
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1, // Determines how many samples the pipeline will use
                mask: !0, // Specifies which samples should be active, here we use all
                alpha_to_coverage_enabled: false, // related to multisampling which is not used here
            },
            // If the pipeline will be used with a multiview render pass, this
            // indicates how many array layers the attachments will have.
            multiview: None,
            // Useful for optimizing shader compilation on Android
            cache: None,
        });
        
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

        // Initialize camera system
        let camera = Camera::new(size.width as f32, size.height as f32);
        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_view_proj(&camera);

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        });

        let camera_controller = CameraController::new(4.0, 0.4);

        // Now that we configured our render surface.
        // We can create the struct State with its arguments.
        Ok(Self {
            surface,
            device,
            queue,
            config,
            is_surface_configured: false,
            // Pipeline for rendering solid color
            render_pipeline_solid,
            render_pipeline_color,
            use_color_pipeline: true,  
            vertex_buffer,
            index_buffer,
            // Camera system - testing step by step
            camera,
            camera_uniform,
            camera_buffer,
            camera_bind_group,
            camera_controller,
            last_render_time: Instant::now(),
            mouse_pressed: false,
            window,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32){
        // If we want to resize the window, we need to update the surface,
        // every time we resize the window.
        // This was the reason we store size and config to configure the surface.
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
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
        let now = Instant::now();
        let dt = now - self.last_render_time;
        self.last_render_time = now;
        self.camera_controller.update_camera(&mut self.camera, dt);
        self.camera_uniform.update_view_proj(&self.camera);
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
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.9,
                            g: 0.9,
                            b: 0.9,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
        
        
            // We set the the pipeline on the render_pass using the one we created for shader.
            if self.use_color_pipeline {
                render_pass.set_pipeline(&self.render_pipeline_solid);
            } else {
                render_pass.set_pipeline(&self.render_pipeline_color);
            }

            // Set the camera bind group (pipeline expects it even if shaders don't use it)
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);

            // Set the vertex buffer otherwise the app will crash.
            // First arguement is the buffer slot index
            // Second argument allows us to specifiy which portion of buffer to use, .. is entire buffer.
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));

            // You can only have one index buffer set at a time.
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16); // 1.

            // When using an index buffer, we need to use draw_indexed instead of draw.
            // First argument is the range of indices to draw.
            // Second argument is the base vertex.
            // Third argument is the instance count.
            render_pass.draw_indexed(0..(self.index_buffer.size() / std::mem::size_of::<u16>() as u64) as u32, 0, 0..1);
        
        }
        self.queue.submit(iter::once(encoder.finish()));
        output.present();
    
        Ok(())
    }
    


    // Handle key events.
    // Escape - to exit the app
    // Space - to change the shader in the render pipeline
    fn handle_key(&mut self, event_loop: &ActiveEventLoop, code: KeyCode, is_pressed: bool) {
        match (code, is_pressed) {
            (KeyCode::Escape, true) => event_loop.exit(),
            (KeyCode::Space, true) => self.use_color_pipeline = !self.use_color_pipeline,
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
    indices: Vec<u16>, // User geometry
}

impl App {
    pub fn new(
        #[cfg(target_arch = "wasm32")] event_loop: &EventLoop<State>,
        vertices: Vec<Vertex>, // User geometry
        indices: Vec<u16>, // User geometry
    ) -> Self {
        #[cfg(target_arch = "wasm32")]
        let proxy = Some(event_loop.create_proxy());
        Self {
            state: None,
            vertices, // User geometry
            indices, // User geometry
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
            self.state = Some(pollster::block_on(State::new(window, &self.vertices, &self.indices)).unwrap());
        }

        #[cfg(target_arch = "wasm32")]
        {
            // Run the future asynchronously and use the
            // proxy to send the results to the event loop
            if let Some(proxy) = self.proxy.take() {
                let vertices = self.vertices.clone(); // User geometry
                let indices = self.indices.clone(); // User geometry
                wasm_bindgen_futures::spawn_local(async move {
                    assert!(proxy
                        .send_event(
                            State::new(window, &vertices, &indices)
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
            WindowEvent::Resized(size) => state.resize(size.width, size.height),
            // Redraw method to render the geometry
            WindowEvent::RedrawRequested => {
                state.update();
                match state.render() {
                    Ok(_) => {}
                    // Reconfigure the surface if it's lost or outdated
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        let size = state.window.inner_size();
                        state.resize(size.width, size.height);
                    }
                    Err(e) => {
                        log::error!("Unable to render {}", e);
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
                    if state.mouse_pressed {
                        state.camera_controller.process_mouse(delta.0, delta.1);
                    }
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



    let (vertices, indices) = get_geometry();
    let mut app = App::new(
        #[cfg(target_arch = "wasm32")]
        &event_loop,
        vertices.to_vec(),  // Convert to Vec here
        indices.to_vec(),   // Convert to Vec here
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


// Geometry
// This has to be replaced with a proper geometry file reader.


pub fn get_geometry() -> (&'static [Vertex], &'static [u16]) {

    const VERTICES: &[Vertex] = &[
        // Colors converted from sRGB to linear space
        // [0.5, 0.5, 0.5] -> [0.21404114, 0.21404114, 0.21404114]
        Vertex { position: [-0.0868241, 0.49240386, 0.0], color: [0.21404114, 0.21404114, 0.21404114] }, // A
        // [0.0, 0.0, 0.5] -> [0.0, 0.0, 0.21404114]
        Vertex { position: [-0.49513406, 0.06958647, 0.0], color: [0.0, 0.0, 0.21404114] }, // B
        // [0.5, 0.0, 0.0] -> [0.21404114, 0.0, 0.0]
        Vertex { position: [-0.21918549, -0.44939706, 0.0], color: [0.21404114, 0.0, 0.0] }, // C
        // [0.0, 0.0, 0.0] -> [0.0, 0.0, 0.0]
        Vertex { position: [0.35966998, -0.3473291, 0.0], color: [0.0, 0.0, 0.0] }, // D
        // [1.0, 1.0, 1.0] -> [1.0, 1.0, 1.0]
        Vertex { position: [0.44147372, 0.2347359, 0.0], color: [1.0, 1.0, 1.0] }, // E
    ];
    
    const INDICES: &[u16] = &[0, 1, 4, 1, 2, 4, 2, 3, 4, /* padding */ 0];

    
    (VERTICES, INDICES)
}
