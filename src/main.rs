use anyhow::anyhow;
use futures::{FutureExt, future::Shared};
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

#[non_exhaustive]
#[allow(dead_code)]
enum ViewerCommand {
    SetSurface(Image<f32>),
    SetAmplitude(Image<u16>),
    SetState(State),
    BackToOrigin,
    SetAmplitudeShader,
    SetHeightShader,
    SetOverlays(Arc<Vec<Overlay>>),
    ClearOverlays,
    GetPixel(
        futures::channel::oneshot::Sender<
            Shared<std::pin::Pin<Box<dyn std::future::Future<Output = PixelResult>>>>,
        >,
    ),
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub struct WasmViewer {
    proxy: Option<winit::event_loop::EventLoopProxy<ViewerCommand>>,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl WasmViewer {
    pub fn new() -> Result<Self, wasm_bindgen::JsValue> {
        Ok(Self { proxy: None })
    }

    pub fn run(&mut self) -> Result<(), wasm_bindgen::JsValue> {
        console_log::init_with_level(log::Level::Info).map_err(|e| {
            wasm_bindgen::JsValue::from_str(&format!("Error initializing console_log: {}", e))
        })?;
        console_error_panic_hook::set_once();

        let event_loop = EventLoop::with_user_event().build().map_err(|e| {
            wasm_bindgen::JsValue::from_str(&format!("Error initializing console_log: {}", e))
        })?;
        self.proxy = Some(event_loop.create_proxy());
        wasm_bindgen_futures::spawn_local(async move {
            let mut app = ImageViewer3D::new(&event_loop);
            event_loop
                .run_app(&mut app)
                .map_err(|e| {
                    wasm_bindgen::JsValue::from_str(&format!(
                        "Error initializing console_log: {}",
                        e
                    ))
                })
                .unwrap_throw();
        });
        Ok(())
    }

    pub async fn set_surface(&self, data: Vec<u8>) -> Result<(), wasm_bindgen::JsValue> {
        if let Some(proxy) = &self.proxy {
            let image = Image::<f32>::try_from(data)
                .map_err(|e| wasm_bindgen::JsValue::from_str(&format!("Error: {}", e)))?;
            proxy
                .send_event(ViewerCommand::SetSurface(image))
                .map_err(|e| wasm_bindgen::JsValue::from_str(&format!("Error: {}", e)))?;
            Ok(())
        } else {
            Err(wasm_bindgen::JsValue::from_str(
                "Event loop proxy not initialized",
            ))
        }
    }

    pub async fn set_amplitude(&self, data: Vec<u8>) -> Result<(), wasm_bindgen::JsValue> {
        if let Some(proxy) = &self.proxy {
            let image = Image::<u16>::try_from(data)
                .map_err(|e| wasm_bindgen::JsValue::from_str(&format!("Error: {}", e)))?;
            proxy
                .send_event(ViewerCommand::SetAmplitude(image))
                .map_err(|e| wasm_bindgen::JsValue::from_str(&format!("Error: {}", e)))?;
            Ok(())
        } else {
            Err(wasm_bindgen::JsValue::from_str(
                "Event loop proxy not initialized",
            ))
        }
    }

    pub async fn get_pixel_value(&self) -> Result<Vec<f32>, wasm_bindgen::JsValue> {
        if let Some(proxy) = &self.proxy {
            let (sender, receiver) = futures::channel::oneshot::channel();
            proxy
                .send_event(ViewerCommand::GetPixel(sender))
                .map_err(|e| wasm_bindgen::JsValue::from_str(&format!("Error: {}", e)))?;
            let pixels = receiver
                .await
                .map_err(|e| wasm_bindgen::JsValue::from_str(&format!("Error: {}", e)))?
                .await
                .map(|(x, y, z)| vec![x as f32, y as f32, z])
                .map_err(|e| wasm_bindgen::JsValue::from_str(&format!("Error: {}", e)))?;
            Ok(pixels)
        } else {
            wasm_bindgen::throw_str("Event loop proxy not initialized");
        }
    }

    pub fn set_height_shader(&self) -> Result<(), wasm_bindgen::JsValue> {
        if let Some(proxy) = &self.proxy {
            proxy
                .send_event(ViewerCommand::SetHeightShader)
                .map_err(|e| e.to_string())?;
            Ok(())
        } else {
            Err(wasm_bindgen::JsValue::from_str(
                "Event loop proxy not initialized",
            ))
        }
    }

    pub fn set_amplitude_shader(&self) -> Result<(), wasm_bindgen::JsValue> {
        if let Some(proxy) = &self.proxy {
            proxy
                .send_event(ViewerCommand::SetAmplitudeShader)
                .map_err(|e| e.to_string())?;
            Ok(())
        } else {
            Err(wasm_bindgen::JsValue::from_str(
                "Event loop proxy not initialized",
            ))
        }
    }

    pub fn set_overlays(&self) -> Result<(), wasm_bindgen::JsValue> {
        if let Some(proxy) = &self.proxy {
            proxy
                .send_event(ViewerCommand::SetOverlays(Arc::new(
                    texture::example_overlays(),
                )))
                .map_err(|e| e.to_string())?;
            Ok(())
        } else {
            Err(wasm_bindgen::JsValue::from_str(
                "Event loop proxy not initialized",
            ))
        }
    }

    pub fn clear_overlays(&self) -> Result<(), wasm_bindgen::JsValue> {
        if let Some(proxy) = &self.proxy {
            proxy
                .send_event(ViewerCommand::ClearOverlays)
                .map_err(|e| e.to_string())?;
            Ok(())
        } else {
            Err(wasm_bindgen::JsValue::from_str(
                "Event loop proxy not initialized",
            ))
        }
    }

    pub fn back_to_origin(&self) -> Result<(), wasm_bindgen::JsValue> {
        if let Some(proxy) = &self.proxy {
            proxy
                .send_event(ViewerCommand::BackToOrigin)
                .map_err(|e| e.to_string())?;
            Ok(())
        } else {
            Err(wasm_bindgen::JsValue::from_str(
                "Event loop proxy not initialized",
            ))
        }
    }
}

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
    image::{Image, ImageSize, ZValueRange},
    index_buffer::{IndexBuffer, IndexBufferBuilder},
    keyboard::Keyboard,
    pixel_picker::{PixelPicker, PixelResult},
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
    texture_bind_group_layout: wgpu::BindGroupLayout,
    vertex_buffer: Option<VertexBuffer>,
    index_buffer: Option<IndexBuffer>,
    texture: Option<Texture>,
    image_dims_buffer: wgpu::Buffer,
    z_value_range_buffer: wgpu::Buffer,
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

        let texture_bind_group_layout = Texture::create_bind_group_layout(&device);

        let pixel_picker = PixelPicker::new(&device, window.inner_size());
        let zoom_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("mip_level_buffer"),
            contents: bytemuck::cast_slice(&[2u32]),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
        });

        let image_dims_buffer = ImageSize::create_buffer(&device);
        let z_value_range_buffer = ZValueRange::<f32>::create_buffer(&device);
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
            texture_bind_group_layout,
            vertex_buffer: None,
            index_buffer: None,
            texture: None,
            image_dims_buffer,
            z_value_range_buffer,
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
        if let Some(texture) = &self.texture {
            renderpass.set_bind_group(0, &texture.bind_group, &[]);
        }
        renderpass.set_bind_group(1, &self.image_info_bind_group, &[]);
        renderpass.set_bind_group(2, &self.transformation.bind_group, &[]);
        renderpass.set_bind_group(3, &self.projection.bind_group, &[]);
        if let Some(vertex_buffer) = &self.vertex_buffer {
            renderpass.set_vertex_buffer(0, vertex_buffer.buffer.slice(..));
        }
        if let Some(index_buffer) = &self.index_buffer {
            renderpass.set_index_buffer(index_buffer.buffer.slice(..), wgpu::IndexFormat::Uint32);
            renderpass.draw_indexed(
                0..index_buffer.buffer.size() as u32 / std::mem::size_of::<u32>() as u32,
                0,
                0..1,
            );
        }

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
        self.transformation.update_gpu(&self.queue);
        self.projection.update_gpu(&self.queue);
        // Submit the command in the queue to execute
        self.queue.submit([encoder.finish()]);
        self.window.pre_present_notify();
        surface_texture.present();

        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(texture) = &self.texture {
                match pollster::block_on(
                    self.pixel_picker
                        .get(self.device.clone(), texture.surface.image.clone()),
                ) {
                    Ok((x, y, z)) => {
                        log::info!("Pixel at [{}/{}]={:.3}", x, y, z);
                    }
                    Err(e) => {
                        log::error!("Pixel read failed: {}", e);
                    }
                };
            }
        }
    }

    fn set_surface(&mut self, data: Image<f32>) {
        log::info!("Setting new surface image");
        let outlier_removed_data = data.outlier_removed_data(2.0, 98.0);
        let z_range = image::value_range(&outlier_removed_data);
        z_range.write_buffer(&self.queue, &self.z_value_range_buffer);

        data.size.write_buffer(&self.queue, &self.image_dims_buffer);

        self.vertex_buffer = Some(VertexBuffer::new(&data, &self.device));

        self.index_buffer = Some(
            IndexBufferBuilder::new_triangle_strip(&data.size).create_buffer_init(&self.device),
        );

        let texture = Texture::new(&self.device, data, &self.texture_bind_group_layout);
        texture.surface.write_to_queue(&self.queue);
        self.texture = Some(texture);
    }

    fn set_amplitude(&mut self, data: Image<u16>) {
        log::info!("Setting new amplitude image");
        if let Some(texture) = &mut self.texture {
            texture.amplitude.set_image(data);
            texture.amplitude.write_to_queue(&self.queue);
        }
    }

    fn get_pixel_value(
        &mut self,
        sender: futures::channel::oneshot::Sender<
            Shared<std::pin::Pin<Box<dyn std::future::Future<Output = PixelResult>>>>,
        >,
    ) {
        if let Some(texture) = &self.texture {
            self.pixel_picker.write_to_channel(
                self.device.clone(),
                texture.surface.image.clone(),
                sender,
            );
        } else {
            let future: std::pin::Pin<Box<dyn std::future::Future<Output = PixelResult>>> =
                Box::pin(async move {
                    Err::<(u32, u32, f32), Arc<anyhow::Error>>(Arc::new(anyhow!(
                        "Surface not initialized"
                    )))
                });
            if let Err(_) = sender.send(future.shared()) {
                log::error!("Failed to return error message");
            }
        }
    }

    fn set_amplitude_shader(&mut self) {
        log::info!("Setting amplitude shader");
        self.use_height_shader = false;
    }

    fn set_height_shader(&mut self) {
        log::info!("Setting height shader");
        self.use_height_shader = true;
    }

    fn set_overlays(&mut self, overlays: Arc<Vec<Overlay>>) {
        log::info!("Setting overlays");
        if let Some(texture) = &mut self.texture {
            texture.overlay.set_overlays(overlays);
            texture.overlay.write_to_queue(&self.queue);
        }
    }

    fn clear_overlays(&mut self) {
        log::info!("Clearing overlays");
        if let Some(texture) = &mut self.texture {
            texture.overlay.set_overlays(Arc::new(Vec::new()));
            texture.overlay.write_to_queue(&self.queue);
        }
    }

    fn back_to_origin(&mut self) {
        self.projection.reset();
        self.transformation.reset();
    }
}

