use anyhow::anyhow;
use futures::FutureExt;
use std::sync::Arc;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy},
    window::{Window, WindowId},
};

#[cfg(target_arch = "wasm32")]
mod wasm_commands {
    use std::cell::RefCell;
    use std::sync::Arc;
    use winit::window::Window;

    thread_local! {
        /// Reference to the window for requesting redraws
        pub static WINDOW: RefCell<Option<Arc<Window>>> = RefCell::new(None);
    }

    pub fn set_window(window: Arc<Window>) {
        WINDOW.with(|w| *w.borrow_mut() = Some(window));
    }
}

mod events;
mod gpu_data;
mod index_buffer;
mod interaction;
mod keyboard;
mod mip;
mod mouse;
mod render;
mod scene;
mod vertex_buffer;
mod view;
#[cfg(target_arch = "wasm32")]
mod wasm_viewer;

use crate::{
    events::{ErrorEvent, Event, SharedFuture, SystemEvent, UserEvent},
    interaction::Interaction,
    render::Renderer,
    scene::Scene,
};

#[derive(thiserror::Error, Debug)]
enum InitializationError {
    #[error("Failed to get GPU adapter ({0})")]
    AdapterError(#[from] wgpu::RequestAdapterError),
    #[error("Failed to create GPU device ({0})")]
    DeviceError(#[from] wgpu::RequestDeviceError),
    #[error("Failed to create surface ({0})")]
    SurfaceError(#[from] wgpu::CreateSurfaceError),
    #[error("Failed to create window ({0})")]
    CreateWindowError(#[from] winit::error::OsError),
}

struct State {
    window: Arc<Window>,
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    texture_bind_group_layout: wgpu::BindGroupLayout,
    renderer: Renderer,
    interaction: Interaction,
    scene: Option<Scene>,
}

impl State {
    async fn new(window: Arc<Window>) -> Result<State, InitializationError> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_limits: wgpu::Limits {
                    max_buffer_size: 2u64.pow(31) - 1,
                    max_texture_dimension_2d: 2u32.pow(14),
                    ..Default::default()
                },
                ..Default::default()
            })
            .await?;
        let device = Arc::new(device);
        let queue = Arc::new(queue);

        let surface = instance.create_surface(window.clone())?;
        let cap = surface.get_capabilities(&adapter);
        let surface_format = cap.formats[0];

        let texture_bind_group_layout = Scene::create_bind_group_layout(&device);

        let interaction = Interaction::new(&device, &window.inner_size(), surface_format);

        let renderer = Renderer::new(
            &window,
            device.clone(),
            queue.clone(),
            &texture_bind_group_layout,
            surface,
            surface_format,
            &interaction,
        );

        Ok(State {
            window,
            device,
            queue,
            texture_bind_group_layout,
            renderer,
            interaction,
            scene: None,
        })
    }

    fn apply_user_event(&mut self, event: UserEvent) {
        match event {
            UserEvent::ResetView => {
                self.interaction.reset();
                self.scene = None;
            }
            UserEvent::GetPixel(sender) => {
                if let Some(scene) = &self.scene
                    && let Some(texture_image) = scene.get_texture_image()
                {
                    self.interaction.pixel_picker.write_to_channel(
                        self.device.clone(),
                        scene.get_topology_image(),
                        texture_image,
                        sender,
                    );
                } else {
                    send_err(sender, "Texture not initialized")
                }
            }
            UserEvent::CaptureImage(sender) => {
                if self.scene.is_some() {
                    self.interaction
                        .image_capture
                        .write_to_channel(self.device.clone(), sender)
                } else {
                    send_err(sender, "Texture not initialized");
                }
            }
            UserEvent::SetFragmentShader(variant) => {
                log::info!("Setting shader: {:?}", variant);
                self.interaction.set_fragment_shader_variant(variant);
            }
            UserEvent::SetOverlays(overlays) => {
                log::info!("Setting overlays");
                if let Some(scene) = &mut self.scene {
                    scene.set_overlays(overlays, &self.queue);
                }
            }
            UserEvent::ClearOverlays => {
                log::info!("Clearing overlays");
                if let Some(scene) = &mut self.scene {
                    scene.clear_overlays(&self.queue);
                }
            }
            UserEvent::SetOrientation(orientation) => {
                self.interaction.set_orientation(orientation);
            }
            UserEvent::ResetOrientation => {
                self.interaction.reset_orientation();
            }
            UserEvent::SetTopology(data) => {
                log::info!("Setting new topology image");
                self.interaction
                    .percentile_range_buffer
                    .update_data(&self.queue, data.buffer());

                self.interaction
                    .mip
                    .set_image(data.dimensions().into(), &self.device);

                self.renderer.update_axes_origin(
                    data.dimensions(),
                    self.interaction.percentile_range_buffer.z_range(),
                );

                self.scene = Some(Scene::new_topology(
                    data,
                    &self.device,
                    &self.queue,
                    &self.texture_bind_group_layout,
                ));
            }
            UserEvent::SetTexture(data) => {
                log::info!("Setting new texture image");
                if let Some(scene) = &mut self.scene {
                    scene.set_texture(data, &self.queue);
                } else {
                    log::warn!("Can't set texture image, topology not initialized");
                }
            }
            UserEvent::ZoomIn => {
                self.interaction.zoom_in();
            }
            UserEvent::ZoomOut => {
                self.interaction.zoom_out();
            }
            UserEvent::SetPercentile(percentile) => {
                let topology = self.scene.as_ref().map(|scene| scene.get_topology_image());
                self.interaction.percentile_range_buffer.update_percentile(
                    &self.queue,
                    percentile,
                    topology,
                );
                self.renderer
                    .update_z_range(self.interaction.percentile_range_buffer.z_range());
            }
            UserEvent::SetTextureRange(start, end) => {
                self.interaction
                    .texture_range_buffer
                    .update(&self.queue, start, end);
            }
            UserEvent::DisplayGrid(visible) => {
                self.renderer.display_grid(visible);
            }
        }
    }
}

fn send_err<T>(
    sender: futures::channel::oneshot::Sender<SharedFuture<Result<T, Arc<anyhow::Error>>>>,
    msg: &str,
) where
    T: Clone,
{
    let msg = msg.to_owned();
    let future: std::pin::Pin<Box<dyn Future<Output = Result<T, Arc<anyhow::Error>>>>> =
        Box::pin(async move { Err(Arc::new(anyhow!("{}", msg))) });
    if sender.send(future.shared()).is_err() {
        log::error!("Failed to return error message");
    }
}

struct ImageViewer3D {
    state: Option<State>,
    proxy: EventLoopProxy<Event>,
    #[cfg(target_arch = "wasm32")]
    canvas_id: String,
}

impl ImageViewer3D {
    pub fn new(
        event_loop: &EventLoop<Event>,
        #[cfg(target_arch = "wasm32")] canvas_id: String,
    ) -> Self {
        let proxy = event_loop.create_proxy();
        Self {
            state: None,
            proxy,
            #[cfg(target_arch = "wasm32")]
            canvas_id,
        }
    }
}

impl ApplicationHandler<Event> for ImageViewer3D {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        #[allow(unused_mut)]
        let mut window_attributes = Window::default_attributes();

        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::JsCast;
            use winit::platform::web::WindowAttributesExtWebSys;

