#[cfg(not(target_arch = "wasm32"))]
use anyhow::anyhow;
use glam::{Vec2, Vec3};
use log::error;
use std::{borrow::Cow, num::NonZeroU32, sync::Arc, vec};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
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

mod image;
mod index_buffer;
mod keyboard;
mod mip;
mod mouse;
mod pixel_picker;
mod projection;
mod scene;
mod transformation;
mod user_events;
mod vertex_buffer;
#[cfg(target_arch = "wasm32")]
mod wasm_viewer;
use mouse::Mouse;
use projection::Projection;

use crate::{
    image::{Capture, DataSize, SurfacePercentileRangeBuffer, TextureImageRangeBuffer},
    keyboard::Keyboard,
    mip::Mip,
    pixel_picker::PixelPicker,
    scene::Scene,
    transformation::Transformation,
    user_events::{UserEvent, UserEventHandler},
    vertex_buffer::VertexBuffer,
};

struct State {
    window: Arc<Window>,
    device: Arc<wgpu::Device>,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    surface_format: wgpu::TextureFormat,
    surface_usages: wgpu::TextureUsages,
    mouse: Mouse,
    keyboard: Keyboard,
    transformation: Transformation,
    projection: Projection,
    render_pipeline_texture: wgpu::RenderPipeline,
    render_pipeline_height: wgpu::RenderPipeline,
    use_height_shader: bool,
    texture_bind_group_layout: wgpu::BindGroupLayout,
    mip: Mip,
    scene: Option<Scene>,
    percentile_range_buffer: SurfacePercentileRangeBuffer,
    texture_range_buffer: TextureImageRangeBuffer,
    image_info_bind_group: wgpu::BindGroup,
    depth_view: wgpu::TextureView,
    pixel_picker: PixelPicker,
    image_capture: Capture,
}

