use glam::{Mat4, Vec2, Vec3};
use log::error;
use std::{borrow::Cow, sync::Arc, vec};
use wgpu::util::DeviceExt;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

mod image;
mod keyboard;
mod mouse;
mod projection;
mod texture;
mod transformation;
use image::SurfaceAmplitudeImage;
use mouse::Mouse;
use projection::Projection;

use crate::{
    keyboard::Keyboard,
    projection::ProjectionBuffer,
    transformation::{Transformation, TransformationBuffer},
};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 1],
    vertex_id: [u32; 1],
}

impl Vertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<f32>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Uint32,
                },
            ],
        }
    }
}

struct ImageSize {
    width: u32,
    height: u32,
}

impl ImageSize {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<ImageSize>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Uint32,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<u32>() as wgpu::BufferAddress,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Uint32,
                },
            ],
        }
    }
}

struct ZValueRange {
    min: f32,
    max: f32,
}

impl ZValueRange {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<ZValueRange>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float32,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<f32>() as wgpu::BufferAddress,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32,
                },
            ],
        }
    }
}

struct State {
    window: Arc<Window>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    size: winit::dpi::PhysicalSize<u32>,
    aspect_ratio: f32,
    surface: wgpu::Surface<'static>,
    surface_format: wgpu::TextureFormat,
    current_transformation: Mat4,
    current_projection: Mat4,
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    image_size_buffer: wgpu::Buffer,
    z_value_range_buffer: wgpu::Buffer,
    texture_bind_group: wgpu::BindGroup,
    depth_view: wgpu::TextureView,
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

        let size = window.inner_size();

        let surface = instance.create_surface(window.clone()).unwrap();
        let cap = surface.get_capabilities(&adapter);
        let surface_format = cap.formats[0];

