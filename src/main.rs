use glam::{Vec2, Vec3};
use log::error;
use std::{borrow::Cow, sync::Arc, vec};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum ViewerCommand {
    BackToOrigin,
    SetAmplitudeShader,
    SetHeightShader,
    SetOverlays(Arc<Vec<Overlay>>),
    ClearOverlays,
}

#[cfg(target_arch = "wasm32")]
mod wasm_commands {
    use super::ViewerCommand;
    use std::cell::RefCell;
    use std::collections::VecDeque;
    use std::sync::Arc;
    use winit::window::Window;

    thread_local! {
        /// Queue of commands from JavaScript to be processed by the viewer
        pub static COMMAND_QUEUE: RefCell<VecDeque<ViewerCommand>> = RefCell::new(VecDeque::new());
        /// Reference to the window for requesting redraws
        pub static WINDOW: RefCell<Option<Arc<Window>>> = RefCell::new(None);
    }

    pub fn set_window(window: Arc<Window>) {
        WINDOW.with(|w| *w.borrow_mut() = Some(window));
    }

    pub fn push_command(cmd: ViewerCommand) {
        COMMAND_QUEUE.with(|q| q.borrow_mut().push_back(cmd));
    }

    pub fn pop_command() -> Option<ViewerCommand> {
        COMMAND_QUEUE.with(|q| q.borrow_mut().pop_front())
    }
}

