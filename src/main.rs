use anyhow::anyhow;
use futures::FutureExt;
use std::sync::Arc;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{ElementState, MouseButton, WindowEvent},
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
mod keyboard;
mod mouse;
#[cfg(target_arch = "wasm32")]
mod wasm_viewer;

use std::num::NonZeroU32;

use data_viewer_3d::{
    SharedFuture,
    gpu_data::{
        Capture, DataSize, pixel_picker::PixelPicker, texture_image_range::TextureImageRangeBuffer,
        topology_percentile_range::TopologyPercentileRangeBuffer,
    },
    interaction::Interaction,
    render::Renderer,
    scene::Scene,
};

use crate::{
    events::{ErrorEvent, Event, SystemEvent, UserEvent},
    keyboard::Keyboard,
    mouse::Mouse,
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
    surface: wgpu::Surface<'static>,
    surface_format: wgpu::TextureFormat,
    texture_bind_group_layout: wgpu::BindGroupLayout,
    renderer: Renderer,
    interaction: Interaction,
    mouse: Mouse,
    keyboard: Keyboard,
    dragging: bool,
    pixel_picker: PixelPicker,
    image_capture: Capture,
    percentile_range_buffer: TopologyPercentileRangeBuffer,
    texture_range_buffer: TextureImageRangeBuffer,
    scene: Option<Scene>,
}

impl State {
    async fn new(window: Arc<Window>) -> Result<State, InitializationError> {
        let instance =
            wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
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

        let size = window.inner_size();
        let size = (size.width.max(1), size.height.max(1));
        let interaction = Interaction::new(&device, size.0 as f32 / size.1 as f32);
        let pixel_picker = PixelPicker::new(&device, size);
        let image_capture = Capture::new(
            &device,
            DataSize {
                width: NonZeroU32::new(size.0).expect("Takes the maximum of value and 1"),
                height: NonZeroU32::new(size.1).expect("Takes the maximum of value and 1"),
            },
            surface_format,
        );
        let percentile_range_buffer = TopologyPercentileRangeBuffer::new(&device);
        let texture_range_buffer = TextureImageRangeBuffer::new(&device);

        let renderer = Renderer::new(
            device.clone(),
            queue.clone(),
            &texture_bind_group_layout,
            surface_format.add_srgb_suffix(),
            &interaction,
            &percentile_range_buffer,
            &texture_range_buffer,
            size,
        );

        let mut state = State {
            window,
            device,
            queue,
            surface,
            surface_format,
            texture_bind_group_layout,
            renderer,
            interaction,
            mouse: Mouse::default(),
            keyboard: Keyboard::default(),
            dragging: false,
            pixel_picker,
            image_capture,
            percentile_range_buffer,
            texture_range_buffer,
            scene: None,
        };
        state.configure_surface(PhysicalSize::new(size.0, size.1));
        Ok(state)
    }

