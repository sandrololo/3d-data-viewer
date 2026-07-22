use std::{num::NonZeroU32, sync::Arc};

use wgpu::{BindGroupLayout, TextureFormat};

use crate::{
    gpu_data::{
        Capture, DataSize, pixel_picker::PixelPicker, texture_image_range::TextureImageRangeBuffer,
        topology_percentile_range::TopologyPercentileRangeBuffer,
    },
    interaction::Interaction,
    render::{axes::Axes, depth_buffer::DepthBuffer, pipeline::Pipeline},
    scene::Scene,
};

pub mod axes;
mod depth_buffer;
pub mod font_atlas;
pub mod pipeline;

/// Renders the 3D scene into a caller-provided offscreen color target (plus an
/// internal picking texture + depth buffer). No swapchain/surface is owned here —
/// eframe owns the WebGPU surface; the produced color texture is sampled by egui.
pub struct Renderer {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    pipeline: Pipeline,
    axes: Axes,
    axes_visible: bool,
    depth_buffer: DepthBuffer,
    image_info_bind_group: wgpu::BindGroup,
    size: (u32, u32),
}

impl Renderer {
    pub fn new(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        texture_bind_group_layout: &BindGroupLayout,
        target_format: TextureFormat,
        interaction: &Interaction,
        percentile_range_buffer: &TopologyPercentileRangeBuffer,
        texture_range_buffer: &TextureImageRangeBuffer,
        size: (u32, u32),
    ) -> Self {
        let image_info_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("image_info_bind_group_layout"),
                entries: &[
                    uniform_buffer_layout_entry(0),
                    uniform_buffer_layout_entry(1),
                    uniform_buffer_layout_entry(2),
                    uniform_buffer_layout_entry(3),
                ],
            });
        let image_info_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("image_info_bind_group"),
            layout: &image_info_bind_group_layout,
            entries: &[
                DataSize::get_bind_group_entry(&interaction.mip.image_dims_buffer),
                percentile_range_buffer.get_bind_group_entry(),
                texture_range_buffer.get_bind_group_entry(),
                interaction.mip.get_bind_group_entry(),
            ],
        });

        let pipeline = Pipeline::new(
            &device,
            target_format,
            texture_bind_group_layout,
            &image_info_bind_group_layout,
            interaction,
        );
        let axes = Axes::new(&device, &queue, target_format, interaction);
        let depth_buffer = DepthBuffer::new(&device, size);

        let mut this = Self {
            device,
            queue,
            pipeline,
            axes,
            axes_visible: true,
            image_info_bind_group,
            depth_buffer,
            size,
        };
        this.axes.update_screen_size(&this.queue, size.0, size.1);
        this
    }

    pub fn resize(&mut self, size: (u32, u32)) {
        if self.size != size && size.0 > 0 && size.1 > 0 {
            self.size = size;
            self.depth_buffer = DepthBuffer::new(&self.device, size);
            self.axes.update_screen_size(&self.queue, size.0, size.1);
        }
    }

    pub fn display_grid(&mut self, visible: bool) {
        self.axes_visible = visible;
    }

    pub fn update_axes_origin(
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

    pub fn update_z_range(&mut self, z_range: (f32, f32)) {
        self.axes.update_z_range(&self.device, z_range);
    }

    pub fn render(
        &mut self,
        color_view: &wgpu::TextureView,
        color_texture: &wgpu::Texture,
        interaction: &Interaction,
        pixel_picker: &PixelPicker,
        image_capture: &Capture,
        scene: &Scene,
    ) {
        // Rebuild labels with view-dependent density before drawing.
        if self.axes_visible {
            let mvp =
                interaction.projection.get_current() * interaction.transformation.get_current();
            self.axes.update_labels(&self.device, &self.queue, mvp);
        }

        let mut encoder = self.device.create_command_encoder(&Default::default());

        self.encode_scene_phase(&mut encoder, color_view, interaction, pixel_picker, scene);
        self.encode_post_process_phase(
            &mut encoder,
            color_texture,
            interaction,
            pixel_picker,
            image_capture,
        );

        self.queue.submit([encoder.finish()]);
    }

    fn encode_scene_phase(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        color_view: &wgpu::TextureView,
        interaction: &Interaction,
        pixel_picker: &PixelPicker,
        scene: &Scene,
    ) {
        let mut renderpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[
                Some(wgpu::RenderPassColorAttachment {
                    view: color_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                }),
                Some(wgpu::RenderPassColorAttachment {
                    view: &pixel_picker.picking_texture_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        // Out-of-bounds sentinel (same as the axes shaders), so
                        // background picks don't read as pixel (0, 0).
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: u32::MAX as f64,
                            g: u32::MAX as f64,
                            b: 0.0,
                            a: 0.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                }),
            ],
            depth_stencil_attachment: Some(
                self.depth_buffer.renderpass_depth_stencil_attachement(),
            ),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        let pipeline = self.pipeline.get(interaction.get_fragment_shader_variant());
        renderpass.set_pipeline(pipeline);

        renderpass.set_bind_group(0, scene.get_bind_group(), &[]);
        renderpass.set_bind_group(1, &self.image_info_bind_group, &[]);
        renderpass.set_bind_group(2, &interaction.transformation.bind_group, &[]);
        renderpass.set_bind_group(3, &interaction.projection.bind_group, &[]);

        interaction.mip.update_gpu(&mut renderpass, &self.queue);

        if self.axes_visible {
            self.axes.draw(&mut renderpass, interaction);
        }
    }

    fn encode_post_process_phase(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        color_texture: &wgpu::Texture,
        interaction: &Interaction,
        pixel_picker: &PixelPicker,
        image_capture: &Capture,
    ) {
        pixel_picker.copy_pixel_at_mouse(encoder);
        image_capture.copy_texture(encoder, color_texture);
        interaction.update_gpu(&self.queue);
    }
}

pub fn uniform_buffer_layout_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}
