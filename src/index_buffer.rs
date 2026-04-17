use std::num::NonZeroU32;

use wgpu::util::DeviceExt;

use crate::gpu_data::DataSize;

pub(crate) struct IndexBufferBuilder {
    // The index of the first Vec is the mip level
    mip_level_indices: Vec<Vec<u32>>,
}

impl IndexBufferBuilder {
    pub(crate) fn triangle_strip_length(image_size: DataSize, mip_level: u32) -> u64 {
        let width = image_size.width.get() / 2u32.pow(mip_level as u32);
        let height = image_size.height.get() / 2u32.pow(mip_level as u32);
        (width * height * 2) as u64
    }

    pub(crate) fn new_triangle_strip(image_size: DataSize, mip_levels: &Vec<u32>) -> Self {
        log::info!("Creating index buffer for mip levels: {:?}", mip_levels);
        let mip_level_indices = mip_levels
            .iter()
            .map(|mip_level| {
                let triangle_strip = triangle_strip(&DataSize {
                    width: NonZeroU32::new(image_size.width.get() / 2u32.pow(*mip_level as u32))
                        .expect("Can't be zero"),
                    height: NonZeroU32::new(image_size.height.get() / 2u32.pow(*mip_level as u32))
                        .expect("Can't be zero"),
                });
                log::info!(
                    "MIP level {:?} index buffer length: {:?}",
                    mip_level,
                    triangle_strip.len()
                );
                log::info!("Number of triangles: {:?}", triangle_strip.len() - 2);
                triangle_strip
            })
            .collect();
        Self { mip_level_indices }
    }

    pub(crate) fn create_buffer(&self, device: &wgpu::Device) -> IndexBuffer {
        let mut mip_level_buffers: Vec<wgpu::Buffer> = Vec::new();
        for indices in &self.mip_level_indices {
            let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(indices),
                usage: wgpu::BufferUsages::INDEX,
            });
            mip_level_buffers.push(buffer);
        }
        IndexBuffer { mip_level_buffers }
    }
}

pub(crate) struct IndexBuffer {
    mip_level_buffers: Vec<wgpu::Buffer>,
}

impl IndexBuffer {
    pub(crate) fn set_mip_level_buffer(&self, mip_level: u32, renderpass: &mut wgpu::RenderPass) {
        let buffer = &self.mip_level_buffers[mip_level as usize];
        renderpass.set_index_buffer(buffer.slice(..), wgpu::IndexFormat::Uint32);
        renderpass.draw_indexed(
            0..buffer.size() as u32 / std::mem::size_of::<u32>() as u32,
            0,
            0..1,
        );
    }
}

fn triangle_strip(image_size: &DataSize) -> Vec<u32> {
    let mut indices: Vec<u32> =
        Vec::with_capacity((image_size.width.get() * image_size.height.get() * 2) as usize);
    indices.push(0);
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
    use crate::{gpu_data::DataSize, index_buffer::triangle_strip};

    #[test]
    fn test_triangle_strip_minimal() {
        let image_size = DataSize {
            width: std::num::NonZeroU32::new(3).unwrap(),
            height: std::num::NonZeroU32::new(2).unwrap(),
        };
        let indices = triangle_strip(&image_size);
        let expected_indices = vec![0, 3, 1, 4, 2, 5];
        assert_eq!(indices, expected_indices);
    }

    #[test]
    fn test_triangle_strip_3_rows() {
        let image_size = DataSize {
            width: std::num::NonZeroU32::new(3).unwrap(),
            height: std::num::NonZeroU32::new(3).unwrap(),
        };
        let indices = triangle_strip(&image_size);
        let expected_indices = vec![0, 3, 1, 4, 2, 5, 4, 8, 4, 7, 3, 6];
        assert_eq!(indices, expected_indices);
    }

    #[test]
    fn test_triangle_strip_4_rows() {
        let image_size = DataSize {
            width: std::num::NonZeroU32::new(3).unwrap(),
            height: std::num::NonZeroU32::new(4).unwrap(),
        };
        let indices = triangle_strip(&image_size);
        let expected_indices = vec![0, 3, 1, 4, 2, 5, 4, 8, 4, 7, 3, 6, 7, 9, 7, 10, 8, 11];
        assert_eq!(indices, expected_indices);
    }

    #[test]
    fn test_triangle_strip_double_horizontal() {
        let image_size = DataSize {
            width: std::num::NonZeroU32::new(5).unwrap(),
            height: std::num::NonZeroU32::new(2).unwrap(),
        };
        let indices = triangle_strip(&image_size);
        let expected_indices = vec![0, 5, 1, 6, 2, 7, 3, 8, 4, 9];
        assert_eq!(indices, expected_indices);
    }
}
