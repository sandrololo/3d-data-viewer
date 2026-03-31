use imbuf::Image;
use std::sync::Arc;
use wgpu::{BindGroup, Queue};

pub(crate) use crate::scene::{overlay::*, surface::*, texture_image::*};

mod overlay;
mod surface;
mod texture_image;

pub(crate) struct Scene {
    overlay: OverlayTexture,
    surface: SurfaceTexture,
    texture: TextureImage,
    bind_group: wgpu::BindGroup,
}

impl Scene {
    pub(crate) fn new_surface(
        surface: Image<f32, 1>,
        device: &wgpu::Device,
        queue: &Queue,
        layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let overlay_texture = OverlayTexture::new(&surface.dimensions().into(), &device);
        let texture = TextureImage::new(&surface.dimensions().into(), &device);
        let surface_data = SurfaceTexture::new(Arc::new(surface), &device);
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("texture_bind_group"),
            layout: layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&surface_data.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&overlay_texture.view),
                },
            ],
        });
        surface_data.write_to_queue(queue);
        Self {
            overlay: overlay_texture,
            surface: surface_data,
            texture,
            bind_group: group,
        }
    }

    pub(crate) fn get_surface_image(&self) -> Arc<Image<f32, 1>> {
        self.surface.image.clone()
    }

    pub(crate) fn set_texture(&mut self, data: Image<u16, 1>, queue: &Queue) {
        self.texture.set_image(data);
        self.texture.write_to_queue(queue);
    }

    pub(crate) fn get_texture_image(&self) -> Option<Arc<Image<u16, 1>>> {
        self.texture.image.clone()
    }

    pub(crate) fn set_overlays(&mut self, overlays: Arc<Vec<Overlay>>, queue: &Queue) {
        self.overlay.set_overlays(overlays);
        self.overlay.write_to_queue(queue);
    }

    pub(crate) fn clear_overlays(&mut self, queue: &Queue) {
        self.overlay.set_overlays(Arc::new(Vec::new()));
        self.overlay.write_to_queue(queue);
    }

    pub(crate) fn get_bind_group(&self) -> &BindGroup {
        &self.bind_group
    }

    pub(crate) fn create_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("texture_bind_group_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Uint,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    },
                    count: None,
                },
            ],
        })
    }
}