            let window = wgpu::web_sys::window().unwrap_throw();
            let document = window.document().unwrap_throw();
            let canvas = document.get_element_by_id(&self.canvas_id).unwrap_throw();
            let html_canvas_element = canvas.unchecked_into();
            window_attributes = window_attributes.with_canvas(Some(html_canvas_element));
        }

        let window = Arc::new(match event_loop.create_window(window_attributes) {
            Ok(window) => window,
            Err(e) => {
                let _ = self.proxy.send_event(
                    ErrorEvent::Initialization(InitializationError::CreateWindowError(e)).into(),
                );
                return;
            }
        });

        #[cfg(not(target_arch = "wasm32"))]
        {
            // If we are not on web we can use pollster to
            // await the async initialization directly
            match pollster::block_on(State::new(window)) {
                Ok(state) => self.state = Some(state),
                Err(e) => {
                    let _ = self.proxy.send_event(ErrorEvent::Initialization(e).into());
                }
            }
        }

        #[cfg(target_arch = "wasm32")]
        {
            // Run the future asynchronously and use the
            // proxy to send the results to the event loop
            let proxy = self.proxy.clone();
            wasm_bindgen_futures::spawn_local(async move {
                assert!(match State::new(window).await {
                    Ok(state) => proxy
                        .send_event(SystemEvent::SetState(state).into())
                        .is_ok(),
                    Err(e) => proxy
                        .send_event(ErrorEvent::Initialization(e).into())
                        .is_ok(),
                })
            });
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        if self.state.is_none() {
            log::warn!("State is None, ignoring event");
            return;
        }

        if let Some(app_state) = self.state.as_mut() {
            let mut request_redraw = false;
            app_state.interaction.handle_event(
                &mut request_redraw,
                &event,
                app_state.window.inner_size(),
                &app_state.device,
            );
            match event {
                WindowEvent::CloseRequested => {
                    println!("The close button was pressed; stopping");
                    event_loop.exit();
                }
                WindowEvent::RedrawRequested => {
                    if let Some(scene) = &app_state.scene {
                        app_state.renderer.render(
                            app_state.window.clone(),
                            &app_state.interaction,
                            scene,
                        );
                    }
                }
                WindowEvent::Resized(size) => {
                    request_redraw = true;
                    app_state.renderer.configure_surface(size);
                }
                _ => (),
            }
            if request_redraw {
                app_state.window.request_redraw();
            }
        }
    }