struct ImageViewer3D {
    #[cfg(target_arch = "wasm32")]
    proxy: Option<winit::event_loop::EventLoopProxy<ViewerCommand>>,
    state: Option<State>,
}

impl ImageViewer3D {
    pub fn new(#[cfg(target_arch = "wasm32")] event_loop: &EventLoop<ViewerCommand>) -> Self {
        #[cfg(target_arch = "wasm32")]
        let proxy = Some(event_loop.create_proxy());
        Self {
            state: None,
            #[cfg(target_arch = "wasm32")]
            proxy,
        }
    }
}

impl ApplicationHandler<ViewerCommand> for ImageViewer3D {
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
                    assert!(
                        proxy
                            .send_event(ViewerCommand::SetState(State::new(window).await))
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
                            if let Some(texture) = &mut app_state.texture {
                                if texture.overlay.overlays.is_empty() {
                                    app_state.set_overlays(Arc::new(texture::example_overlays()));
                                } else {
                                    app_state.clear_overlays();
                                }
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
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, mut event: ViewerCommand) {
        match event {
            ViewerCommand::GetPixel(sender) => {
                if let Some(app_state) = self.state.as_mut() {
                    app_state.get_pixel_value(sender);
                }
            }
            ViewerCommand::SetAmplitudeShader => {
                if let Some(app_state) = self.state.as_mut() {
                    app_state.set_amplitude_shader();
                }
            }
            ViewerCommand::SetHeightShader => {
                if let Some(app_state) = self.state.as_mut() {
                    app_state.set_height_shader();
                }
            }
            ViewerCommand::SetOverlays(overlays) => {
                if let Some(app_state) = self.state.as_mut() {
                    app_state.set_overlays(overlays.clone());
                }
            }
            ViewerCommand::ClearOverlays => {
                if let Some(app_state) = self.state.as_mut() {
                    app_state.clear_overlays();
                }
            }
            ViewerCommand::BackToOrigin => {
                if let Some(app_state) = self.state.as_mut() {
                    app_state.back_to_origin();
                }
            }
            ViewerCommand::SetSurface(data) => {
                if let Some(app_state) = self.state.as_mut() {
                    app_state.set_surface(data);
                } else {
                    log::warn!("State is None, cannot set surface");
                }
            }
            ViewerCommand::SetAmplitude(data) => {
                if let Some(app_state) = self.state.as_mut() {
                    app_state.set_amplitude(data);
                }
            }
            ViewerCommand::SetState(mut state) => {
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
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_secs()
        .init();

    let image = SurfaceAmplitudeImage::from_file("example-img.tiff").unwrap();
    let event_loop = EventLoop::with_user_event().build()?;
    let proxy = event_loop.create_proxy();
    proxy
        .send_event(ViewerCommand::SetSurface(image.surface))
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