impl State {
    async fn new(window: Arc<Window>) -> State {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_limits: wgpu::Limits {
                    max_buffer_size: 2u64.pow(31) - 1,
                    max_texture_dimension_2d: 2u32.pow(14),
                    ..Default::default()
                },
                ..Default::default()
            })
            .await
            .unwrap();
        let device = Arc::new(device);

        let surface = instance.create_surface(window.clone()).unwrap();
        let cap = surface.get_capabilities(&adapter);
        let surface_format = cap.formats[0];
        let surface_usages = cap.usages;

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader.wgsl"))),
        });

        let image_info_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("image_info_bind_group_layout"),
                entries: &[
                    DataSize::get_bind_group_layout_entry(),
                    SurfacePercentileRangeBuffer::get_bind_group_layout_entry(),
                    TextureImageRangeBuffer::get_bind_group_layout_entry(),
                    Mip::get_bind_group_layout_entry(),
                ],
            });

        let texture_bind_group_layout = Scene::create_bind_group_layout(&device);

        let pixel_picker = PixelPicker::new(&device, window.inner_size());
        let image_capture = Capture::new(
            &device,
            DataSize {
                width: NonZeroU32::new(window.inner_size().width)
                    .expect("Windows size should not be 0"),
                height: NonZeroU32::new(window.inner_size().height)
                    .expect("Windows size should not be 0"),
            },
            surface_format,
        );

        let mip = Mip::new(&device);

        let percentile_range_buffer = SurfacePercentileRangeBuffer::new(&device);
        let texture_range_buffer = TextureImageRangeBuffer::new(&device);
        let image_info_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("image_info_bind_group"),
            layout: &image_info_bind_group_layout,
            entries: &[
                DataSize::get_bind_group_entry(&mip.image_dims_buffer),
                percentile_range_buffer.get_bind_group_entry(),
                texture_range_buffer.get_bind_group_entry(),
                mip.get_bind_group_entry(),
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
                    &texture_bind_group_layout,
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
        let texture_fs_pipeline_descriptor = &wgpu::RenderPipelineDescriptor {
            label: Some("texture_pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[VertexBuffer::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_texture"),
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

        let render_pipeline_texture = device.create_render_pipeline(texture_fs_pipeline_descriptor);

        let mut height_fs_pipeline_descriptor = texture_fs_pipeline_descriptor.clone();
        height_fs_pipeline_descriptor.label = Some("height_pipeline");
        height_fs_pipeline_descriptor.fragment = Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_height"),
            compilation_options: Default::default(),
            targets: &texture_formats,
        });
        let render_pipeline_height = device.create_render_pipeline(&height_fs_pipeline_descriptor);

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
            surface_usages,
            mouse: Mouse::new(),
            keyboard: Keyboard::new(),
            transformation,
            projection,
            render_pipeline_texture,
            render_pipeline_height,
            use_height_shader: true,
            texture_bind_group_layout,
            mip,
            scene: None,
            percentile_range_buffer,
            texture_range_buffer,
            image_info_bind_group,
            depth_view,
            pixel_picker,
            image_capture,
        };

        // Configure surface for the first time
        state.configure_surface();

        state
    }

    fn get_window(&self) -> &Window {
        &self.window
    }

    fn configure_surface(&mut self) {
        let mut usage = wgpu::TextureUsages::RENDER_ATTACHMENT;
        if self.surface_usages.contains(wgpu::TextureUsages::COPY_SRC) {
            usage |= wgpu::TextureUsages::COPY_SRC;
        }
        let surface_config = wgpu::SurfaceConfiguration {
            usage,
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
            &self.render_pipeline_texture
        };
        renderpass.set_pipeline(pipeline);
        if let Some(scene) = &self.scene {
            renderpass.set_bind_group(0, &scene.bind_group, &[]);
        }
        renderpass.set_bind_group(1, &self.image_info_bind_group, &[]);
        renderpass.set_bind_group(2, &self.transformation.bind_group, &[]);
        renderpass.set_bind_group(3, &self.projection.bind_group, &[]);

        self.mip.update_gpu(&mut renderpass, &self.queue);

        // End the renderpass.
        drop(renderpass);

        if self.surface_usages.contains(wgpu::TextureUsages::COPY_SRC) {
            self.image_capture
                .copy_texture(&mut encoder, &surface_texture);
        }
        self.pixel_picker.copy_pixel_at_mouse(&mut encoder);
        self.transformation.update_gpu(&self.queue);
        self.projection.update_gpu(&self.queue);
        // Submit the command in the queue to execute
        self.queue.submit([encoder.finish()]);
        self.window.pre_present_notify();
        surface_texture.present();

        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(scene) = &self.scene {
                if let Some(texture_image) = &scene.texture.image {
                    match pollster::block_on(self.pixel_picker.get(
                        self.device.clone(),
                        scene.surface.image.clone(),
                        texture_image.clone(),
                    )) {
                        Ok(pixel_value) => {
                            log::info!(
                                "Pixel at [{}/{}]={:.3}, texture={}",
                                pixel_value.x,
                                pixel_value.y,
                                pixel_value.z,
                                pixel_value.texture
                            );
                        }
                        Err(e) => {
                            log::error!("Pixel read failed: {}", e);
                        }
                    }
                } else {
                    log::error!("Texture image not initialized");
                }
            } else {
                log::error!("Texture not initialized");
            }
        }
    }
}

struct ImageViewer3D {
    state: Option<State>,
    #[cfg(target_arch = "wasm32")]
    proxy: Option<winit::event_loop::EventLoopProxy<UserEvent>>,
    #[cfg(target_arch = "wasm32")]
    canvas_id: String,
}

impl ImageViewer3D {
    pub fn new(
        #[cfg(target_arch = "wasm32")] event_loop: &EventLoop<UserEvent>,
        #[cfg(target_arch = "wasm32")] canvas_id: String,
    ) -> Self {
        #[cfg(target_arch = "wasm32")]
        let proxy = Some(event_loop.create_proxy());
        Self {
            state: None,
            #[cfg(target_arch = "wasm32")]
            proxy,
            #[cfg(target_arch = "wasm32")]
            canvas_id,
        }
    }
}

