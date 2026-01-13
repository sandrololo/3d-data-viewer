use std::{collections::HashMap, num::NonZeroU32};

use wgpu::util::DeviceExt;

use crate::image::ImageSize;

pub(crate) struct IndexBufferBuilder {
    mip_level_indices: HashMap<u32, Vec<u32>>,
}

impl IndexBufferBuilder {
    pub(crate) fn new_triangle_strip(image_size: &ImageSize, mip_levels: u32) -> Self {
        let mut mip_level_indices: HashMap<u32, Vec<u32>> = HashMap::new();
        for mip_level in 0..mip_levels {
            let triangle_strip = triangle_strip(&ImageSize {
                width: NonZeroU32::new(image_size.width.get() / 2u32.pow(mip_level as u32))
                    .expect("Can't be zero"),
                height: NonZeroU32::new(image_size.height.get() / 2u32.pow(mip_level as u32))
                    .expect("Can't be zero"),
            });
            log::info!(
                "MIP level {:?} index buffer length: {:?}",
                mip_level,
                triangle_strip.len()
            );
            log::info!("Number of triangles: {:?}", triangle_strip.len() - 2);
            mip_level_indices.insert(mip_level, triangle_strip);
        }
        Self { mip_level_indices }
    }

    pub(crate) fn create_buffer(&self, device: &wgpu::Device) -> IndexBuffer {
        let mut mip_level_buffers: HashMap<u32, wgpu::Buffer> = HashMap::new();
        for (mip_level, indices) in &self.mip_level_indices {
            let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsages::INDEX,
            });
            mip_level_buffers.insert(*mip_level, buffer);
        }
        IndexBuffer { mip_level_buffers }
    }
}

pub(crate) struct IndexBuffer {
    mip_level_buffers: HashMap<u32, wgpu::Buffer>,
}

impl IndexBuffer {
    pub(crate) fn set_mip_level_buffer(&self, mip_level: u32, renderpass: &mut wgpu::RenderPass) {
        let buffer = self.mip_level_buffers.get(&mip_level).unwrap();
        renderpass.set_index_buffer(buffer.slice(..), wgpu::IndexFormat::Uint32);
        renderpass.draw_indexed(
            0..buffer.size() as u32 / std::mem::size_of::<u32>() as u32,
            0,
            0..1,
        );
    }
}

fn triangle_strip(image_size: &ImageSize) -> Vec<u32> {
    let mut indices: Vec<u32> = vec![0];
    for row in 0..image_size.height.get() - 1 {
        for mut col in 0..(image_size.width.get()) {
            if row % 2 == 0 {
                if col > 0 {
                    indices.push((row * image_size.width.get() + col) as u32);
                }
                indices.push(((row + 1) * image_size.width.get() + col) as u32);
                if col == image_size.width.get() - 1 && row < image_size.height.get() - 2 {
                    // index is added twice to have smooth transition to next row
                    indices.push(((row + 1) * image_size.width.get() + col - 1) as u32);
                }
            } else {
                col = image_size.width.get() - 1 - col;
                if col < image_size.width.get() - 1 {
                    indices.push((row * image_size.width.get() + col) as u32);
                }
                indices.push(((row + 1) * image_size.width.get() + col) as u32);
                if col == 0 && row < image_size.height.get() - 2 {
                    // index is added twice to have smooth transition to next row
                    indices.push(((row + 1) * image_size.width.get() + 1) as u32);
                }
            }
        }
    }
    indices
}

#[cfg(test)]
mod test {
    use crate::{image::ImageSize, index_buffer::triangle_strip};

    #[test]
    fn test_triangle_strip_minimal() {
        let image_size = ImageSize {
            width: std::num::NonZeroU32::new(3).unwrap(),
            height: std::num::NonZeroU32::new(2).unwrap(),
        };
        let indices = triangle_strip(&image_size);
        let expected_indices = vec![0, 3, 1, 4, 2, 5];
        assert_eq!(indices, expected_indices);
    }

    #[test]
    fn test_triangle_strip_3_rows() {
        let image_size = ImageSize {
            width: std::num::NonZeroU32::new(3).unwrap(),
            height: std::num::NonZeroU32::new(3).unwrap(),
        };
        let indices = triangle_strip(&image_size);
        let expected_indices = vec![0, 3, 1, 4, 2, 5, 4, 8, 4, 7, 3, 6];
        assert_eq!(indices, expected_indices);
    }

    #[test]
    fn test_triangle_strip_4_rows() {
        let image_size = ImageSize {
            width: std::num::NonZeroU32::new(3).unwrap(),
            height: std::num::NonZeroU32::new(4).unwrap(),
        };
        let indices = triangle_strip(&image_size);
        let expected_indices = vec![0, 3, 1, 4, 2, 5, 4, 8, 4, 7, 3, 6, 7, 9, 7, 10, 8, 11];
        assert_eq!(indices, expected_indices);
    }

    #[test]
    fn test_triangle_strip_double_horizontal() {
        let image_size = ImageSize {
            width: std::num::NonZeroU32::new(5).unwrap(),
            height: std::num::NonZeroU32::new(2).unwrap(),
        };
        let indices = triangle_strip(&image_size);
        let expected_indices = vec![0, 5, 1, 6, 2, 7, 3, 8, 4, 9];
        assert_eq!(indices, expected_indices);
    }
}