    fn configure_surface(&mut self, window_size: PhysicalSize<u32>) {
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            format: self.surface_format,
            // Request compatibility with the sRGB-format texture view we're going to create later.
            view_formats: vec![self.surface_format.add_srgb_suffix()],
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            width: window_size.width,
            height: window_size.height,
            desired_maximum_frame_latency: 2,
            present_mode: wgpu::PresentMode::AutoVsync,
        };
        self.surface.configure(&self.device, &surface_config);
    }

    fn render(&mut self) {
        let Some(scene) = &self.scene else {
            return;
        };
        let surface_texture = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(t)
            | wgpu::CurrentSurfaceTexture::Suboptimal(t) => t,
            other => {
                log::warn!("Skipping frame, no surface texture: {:?}", other);
                return;
            }
        };
        let surface_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor {
                // Without add_srgb_suffix() the image we will be working with
                // might not be "gamma correct".
                format: Some(self.surface_format.add_srgb_suffix()),
                ..Default::default()
            });
        self.renderer.render(
            &surface_view,
            &surface_texture.texture,
            &self.interaction,
            &self.pixel_picker,
            &self.image_capture,
            scene,
        );
        self.window.pre_present_notify();
        surface_texture.present();
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
                    if sender
                        .send(self.pixel_picker.get(
                            self.device.clone(),
                            scene.get_topology_image(),
                            texture_image,
                        ))
                        .is_err()
                    {
                        log::error!("Failed to return pixel value");
                    }
                } else {
                    send_err(sender, "Texture not initialized")
                }
            }
            UserEvent::CaptureImage(sender) => {
                if self.scene.is_some() {
                    if sender
                        .send(self.image_capture.get(self.device.clone()))
                        .is_err()
                    {
                        log::error!("Failed to return capture");
                    }
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
                self.percentile_range_buffer
                    .update_data(&self.queue, data.buffer());

                self.interaction
                    .mip
                    .set_image(data.dimensions().into(), &self.device);

                self.renderer
                    .update_axes_origin(data.dimensions(), self.percentile_range_buffer.z_range());

                self.scene = Some(Scene::new_topology(
                    data,
                    &self.device,
                    &self.queue,
                    &self.texture_bind_group_layout,
                ));
            }
            UserEvent::SetTopologyMasked(data, mask) => {
                log::info!("Setting new masked topology image");
                self.percentile_range_buffer
                    .update_data(&self.queue, data.buffer());

                self.interaction.mip.set_image_masked(
                    data.dimensions().into(),
                    &mask,
                    &self.device,
                );

                self.renderer
                    .update_axes_origin(data.dimensions(), self.percentile_range_buffer.z_range());

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
                self.percentile_range_buffer
                    .update_percentile(&self.queue, percentile, topology);
                self.renderer
                    .update_z_range(self.percentile_range_buffer.z_range());
            }
            UserEvent::SetTextureRange(start, end) => {
                self.texture_range_buffer.update(&self.queue, start, end);
            }
            UserEvent::SetMipOverride(level) => {
                self.interaction.mip.set_override_level(level);
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
        let Some(app_state) = self.state.as_mut() else {
            log::warn!("State is None, ignoring event");
            return;
        };
        let window_size = app_state.window.inner_size();
        let mut request_redraw = false;
        match event {
            WindowEvent::CloseRequested => {
                println!("The close button was pressed; stopping");
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                app_state.render();
            }
            WindowEvent::Resized(size) => {
                if size.width > 0 && size.height > 0 {
                    request_redraw = true;
                    app_state.configure_surface(size);
                    app_state.renderer.resize((size.width, size.height));
                    app_state
                        .interaction
                        .update_aspect_ratio(size.width as f32 / size.height as f32);
                    app_state
                        .pixel_picker
                        .resize(&app_state.device, (size.width, size.height));
                    app_state.image_capture.resize(
                        &app_state.device,
                        DataSize {
                            width: NonZeroU32::new(size.width)
                                .expect("Window size should not be 0"),
                            height: NonZeroU32::new(size.height)
                                .expect("Window size should not be 0"),
                        },
                    );
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                app_state.mouse.register_move_event(position);
                app_state
                    .pixel_picker
                    .update_mouse_position(position.x as f32, position.y as f32);
                if app_state.dragging {
                    match app_state.mouse.get_device_coordinates(window_size) {
                        Ok(ndc) => {
                            if app_state.mouse.is_pointer_inside(ndc) {
                                request_redraw = true;
                                app_state
                                    .interaction
                                    .drag(ndc, (window_size.width, window_size.height));
                            }
                        }
                        Err(e) => log::error!("Failed to calculate pointer position: {}", e),
                    }
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if button == MouseButton::Left {
                    if state == ElementState::Pressed {
                        match app_state.mouse.get_device_coordinates(window_size) {
                            Ok(ndc) => {
                                app_state
                                    .interaction
                                    .begin_drag(ndc, app_state.keyboard.is_control_pressed());
                                app_state.dragging = true;
                            }
                            Err(e) => log::error!("Failed to calculate pointer position: {}", e),
                        }
                    } else {
                        app_state.interaction.end_drag();
                        app_state.dragging = false;
                    }
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                request_redraw = true;
                app_state.interaction.scroll(Mouse::scroll_delta(&delta));
            }
            WindowEvent::KeyboardInput { event, .. } => {
                app_state.keyboard.register_event(event);
            }
            _ => (),
        }
        if request_redraw {
            app_state.window.request_redraw();
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
                    let size = state.window.inner_size();
                    state.configure_surface(size);
                    state.renderer.resize((size.width, size.height));
                    state
                        .interaction
                        .update_aspect_ratio(size.width as f32 / size.height as f32);
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
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_secs()
        .init();

    let file = std::fs::File::open("example-img.tiff")?;
    let data = data_viewer_3d::tiff_decode::decode_tiff::<f32, _>(file)?;
    let event_loop = EventLoop::with_user_event().build()?;
    let proxy = event_loop.create_proxy();
    proxy
        .send_event(UserEvent::SetTopology(data).into())
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
