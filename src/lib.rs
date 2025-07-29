use std::{iter, sync::Arc}; // Arc is a thread-safe reference-counted pointer
use anyhow::Result;
use winit::{
    application::ApplicationHandler, 
    event::{WindowEvent, KeyEvent, MouseButton}, //* - import everythingi is skipped due to warnings
    event_loop::{ActiveEventLoop, EventLoop}, 
    keyboard::{KeyCode, PhysicalKey}, 
    window::Window
};

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
    // default pointer to the window
    window: Arc<Window>,
}

impl State{
    // We don't need to be async right now, will implement later
    pub async fn new(window: Arc<Window>) -> anyhow::Result<Self> {

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
     


        // Now that we configured our render surface.
        // We can create the struct State with its arguments.
        Ok(Self {
            surface,
            device,
            queue,
            config,
            is_surface_configured: false,
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

    fn update(&mut self) {
        // TODO: Update the state of the application
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
        {
        let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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
        }

        // submit will accept anything that implements IntoIter
        self.queue.submit(iter::once(encoder.finish()));
        output.present();
    
        Ok(())
    }
    


    // Handle key events.
    // Escape - to exit the app
    fn handle_key(&self, event_loop: &ActiveEventLoop, code: KeyCode, is_pressed: bool) {
        match (code, is_pressed) {
            (KeyCode::Escape, true) => event_loop.exit(),
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
}

impl App {
    pub fn new(#[cfg(target_arch = "wasm32")] event_loop: &EventLoop<State>) -> Self {
        #[cfg(target_arch = "wasm32")]
        let proxy = Some(event_loop.create_proxy());
        Self {
            state: None,
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
            self.state = Some(pollster::block_on(State::new(window)).unwrap());
        }

        #[cfg(target_arch = "wasm32")]
        {
            // Run the future asynchronously and use the
            // proxy to send the results to the event loop
            if let Some(proxy) = self.proxy.take() {
                wasm_bindgen_futures::spawn_local(async move {
                    assert!(proxy
                        .send_event(
                            State::new(window)
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

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => state.resize(size.width, size.height),
            // Redraw method to render the geometry
            WindowEvent::RedrawRequested => {
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
            WindowEvent::MouseInput { state, button, .. } => match (button, state.is_pressed()) {
                (MouseButton::Left, true) => {}
                (MouseButton::Left, false) => {}
                _ => {}
            },
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
    let mut app = App::new(
        #[cfg(target_arch = "wasm32")]
        &event_loop,
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

    