// JavaScript-callable functions
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn viewer_back_to_origin() {
    wasm_commands::push_command(ViewerCommand::BackToOrigin);
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn viewer_set_amplitude_shader() {
    wasm_commands::push_command(ViewerCommand::SetAmplitudeShader);
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn viewer_set_height_shader() {
    wasm_commands::push_command(ViewerCommand::SetHeightShader);
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn viewer_set_overlays() {
    let overlays = texture::example_overlays();
    wasm_commands::push_command(ViewerCommand::SetOverlays(Arc::new(overlays)));
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn viewer_clear_overlays() {
    wasm_commands::push_command(ViewerCommand::ClearOverlays);
}

mod image;
mod index_buffer;
mod keyboard;
mod mouse;
mod pixel_picker;
mod projection;
mod texture;
mod transformation;
mod vertex_buffer;
use image::SurfaceAmplitudeImage;
use mouse::Mouse;
use projection::Projection;

use crate::{
    image::{ImageSize, ZValueRange},
    index_buffer::{IndexBuffer, IndexBufferBuilder},
    keyboard::Keyboard,
    pixel_picker::PixelPicker,
    texture::{Overlay, Texture},
    transformation::Transformation,
    vertex_buffer::VertexBuffer,
};

struct State {
    window: Arc<Window>,
    device: Arc<wgpu::Device>,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    surface_format: wgpu::TextureFormat,
    mouse: Mouse,
    keyboard: Keyboard,
    transformation: Transformation,
    projection: Projection,
    render_pipeline_amplitude: wgpu::RenderPipeline,
    render_pipeline_height: wgpu::RenderPipeline,
    use_height_shader: bool,
    vertex_buffer: VertexBuffer,
    index_buffer: IndexBuffer,
    texture: Texture,
    image_info_bind_group: wgpu::BindGroup,
    depth_view: wgpu::TextureView,
    pixel_picker: PixelPicker,
    zoom_buffer: wgpu::Buffer,
}

impl State {
    async fn new(window: Arc<Window>) -> State {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .unwrap();
        let device = Arc::new(device);

        let surface = instance.create_surface(window.clone()).unwrap();
        let cap = surface.get_capabilities(&adapter);
        let surface_format = cap.formats[0];

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader.wgsl"))),
        });

        #[cfg(not(target_arch = "wasm32"))]
        let image = SurfaceAmplitudeImage::from_file("example-img.tiff").unwrap();
        #[cfg(target_arch = "wasm32")]
        let image = SurfaceAmplitudeImage::from_url("http://localhost:8081/img")
            .await
            .unwrap();

        let outlier_removed_data = image.surface.outlier_removed_data(2.0, 98.0);
        let z_range = image::value_range(&outlier_removed_data);

        let image_dims_buffer = image.surface.size.create_buffer_init(&device);
        let z_value_range_buffer = z_range.create_buffer_init(&device);
        let image_info_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("image_info_bind_group_layout"),
                entries: &[
                    ImageSize::get_bind_group_layout_entry(),
                    ZValueRange::<f32>::get_bind_group_layout_entry(),
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let vertex_buffer = VertexBuffer::new(&image.surface.size, &outlier_removed_data, &device);
        let index_buffer =
            IndexBufferBuilder::new_triangle_strip(&image.surface.size).create_buffer_init(&device);

        let texture = Texture::new(&device, image);
        texture.write_to_queue(&queue);

        let pixel_picker = PixelPicker::new(&device, window.inner_size());
        let zoom_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("mip_level_buffer"),
            contents: bytemuck::cast_slice(&[2u32]),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
        });
        let image_info_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("image_info_bind_group"),
            layout: &image_info_bind_group_layout,
            entries: &[
                ImageSize::get_bind_group_entry(&image_dims_buffer),
                ZValueRange::<f32>::get_bind_group_entry(&z_value_range_buffer),
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: zoom_buffer.as_entire_binding(),
                },
            ],
        });

        let mut transformation = Transformation::default();
        let transformation_bind_group_layout = transformation.create_bind_group(&device);
        let mut projection = Projection::default();
        let projection_bind_group_layout = projection.create_bind_group(&device);

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("render_pipeline_layout"),
                bind_group_layouts: &[
                    &texture.bind_group_layout,
                    &image_info_bind_group_layout,
                    &transformation_bind_group_layout,
                    &projection_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

        const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

        // Two render targets: main color + picking texture
        let texture_formats = [
            Some(surface_format.add_srgb_suffix().into()),
            Some(PixelPicker::PICKING_FORMAT.into()),
        ];
        let amplitude_pipeline_descriptor = &wgpu::RenderPipelineDescriptor {
            label: Some("amplitude_pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[VertexBuffer::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_amplitude"),
                compilation_options: Default::default(),
                targets: &texture_formats,
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                strip_index_format: Some(wgpu::IndexFormat::Uint32),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        };

        let render_pipeline_amplitude =
            device.create_render_pipeline(amplitude_pipeline_descriptor);

        let mut height_pipeline_descriptor = amplitude_pipeline_descriptor.clone();
        height_pipeline_descriptor.label = Some("height_pipeline");
        height_pipeline_descriptor.fragment = Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_height"),
            compilation_options: Default::default(),
            targets: &texture_formats,
        });
        let render_pipeline_height = device.create_render_pipeline(&height_pipeline_descriptor);

        // Create depth texture view
        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("depth_texture"),
            size: wgpu::Extent3d {
                width: window.inner_size().width.max(1),
                height: window.inner_size().height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut state = State {
            window,
            device,
            queue,
            surface,
            surface_format,
            mouse: Mouse::new(),
            keyboard: Keyboard::new(),
            transformation,
            projection,
            render_pipeline_amplitude,
            render_pipeline_height,
            use_height_shader: true,
            vertex_buffer,
            index_buffer,
            texture,
            image_info_bind_group,
            depth_view,
            pixel_picker,
            zoom_buffer,
        };

        // Configure surface for the first time
        state.configure_surface();

        state
    }

    fn get_window(&self) -> &Window {
        &self.window
    }

    fn configure_surface(&mut self) {
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: self.surface_format,
            // Request compatibility with the sRGB-format texture view we‘re going to create later.
            view_formats: vec![self.surface_format.add_srgb_suffix()],
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            width: self.window.inner_size().width,
            height: self.window.inner_size().height,
            desired_maximum_frame_latency: 2,
            present_mode: wgpu::PresentMode::AutoVsync,
        };
        self.surface.configure(&self.device, &surface_config);
        // Recreate depth texture to match the new size
        let depth_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("depth_texture"),
            size: wgpu::Extent3d {
                width: self.window.inner_size().width.max(1),
                height: self.window.inner_size().height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        self.depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.configure_surface();
        // Resize the picking texture to match the new window size
        self.pixel_picker.resize(&self.device, new_size);
    }

    fn render(&mut self) {
        // Create texture view
        let surface_texture = self
            .surface
            .get_current_texture()
            .expect("failed to acquire next swapchain texture");
        let texture_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor {
                // Without add_srgb_suffix() the image we will be working with
                // might not be "gamma correct".
                format: Some(self.surface_format.add_srgb_suffix()),
                ..Default::default()
            });

        let mut encoder = self.device.create_command_encoder(&Default::default());

        // Create the renderpass which will clear the screen.
        // Two color attachments: main color + picking texture
        let mut renderpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[
                Some(wgpu::RenderPassColorAttachment {
                    view: &texture_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                }),
                Some(wgpu::RenderPassColorAttachment {
                    view: &self.pixel_picker.picking_texture_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                }),
            ],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        let pipeline = if self.use_height_shader {
            &self.render_pipeline_height
        } else {
            &self.render_pipeline_amplitude
        };
        renderpass.set_pipeline(pipeline);
        renderpass.set_bind_group(0, &self.texture.bind_group, &[]);
        renderpass.set_bind_group(1, &self.image_info_bind_group, &[]);
        renderpass.set_bind_group(2, &self.transformation.bind_group, &[]);
        renderpass.set_bind_group(3, &self.projection.bind_group, &[]);
        renderpass.set_vertex_buffer(0, self.vertex_buffer.buffer.slice(..));
        renderpass.set_index_buffer(
            self.index_buffer.buffer.slice(..),
            wgpu::IndexFormat::Uint32,
        );
        renderpass.draw_indexed(
            0..self.index_buffer.buffer.size() as u32 / std::mem::size_of::<u32>() as u32,
            0,
            0..1,
        );

        // End the renderpass.
        drop(renderpass);

        self.pixel_picker.copy_pixel_at_mouse(&mut encoder);

        let zoom = self.mouse.get_zoom();
        if zoom > 0.8 {
            self.queue
                .write_buffer(&self.zoom_buffer, 0, bytemuck::cast_slice(&[2u32]));
        } else if zoom > 0.2 {
            self.queue
                .write_buffer(&self.zoom_buffer, 0, bytemuck::cast_slice(&[1u32]));
        } else {
            self.queue
                .write_buffer(&self.zoom_buffer, 0, bytemuck::cast_slice(&[0u32]));
        }
        self.texture.overlay.write_to_queue(&self.queue);
        self.transformation.update_gpu(&self.queue);
        self.projection.update_gpu(&self.queue);
        // Submit the command in the queue to execute
        self.queue.submit([encoder.finish()]);
        self.window.pre_present_notify();
        surface_texture.present();

        #[cfg(target_arch = "wasm32")]
        {
            let future = self.pixel_picker.get(self.device.clone());
            wasm_bindgen_futures::spawn_local(async move {
                match future.await {
                    Ok((x, y)) => {
                        log::info!("Pixel at [{}/{}]", x, y);
                    }
                    Err(e) => {
                        log::error!("Pixel read failed: {}", e);
                    }
                }
            });
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            match pollster::block_on(self.pixel_picker.get(self.device.clone())) {
                Ok((x, y)) => {
                    let z = self.texture.surface.image.get_pixel(x, y);
                    log::info!("Pixel at [{}/{}]={:.3}", x, y, z);
                }
                Err(e) => {
                    log::error!("Pixel read failed: {}", e);
                }
            };
        }
    }

    /// Process any pending commands from JavaScript (WASM only)
    #[cfg(target_arch = "wasm32")]
    pub fn process_commands(&mut self) {
        while let Some(cmd) = wasm_commands::pop_command() {
            match cmd {
                ViewerCommand::BackToOrigin => {
                    self.projection.reset();
                    self.transformation.reset();
                }
                ViewerCommand::SetAmplitudeShader => {
                    self.use_height_shader = false;
                }
                ViewerCommand::SetHeightShader => {
                    self.use_height_shader = true;
                }
                ViewerCommand::SetOverlays(overlays) => {
                    self.texture.overlay.set_overlays(overlays.clone());
                }
                ViewerCommand::ClearOverlays => {
                    self.texture.overlay.set_overlays(Arc::new(Vec::new()));
                }
            }
        }
        self.get_window().request_redraw();
    }
}

