use std::{num::NonZeroU32, sync::Arc};

use wgpu::{BindGroupLayout, Surface, TextureFormat};
use winit::{dpi::PhysicalSize, window::Window};

use crate::{
    gpu_data::{
        DataSize, texture_image_range::TextureImageRangeBuffer,
        topology_percentile_range::TopologyPercentileRangeBuffer,
    },
    interaction::Interaction,
    mip::Mip,
    render::{axes::Axes, depth_buffer::DepthBuffer, pipeline::Pipeline},
    scene::Scene,
};

pub(crate) mod axes;
mod depth_buffer;
pub(crate) mod font_atlas;
pub(crate) mod pipeline;

pub(crate) struct Renderer {
    surface: wgpu::Surface<'static>,
    surface_format: wgpu::TextureFormat,
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    pipeline: Pipeline,
    axes: Axes,
    axes_visible: bool,
    depth_buffer: DepthBuffer,
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

        let pipeline = Pipeline::new(
            &device,
            surface_format,
            texture_bind_group_layout,
            &image_info_bind_group_layout,
            interaction,
        );
        let axes = Axes::new(&device, &queue, surface_format, interaction);
        let depth_buffer = DepthBuffer::new(window.inner_size(), &device);

        let mut this = Self {
            surface,
            surface_format,
            device,
            queue,
            pipeline,
            axes,
            axes_visible: true,
            image_info_bind_group,
            depth_buffer,
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
        self.depth_buffer = DepthBuffer::new(window_size, &self.device);
        self.axes
            .update_screen_size(&self.queue, window_size.width, window_size.height);
    }

    pub(crate) fn display_grid(&mut self, visible: bool) {
        self.axes_visible = visible;
    }

    pub(crate) fn update_axes_origin(
        &mut self,
        image_size: (NonZeroU32, NonZeroU32),
        z_range: (f32, f32),
    ) {
        self.axes.update_grid(
            &self.device,
            image_size.0.get(),
            image_size.1.get(),
            z_range,
        );
    }

    pub(crate) fn update_z_range(&mut self, z_range: (f32, f32)) {
        self.axes.update_z_range(&self.device, z_range);
    }

    pub(crate) fn render(&mut self, window: Arc<Window>, interaction: &Interaction, scene: &Scene) {
        // Rebuild labels with view-dependent density before drawing.
        if self.axes_visible {
            let mvp =
                interaction.projection.get_current() * interaction.transformation.get_current();
            self.axes.update_labels(&self.device, mvp);
        }

        let surface_texture = self
            .surface
            .get_current_texture()
            .expect("failed to acquire next swapchain texture");
        let surface_view = self.create_surface_view_phase(&surface_texture);
        let mut encoder = self.device.create_command_encoder(&Default::default());

        self.encode_scene_phase(&mut encoder, &surface_view, interaction, scene);
        self.encode_post_process_phase(&mut encoder, &surface_texture, interaction);

        self.queue.submit([encoder.finish()]);
        window.pre_present_notify();
        surface_texture.present();

        #[cfg(not(target_arch = "wasm32"))]
        self.log_hover_pixel_phase(interaction, scene);
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
            depth_stencil_attachment: Some(
                self.depth_buffer.renderpass_depth_stencil_attachement(),
            ),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        let pipeline = self.pipeline.get(interaction.get_fragment_shader_variant());
        renderpass.set_pipeline(pipeline);

        renderpass.set_bind_group(0, scene.get_bind_group(), &[]);
        renderpass.set_bind_group(1, &self.image_info_bind_group, &[]);
        renderpass.set_bind_group(2, &interaction.transformation.bind_group, &[]);
        renderpass.set_bind_group(3, &interaction.projection.bind_group, &[]);

        interaction.mip.update_gpu(&mut renderpass, &self.queue);

        // Draw coordinate grid on top of the scene
        if self.axes_visible {
            self.axes.draw(&mut renderpass, interaction);
        }
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
