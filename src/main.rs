use glam::{Vec2, Vec3};
use log::error;
use std::{borrow::Cow, sync::Arc, vec};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

mod amplitude_texture;
mod image;
mod index_buffer;
mod keyboard;
mod mouse;
mod overlay;
mod projection;
mod texture;
mod transformation;
mod vertex_buffer;
use image::SurfaceAmplitudeImage;
use mouse::Mouse;
use projection::Projection;

use crate::{
    index_buffer::{IndexBuffer, IndexBufferBuilder},
    keyboard::Keyboard,
    overlay::{Overlay, OverlayTexture},
    texture::Texture,
    transformation::Transformation,
    vertex_buffer::VertexBuffer,
};

struct State {
    window: Arc<Window>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    size: winit::dpi::PhysicalSize<u32>,
    aspect_ratio: f32,
    surface: wgpu::Surface<'static>,
    surface_format: wgpu::TextureFormat,
    transformation: Transformation,
    projection: Projection,
    render_pipeline_amplitude: wgpu::RenderPipeline,
    render_pipeline_height: wgpu::RenderPipeline,
    use_height_shader: bool,
    vertex_buffer: VertexBuffer,
    index_buffer: IndexBuffer,
    texture_bind_group: wgpu::BindGroup,
    image_info_bind_group: wgpu::BindGroup,
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

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader.wgsl"))),
        });

        let image = SurfaceAmplitudeImage::from_file("img.tiff").unwrap();

        let amplitude_texture = amplitude_texture::AmplitudeTexture::new(image.amplitude, &device);
        amplitude_texture.write_to_queue(&queue);

        let example_overlays = vec![
            Overlay {
                pixels: vec![0..100, 1024..1124, 2048..2148, 3072..3172, 4096..4196],
                color: [255, 0, 0, 100],
            },
            Overlay {
                pixels: vec![5000..50000],
                color: [255, 255, 0, 100],
            },
        ];

        let overlay_texture = OverlayTexture::new(&image.surface.size, &example_overlays, &device);
        overlay_texture.write_to_queue(&queue);

        let texture = Texture::new(amplitude_texture, overlay_texture);
        let (texture_bind_group_layout, texture_bind_group) = texture.create_bind_group(&device);

        let outlier_removed_data = image.surface.outlier_removed_data(5.0, 95.0);
        let z_range = image::value_range(&outlier_removed_data);

        // Combined bind group: image dimensions (binding 0) + z range (binding 1) in group 1
        let image_dims_buffer = image.surface.size.create_buffer_init(&device);
        let z_value_range_buffer = z_range.create_buffer_init(&device);
        let image_info_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("image_info_bind_group_layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
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
        let image_info_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("image_info_bind_group"),
            layout: &image_info_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: image_dims_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: z_value_range_buffer.as_entire_binding(),
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

        let texture_format = [Some(surface_format.into())];
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
                targets: &texture_format,
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
            targets: &texture_format,
        });
        let render_pipeline_height = device.create_render_pipeline(&height_pipeline_descriptor);

        let vertex_buffer = VertexBuffer::new(&image.surface.size, &outlier_removed_data, &device);
        let index_buffer =
            IndexBufferBuilder::new_triangle_strip(&image.surface.size).create_buffer_init(&device);

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
            transformation,
            projection,
            render_pipeline_amplitude,
            render_pipeline_height,
            use_height_shader: true,
            vertex_buffer,
            index_buffer,
            texture_bind_group,
            image_info_bind_group,
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
        let pipeline = if self.use_height_shader {
            &self.render_pipeline_height
        } else {
            &self.render_pipeline_amplitude
        };
        renderpass.set_pipeline(pipeline);
        renderpass.set_bind_group(0, &self.texture_bind_group, &[]);
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
        self.transformation.update_gpu(&self.queue);
        self.projection.update_gpu(&self.queue);
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
                app_state.render();
            }
            WindowEvent::Resized(size) => {
                app_state.resize(size);
                app_state
                    .projection
                    .update_aspect_ratio(app_state.aspect_ratio);
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
                                    app_state.projection.change_position(new_position);
                                } else {
                                    app_state
                                        .transformation
                                        .rotate(Vec3::from((new_position, 1.0)));
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
                self.mouse.register_scroll_event(delta);
                app_state.projection.zoom(self.mouse.get_zoom());
                app_state.get_window().request_redraw();
            }
            WindowEvent::KeyboardInput {
                device_id: _,
                event,
                is_synthetic: _,
            } => {
                self.keyboard.register_event(event.clone());
                if let winit::keyboard::Key::Character(ref c) = event.logical_key {
                    // Toggle shader with 'S' key
                    if c.as_str() == "s" && event.state == winit::event::ElementState::Pressed {
                        app_state.use_height_shader = !app_state.use_height_shader;
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