impl ApplicationHandler<UserEvent> for ImageViewer3D {
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
                    assert!(
                        proxy
                            .send_event(UserEvent::SetState(State::new(window).await))
                            .is_ok()
                    )
                });
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
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
                    app_state.mip.set_zoom(app_state.mouse.get_zoom());
                    app_state.get_window().request_redraw();
                }
                WindowEvent::KeyboardInput {
                    device_id: _,
                    event,
                    is_synthetic: _,
                } => {
                    app_state.keyboard.register_event(event.clone());
                }
                _ => (),
            }
        }
    }

    #[allow(unused_mut)]
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, mut event: UserEvent) {
        match event {
            UserEvent::ResetView => {
                if let Some(app_state) = self.state.as_mut() {
                    app_state.reset_view();
                }
            }
            UserEvent::GetPixel(sender) => {
                if let Some(app_state) = self.state.as_mut() {
                    app_state.get_pixel_value(sender);
                }
            }
            UserEvent::CaptureImage(sender) => {
                if let Some(app_state) = self.state.as_mut() {
                    app_state.capture_image(sender);
                }
            }
            UserEvent::SetTextureShader => {
                if let Some(app_state) = self.state.as_mut() {
                    app_state.set_texture_shader();
                }
            }
            UserEvent::SetHeightShader => {
                if let Some(app_state) = self.state.as_mut() {
                    app_state.set_height_shader();
                }
            }
            UserEvent::SetOverlays(overlays) => {
                if let Some(app_state) = self.state.as_mut() {
                    app_state.set_overlays(overlays.clone());
                }
            }
            UserEvent::ClearOverlays => {
                if let Some(app_state) = self.state.as_mut() {
                    app_state.clear_overlays();
                }
            }
            UserEvent::ResetOrientation => {
                if let Some(app_state) = self.state.as_mut() {
                    app_state.reset_orientation();
                }
            }
            UserEvent::SetSurface(data) => {
                if let Some(app_state) = self.state.as_mut() {
                    app_state.set_surface(data);
                } else {
                    log::warn!("State is None, cannot set surface");
                }
            }
            UserEvent::SetTexture(data) => {
                if let Some(app_state) = self.state.as_mut() {
                    app_state.set_texture(data);
                }
            }
            UserEvent::ZoomIn => {
                if let Some(app_state) = self.state.as_mut() {
                    app_state.zoom_in();
                }
            }
            UserEvent::ZoomOut => {
                if let Some(app_state) = self.state.as_mut() {
                    app_state.zoom_out();
                }
            }
            UserEvent::SetPercentile(percentile) => {
                if let Some(app_state) = self.state.as_mut() {
                    app_state.set_percentile(percentile);
                }
            }
            UserEvent::SetTextureRange(start, end) => {
                if let Some(app_state) = self.state.as_mut() {
                    app_state.set_texture_range(start, end);
                }
            }
            UserEvent::SetState(mut state) => {
                #[cfg(target_arch = "wasm32")]
                {
                    // Resize first while we still own the event
                    state.resize(state.window.inner_size());
                    // Update projection aspect ratio to match viewport
                    state.projection.update_aspect_ratio(
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
            _ => {
                log::warn!("Unhandled user event");
            }
        }
        if let Some(app_state) = self.state.as_mut() {
            app_state.get_window().request_redraw();
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn run() -> anyhow::Result<()> {
    use crate::image::SurfaceData;

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_secs()
        .init();

    let data = SurfaceData::from_file("example-img.tiff").unwrap();
    let event_loop = EventLoop::with_user_event().build()?;
    let proxy = event_loop.create_proxy();
    proxy
        .send_event(UserEvent::SetSurface(data.0))
        .map_err(|e| anyhow!("Error: {}", e))
        .unwrap();

    let mut app = ImageViewer3D::new();
    event_loop.run_app(&mut app)?;

    Ok(())
}

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    if let Err(e) = run() {
        log::error!("Failed to run image viewer: {}", e)
    };
}
