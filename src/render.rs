use std::{borrow::Cow, sync::Arc};

use wgpu::{BindGroupLayout, Surface, TextureFormat};
use winit::{dpi::PhysicalSize, window::Window};

use crate::{
    gpu_data::{
        DataSize, pixel_picker::PixelPicker, texture_image_range::TextureImageRangeBuffer,
        topology_percentile_range::TopologyPercentileRangeBuffer,
    },
    interaction::Interaction,
    mip::Mip,
    scene::Scene,
    vertex_buffer::VertexBuffer,
};

const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

pub(crate) struct Renderer {
    surface: wgpu::Surface<'static>,
    surface_format: wgpu::TextureFormat,
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    render_pipeline_texture: wgpu::RenderPipeline,
    render_pipeline_height: wgpu::RenderPipeline,
    depth_view: wgpu::TextureView,
    image_info_bind_group: wgpu::BindGroup,
}

impl Renderer {
    pub(crate) fn new(
        window: &Window,
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        texture_bind_group_layout: &BindGroupLayout,
        surface: Surface<'static>,
        surface_format: TextureFormat,
        interaction: &Interaction,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader.wgsl"))),
        });

        let image_info_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("image_info_bind_group_layout"),
                entries: &[
                    DataSize::get_bind_group_layout_entry(),
                    TopologyPercentileRangeBuffer::get_bind_group_layout_entry(),
                    TextureImageRangeBuffer::get_bind_group_layout_entry(),
                    Mip::get_bind_group_layout_entry(),
                ],
            });
        let image_info_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("image_info_bind_group"),
            layout: &image_info_bind_group_layout,
            entries: &[
                DataSize::get_bind_group_entry(&interaction.mip.image_dims_buffer),
                interaction.percentile_range_buffer.get_bind_group_entry(),
                interaction.texture_range_buffer.get_bind_group_entry(),
                interaction.mip.get_bind_group_entry(),
            ],
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("render_pipeline_layout"),
                bind_group_layouts: &[
                    &texture_bind_group_layout,
                    &image_info_bind_group_layout,
                    &interaction.transformation.bind_group_layout,
                    &interaction.projection.bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

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
        let mut this = Self {
            surface,
            surface_format,
            device,
            queue,
            render_pipeline_texture,
            render_pipeline_height,
            image_info_bind_group,
            depth_view,
        };

        // Configure surface for the first time
        this.configure_surface(window.inner_size());
        this
    }

    pub(crate) fn configure_surface(&mut self, window_size: PhysicalSize<u32>) {
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            format: self.surface_format,
            // Request compatibility with the sRGB-format texture view we‘re going to create later.
            view_formats: vec![self.surface_format.add_srgb_suffix()],
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            width: window_size.width,
            height: window_size.height,
            desired_maximum_frame_latency: 2,
            present_mode: wgpu::PresentMode::AutoVsync,
        };
        self.surface.configure(&self.device, &surface_config);
        // Recreate depth texture to match the new size
        let depth_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("depth_texture"),
            size: wgpu::Extent3d {
                width: window_size.width.max(1),
                height: window_size.height.max(1),
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

    pub(crate) fn render(&self, window: Arc<Window>, interaction: &Interaction, scene: &Scene) {
        let surface_texture = self.acquire_surface_texture_phase();
        let surface_view = self.create_surface_view_phase(&surface_texture);
        let mut encoder = self.device.create_command_encoder(&Default::default());

        self.encode_scene_phase(&mut encoder, &surface_view, interaction, scene);
        self.encode_post_process_phase(&mut encoder, &surface_texture, interaction);
        self.submit_and_present_phase(window, encoder, surface_texture);

        #[cfg(not(target_arch = "wasm32"))]
        self.log_hover_pixel_phase(interaction, scene);
    }

    fn acquire_surface_texture_phase(&self) -> wgpu::SurfaceTexture {
        self.surface
            .get_current_texture()
            .expect("failed to acquire next swapchain texture")
    }

    fn create_surface_view_phase(
        &self,
        surface_texture: &wgpu::SurfaceTexture,
    ) -> wgpu::TextureView {
        surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor {
                // Without add_srgb_suffix() the image we will be working with
                // might not be "gamma correct".
                format: Some(self.surface_format.add_srgb_suffix()),
                ..Default::default()
            })
    }

    fn encode_scene_phase(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        surface_view: &wgpu::TextureView,
        interaction: &Interaction,
        scene: &Scene,
    ) {
        // Two color attachments: main color + picking texture
        let mut renderpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[
                Some(wgpu::RenderPassColorAttachment {
                    view: surface_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                }),
                Some(wgpu::RenderPassColorAttachment {
                    view: &interaction.pixel_picker.picking_texture_view,
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

        let pipeline = if interaction.use_height_shader() {
            &self.render_pipeline_height
        } else {
            &self.render_pipeline_texture
        };
        renderpass.set_pipeline(pipeline);
        renderpass.set_bind_group(0, scene.get_bind_group(), &[]);
        renderpass.set_bind_group(1, &self.image_info_bind_group, &[]);
        renderpass.set_bind_group(2, &interaction.transformation.bind_group, &[]);
        renderpass.set_bind_group(3, &interaction.projection.bind_group, &[]);

        interaction.mip.update_gpu(&mut renderpass, &self.queue);
    }

    fn encode_post_process_phase(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        surface_texture: &wgpu::SurfaceTexture,
        interaction: &Interaction,
    ) {
        interaction.pixel_picker.copy_pixel_at_mouse(encoder);
        interaction.update_gpu(&self.queue, encoder, surface_texture);
    }

    fn submit_and_present_phase(
        &self,
        window: Arc<Window>,
        encoder: wgpu::CommandEncoder,
        surface_texture: wgpu::SurfaceTexture,
    ) {
        self.queue.submit([encoder.finish()]);
        window.pre_present_notify();
        surface_texture.present();
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn log_hover_pixel_phase(&self, interaction: &Interaction, scene: &Scene) {
        if let Some(texture_image) = scene.get_texture_image() {
            match pollster::block_on(interaction.pixel_picker.get(
                self.device.clone(),
                scene.get_topology_image(),
                texture_image,
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
    }
}