    #[allow(unused_mut)]
    fn user_event(&mut self, event_loop: &ActiveEventLoop, mut event: Event) {
        match event {
            Event::Error(ErrorEvent::Initialization(e)) => {
                log::error!("{}", e);
                event_loop.exit();
            }
            Event::System(SystemEvent::SetState(mut state)) => {
                #[cfg(target_arch = "wasm32")]
                {
                    // Resize first while we still own the event
                    state.renderer.configure_surface(state.window.inner_size());
                    // Update projection aspect ratio to match viewport
                    state.interaction.projection.update_aspect_ratio(
                        state.window.inner_size().width as f32
                            / state.window.inner_size().height as f32,
                    );
                    // Store window reference for JavaScript to request redraws
                    wasm_commands::set_window(state.window.clone());
                }

                // Set state BEFORE requesting redraw so the RedrawRequested handler can access it
                self.state = Some(state);

                #[cfg(target_arch = "wasm32")]
                {
                    // Now request redraw - state is already set
                    if let Some(state) = self.state.as_ref() {
                        state.window.request_redraw();
                    }
                }
            }
            Event::User(user_event) => {
                if let Some(app_state) = self.state.as_mut() {
                    app_state.apply_user_event(user_event);
                }
            }
        }
        if let Some(app_state) = self.state.as_mut() {
            app_state.window.request_redraw();
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn run() -> anyhow::Result<()> {
    use crate::gpu_data::TopologyData;

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_secs()
        .init();

    let data = TopologyData::from_file("example-img.tiff").unwrap();
    let event_loop = EventLoop::with_user_event().build()?;
    let proxy = event_loop.create_proxy();
    proxy
        .send_event(UserEvent::SetTopology(data.0).into())
        .map_err(|e| anyhow!("Error: {}", e))?;

    let mut app = ImageViewer3D::new(&event_loop);
    event_loop.run_app(&mut app)?;

    Ok(())
}

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    if let Err(e) = run() {
        log::error!("Failed to run image viewer: {}", e)
    };
}