        // Create shader module
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader.wgsl"))),
        });

        let image = SurfaceAmplitudeImage::from_file("img.tiff").unwrap();
        let image_surface = image.surface;

        let texture = texture::Texture::new(image.amplitude, &device);
        texture.write_to_queue(&queue);

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("render_pipeline_layout"),
                bind_group_layouts: &[&texture.texture_bind_group_layout],
                push_constant_ranges: &[],
            });

        const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[
                    Vertex::desc(),
                    ImageSize::desc(),
                    ZValueRange::desc(),
                    TransformationBuffer::desc(),
                    ProjectionBuffer::desc(),
                ],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(surface_format.into())],
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
        });

        // Interleave z values and vertex indices into a single vertex buffer
        let mut vertices: Vec<Vertex> =
            Vec::with_capacity((image_surface.width * image_surface.height) as usize);
        for (i, &z) in image_surface
            .outlier_removed_data(10.0, 90.0)
            .iter()
            .enumerate()
        {
            vertices.push(Vertex {
                position: [z],
                vertex_id: [i as u32],
            });
        }
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer (z + index)"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let mut indices: Vec<u32> = Vec::new();
        for i in 0..image_surface.height - 1 {
            for j in 0..((image_surface.width - 1) / 2) {
                let j = j * 2;
                indices.push((i * image_surface.width + j) as u32);
                indices.push(((i + 1) * image_surface.width + j) as u32);
                indices.push((i * image_surface.width + j + 1) as u32);
                indices.push(((i + 1) * image_surface.width + j + 1) as u32);
                indices.push((i * image_surface.width + j + 2) as u32);
                indices.push(((i + 1) * image_surface.width + j + 2) as u32);
            }
        }

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let image_size_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Image Size Buffer"),
            contents: bytemuck::cast_slice(&[image_surface.width, image_surface.height]),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        let z_range = image_surface.value_range();

        let z_value_range_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Z Value Range Buffer"),
            contents: bytemuck::cast_slice(&[z_range.start, z_range.end]),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        // Create depth texture view
        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("depth_texture"),
            size: wgpu::Extent3d {
                width: size.width.max(1),
                height: size.height.max(1),
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
            size,
            aspect_ratio: size.width as f32 / size.height as f32,
            surface,
            surface_format,
            current_transformation: Mat4::IDENTITY,
            current_projection: Mat4::IDENTITY,
            render_pipeline,
            vertex_buffer,
            index_buffer,
            image_size_buffer,
            z_value_range_buffer,
            texture_bind_group: texture.texture_bind_group,
            depth_view,
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
            width: self.size.width,
            height: self.size.height,
            desired_maximum_frame_latency: 2,
            present_mode: wgpu::PresentMode::AutoVsync,
        };
        self.surface.configure(&self.device, &surface_config);
        // Recreate depth texture to match the new size
        let depth_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("depth_texture"),
            size: wgpu::Extent3d {
                width: self.size.width.max(1),
                height: self.size.height.max(1),
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
        self.size = new_size;
        self.aspect_ratio = new_size.width as f32 / new_size.height as f32;
        self.configure_surface();
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

        // Create transformation buffer (this changes per frame based on mouse input)
        let transformation_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Transformation Buffer"),
            size: (std::mem::size_of::<[[f32; 4]; 4]>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let projection_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Projection Buffer"),
            size: (std::mem::size_of::<[[f32; 4]; 4]>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = self.device.create_command_encoder(&Default::default());
        // Create the renderpass which will clear the screen.
        let mut renderpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &texture_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
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
        renderpass.set_pipeline(&self.render_pipeline);
        renderpass.set_bind_group(0, &self.texture_bind_group, &[]);
        renderpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        renderpass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        renderpass.set_vertex_buffer(1, self.image_size_buffer.slice(..));
        renderpass.set_vertex_buffer(2, self.z_value_range_buffer.slice(..));
        renderpass.set_vertex_buffer(3, transformation_buffer.slice(..));
        renderpass.set_vertex_buffer(4, projection_buffer.slice(..));
        renderpass.draw_indexed(
            0..self.index_buffer.size() as u32 / std::mem::size_of::<u32>() as u32,
            0,
            0..1,
        );

        // End the renderpass.
        drop(renderpass);
        self.queue.write_buffer(
            &transformation_buffer,
            0,
            bytemuck::cast_slice(&self.current_transformation.to_cols_array_2d()),
        );
        self.queue.write_buffer(
            &projection_buffer,
            0,
            bytemuck::cast_slice(&self.current_projection.to_cols_array_2d()),
        );
        // Submit the command in the queue to execute
        self.queue.submit([encoder.finish()]);
        self.window.pre_present_notify();
        surface_texture.present();
    }
}

#[derive(Default)]
struct App {
    state: Option<State>,
    mouse: Mouse,
    keyboard: Keyboard,
    transformation: Transformation,
    projection: Projection,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Create window object
        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes())
                .unwrap(),
        );

        let state = pollster::block_on(State::new(window.clone()));
        self.state = Some(state);
        window.request_redraw();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let app_state = self.state.as_mut().unwrap();
        match event {
            WindowEvent::CloseRequested => {
                println!("The close button was pressed; stopping");
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                app_state.current_transformation = self.transformation.get_current();
                app_state.current_projection = self.projection.get_current();
                app_state.render();
            }
            WindowEvent::Resized(size) => {
                app_state.resize(size);
                self.projection.update_aspect_ratio(app_state.aspect_ratio);
            }
            WindowEvent::CursorMoved {
                device_id: _,
                position,
            } => {
                self.mouse.register_move_event(position);
                if self.mouse.is_left_button_pressed() {
                    match self.mouse.get_device_coordinates(app_state.size) {
                        Ok(new_position) => {
                            if self.mouse.is_pointer_inside(Vec2::from(new_position)) {
                                if self.keyboard.is_control_pressed() {
                                    self.projection.change_position(new_position);
                                } else {
                                    self.transformation.rotate(Vec3::from((new_position, 1.0)));
                                }
                            }
                            app_state.get_window().request_redraw();
                        }
                        Err(e) => error!("Failed to calculate pointer position: {}", e),
                    }
                }
            }
            WindowEvent::MouseInput {
                device_id: _,
                state,
                button,
            } => {
                self.mouse.register_button_event(button, state);
                if self.mouse.is_left_button_pressed() {
                    match self.mouse.get_device_coordinates(app_state.size) {
                        Ok(pos) => {
                            if self.keyboard.is_control_pressed() {
                                self.projection.start_move(pos);
                            } else {
                                self.transformation.start_move(Vec3::from((pos, 1.0)))
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
                self.mouse.register_scroll_event(delta);
                self.projection.zoom(self.mouse.get_zoom());
                app_state.get_window().request_redraw();
            }
            WindowEvent::KeyboardInput {
                device_id: _,
                event,
                is_synthetic: _,
            } => self.keyboard.register_event(event),
            _ => (),
        }
    }
}

fn main() {
    // wgpu uses `log` for all of our logging, so we initialize a logger with the `env_logger` crate.
    //
    // To change the log level, set the `RUST_LOG` environment variable. See the `env_logger`
    // documentation for more information.
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_secs()
        .init();

    let event_loop = EventLoop::new().unwrap();

    // When the current loop iteration finishes, immediately begin a new
    // iteration regardless of whether or not new events are available to
    // process. Preferred for applications that want to render as fast as
    // possible, like games.
    event_loop.set_control_flow(ControlFlow::Poll);

    // When the current loop iteration finishes, suspend the thread until
    // another event arrives. Helps keeping CPU utilization low if nothing
    // is happening, which is preferred if the application might be idling in
    // the background.
    // event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = App::default();
    event_loop.run_app(&mut app).unwrap();
}