struct ImageViewer3D {
    #[cfg(target_arch = "wasm32")]
    proxy: Option<winit::event_loop::EventLoopProxy<State>>,
    state: Option<State>,
}

impl ImageViewer3D {
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

impl ApplicationHandler<State> for ImageViewer3D {
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
            self.state = Some(pollster::block_on(State::new(window)));
        }

        #[cfg(target_arch = "wasm32")]
        {
            // Run the future asynchronously and use the
            // proxy to send the results to the event loop
            if let Some(proxy) = self.proxy.take() {
                wasm_bindgen_futures::spawn_local(async move {
                    assert!(proxy.send_event(State::new(window).await).is_ok())
                });
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        // Debug: log all events on WASM
        #[cfg(target_arch = "wasm32")]
        {
            match &event {
                WindowEvent::RedrawRequested => {} // Don't spam redraw logs
                _ => log::info!("Window event: {:?}", event),
            }
        }

        if self.state.is_none() {
            log::warn!("State is None, ignoring event");
            return;
        }

        if let Some(app_state) = self.state.as_mut() {
            match event {
                WindowEvent::CloseRequested => {
                    println!("The close button was pressed; stopping");
                    event_loop.exit();
                }
                WindowEvent::RedrawRequested => {
                    // Process any pending commands from JavaScript (WASM only)
                    #[cfg(target_arch = "wasm32")]
                    app_state.process_commands();

                    app_state.render();
                }
                WindowEvent::Resized(size) => {
                    app_state.resize(size);
                    app_state.projection.update_aspect_ratio(
                        app_state.window.inner_size().width as f32
                            / app_state.window.inner_size().height as f32,
                    );
                }
                WindowEvent::CursorMoved {
                    device_id: _,
                    position,
                } => {
                    app_state.mouse.register_move_event(position);
                    app_state.pixel_picker.update_mouse_position(position);
                    if app_state.mouse.is_left_button_pressed() {
                        match app_state
                            .mouse
                            .get_device_coordinates(app_state.window.inner_size())
                        {
                            Ok(new_position) => {
                                if app_state.mouse.is_pointer_inside(Vec2::from(new_position)) {
                                    if app_state.keyboard.is_control_pressed() {
                                        app_state.projection.change_position(new_position);
                                    } else {
                                        app_state
                                            .transformation
                                            .rotate(Vec3::from((new_position, 1.0)));
                                    }
                                }
                            }
                            Err(e) => error!("Failed to calculate pointer position: {}", e),
                        }
                    }
                    app_state.get_window().request_redraw();
                }
                WindowEvent::MouseInput {
                    device_id: _,
                    state,
                    button,
                } => {
                    app_state.mouse.register_button_event(button, state);
                    if app_state.mouse.is_left_button_pressed() {
                        match app_state
                            .mouse
                            .get_device_coordinates(app_state.window.inner_size())
                        {
                            Ok(pos) => {
                                if app_state.keyboard.is_control_pressed() {
                                    app_state.projection.start_move(pos);
                                } else {
                                    app_state.transformation.start_move(Vec3::from((pos, 1.0)))
                                };
                            }
                            Err(e) => error!("Failed to calculate pointer position: {}", e),
                        }
                    }
                }
                WindowEvent::MouseWheel {
                    device_id: _,
                    delta,
                    phase: _,
                } => {
                    app_state.mouse.register_scroll_event(delta);
                    app_state.projection.zoom(app_state.mouse.get_zoom());
                    app_state.get_window().request_redraw();
                }
                WindowEvent::KeyboardInput {
                    device_id: _,
                    event,
                    is_synthetic: _,
                } => {
                    app_state.keyboard.register_event(event.clone());
                    if let winit::keyboard::Key::Character(ref c) = event.logical_key {
                        // Toggle shader with 'S' key
                        if c.as_str() == "s" && event.state == winit::event::ElementState::Pressed {
                            app_state.use_height_shader = !app_state.use_height_shader;
                            app_state.get_window().request_redraw();
                        }
                        // Toggle overlay with 'T' key
                        if c.as_str() == "t" && event.state == winit::event::ElementState::Pressed {
                            if app_state.texture.overlay.overlays.is_empty() {
                                app_state
                                    .texture
                                    .overlay
                                    .set_overlays(Arc::new(texture::example_overlays()));
                            } else {
                                app_state.texture.overlay.set_overlays(Arc::new(Vec::new()));
                            }
                            app_state.get_window().request_redraw();
                        }
                        // Move object to origin with 'O' key
                        if c.as_str() == "o" && event.state == winit::event::ElementState::Pressed {
                            app_state.projection.reset();
                            app_state.transformation.reset();
                            app_state.get_window().request_redraw();
                        }
                    }
                }
                _ => (),
            }
        }
    }

    #[allow(unused_mut)]
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, mut event: State) {
        #[cfg(target_arch = "wasm32")]
        {
            // Resize first while we still own the event
            event.resize(event.window.inner_size());
            // Update projection aspect ratio to match viewport
            event.projection.update_aspect_ratio(
                event.window.inner_size().width as f32 / event.window.inner_size().height as f32,
            );
            // Store window reference for JavaScript to request redraws
            wasm_commands::set_window(event.window.clone());
        }

        // Set state BEFORE requesting redraw so the RedrawRequested handler can access it
        self.state = Some(event);

        #[cfg(target_arch = "wasm32")]
        {
            // Now request redraw - state is already set
            if let Some(state) = self.state.as_ref() {
                state.window.request_redraw();
            }
        }
    }
}

pub fn run() -> anyhow::Result<()> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
            .format_timestamp_secs()
            .init();
    }
    #[cfg(target_arch = "wasm32")]
    {
        console_log::init_with_level(log::Level::Info).unwrap_throw();
    }

    let event_loop = EventLoop::with_user_event().build()?;
    let mut app = ImageViewer3D::new(
        #[cfg(target_arch = "wasm32")]
        &event_loop,
    );
    event_loop.run_app(&mut app)?;

    Ok(())
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn run_web() -> Result<(), wasm_bindgen::JsValue> {
    console_error_panic_hook::set_once();
    run().unwrap_throw();

    Ok(())
}

fn main() {
    if let Err(e) = run() {
        log::error!("Failed to run image viewer: {}", e)
    };
}
