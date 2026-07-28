use imbuf::Image;
use std::sync::Arc;
use wgpu::{BindGroup, Queue};

pub use crate::scene::{overlay::*, texture_image::*, topology::*};

mod overlay;
mod texture_image;
mod topology;

pub struct Scene {
    overlay: OverlayTexture,
    topology: TopologyTexture,
    texture: TextureImage,
    bind_group: wgpu::BindGroup,
}

impl Scene {
    pub fn new_topology(
        topology: Image<f32, 1>,
        device: &wgpu::Device,
        queue: &Queue,
        layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let overlay_texture = OverlayTexture::new(&topology.dimensions().into(), device);
        let texture = TextureImage::new(&topology.dimensions().into(), device);
        let topology_data = TopologyTexture::new(Arc::new(topology), device);
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("texture_bind_group"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&topology_data.view),
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
        topology_data.write_to_queue(queue);
        Self {
            overlay: overlay_texture,
            topology: topology_data,
            texture,
            bind_group: group,
        }
    }

    pub fn get_topology_image(&self) -> Arc<Image<f32, 1>> {
        self.topology.image.clone()
    }

    pub fn set_texture(&mut self, data: Image<u16, 1>, queue: &Queue) {
        self.texture.set_image(data);
        self.texture.write_to_queue(queue);
    }

    pub fn get_texture_image(&self) -> Option<Arc<Image<u16, 1>>> {
        self.texture.image.clone()
    }

    pub fn set_overlays(&mut self, overlays: Arc<Vec<Region>>, queue: &Queue) {
        self.overlay.set_overlays(overlays);
        self.overlay.write_to_queue(queue);
    }

    pub fn clear_overlays(&mut self, queue: &Queue) {
        self.overlay.set_overlays(Arc::new(Vec::new()));
        self.overlay.write_to_queue(queue);
    }

    pub fn get_bind_group(&self) -> &BindGroup {
        &self.bind_group
    }

    pub fn create_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
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
