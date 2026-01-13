use std::{collections::HashMap, num::NonZeroU32, ops::Range};

use wgpu::util::DeviceExt;

use crate::image::ImageSize;

pub(crate) struct IndexBufferBuilder {
    indices: Vec<u32>,
    mip_level_ranges: HashMap<u8, Range<u32>>,
}

impl IndexBufferBuilder {
    pub(crate) fn new_triangle_strip(image_size: &ImageSize, mip_levels: u8) -> Self {
        let mut indices: Vec<u32> = vec![];
        let mut mip_level_ranges: HashMap<u8, Range<u32>> = HashMap::new();
        for mip_level in 0..mip_levels {
            let start_index = indices.len() as u32;
            let triangle_strip = triangle_strip(&ImageSize {
                width: NonZeroU32::new(image_size.width.get() / 2u32.pow(mip_level as u32))
                    .expect("Can't be zero"),
                height: NonZeroU32::new(image_size.height.get() / 2u32.pow(mip_level as u32))
                    .expect("Can't be zero"),
            });
            indices.extend(triangle_strip);
            mip_level_ranges.insert(mip_level, start_index..indices.len() as u32);
        }
        log::info!("Index buffer length: {:?}", indices.len());
        log::info!("Number of triangles: {:?}", indices.len() - 2);
        Self {
            indices,
            mip_level_ranges,
        }
    }

    pub(crate) fn create_buffer_init(&self, device: &wgpu::Device) -> IndexBuffer {
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(&self.indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        IndexBuffer {
            buffer,
            mip_level_ranges: self.mip_level_ranges.clone(),
        }
    }
}

pub(crate) struct IndexBuffer {
    pub(crate) buffer: wgpu::Buffer,
    mip_level_ranges: HashMap<u8, Range<u32>>,
}

impl IndexBuffer {
    pub(crate) fn get_mip_level_range(&self, mip_level: u8) -> Range<u32> {
        let range = self.mip_level_ranges.get(&mip_level).unwrap();
        range.clone()
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
    use crate::{image::ImageSize, index_buffer::IndexBufferBuilder};

    #[test]
    fn test_triangle_strip_minimal() {
        let image_size = ImageSize {
            width: std::num::NonZeroU32::new(3).unwrap(),
            height: std::num::NonZeroU32::new(2).unwrap(),
        };
        let indices = IndexBufferBuilder::new_triangle_strip(&image_size, 1).indices;
        let expected_indices = vec![0, 3, 1, 4, 2, 5];
        assert_eq!(indices, expected_indices);
    }

    #[test]
    fn test_triangle_strip_3_rows() {
        let image_size = ImageSize {
            width: std::num::NonZeroU32::new(3).unwrap(),
            height: std::num::NonZeroU32::new(3).unwrap(),
        };
        let indices = IndexBufferBuilder::new_triangle_strip(&image_size, 1).indices;
        let expected_indices = vec![0, 3, 1, 4, 2, 5, 4, 8, 4, 7, 3, 6];
        assert_eq!(indices, expected_indices);
    }

    #[test]
    fn test_triangle_strip_4_rows() {
        let image_size = ImageSize {
            width: std::num::NonZeroU32::new(3).unwrap(),
            height: std::num::NonZeroU32::new(4).unwrap(),
        };
        let indices = IndexBufferBuilder::new_triangle_strip(&image_size, 1).indices;
        let expected_indices = vec![0, 3, 1, 4, 2, 5, 4, 8, 4, 7, 3, 6, 7, 9, 7, 10, 8, 11];
        assert_eq!(indices, expected_indices);
    }

    #[test]
    fn test_triangle_strip_double_horizontal() {
        let image_size = ImageSize {
            width: std::num::NonZeroU32::new(5).unwrap(),
            height: std::num::NonZeroU32::new(2).unwrap(),
        };
        let indices = IndexBufferBuilder::new_triangle_strip(&image_size, 1).indices;
        let expected_indices = vec![0, 5, 1, 6, 2, 7, 3, 8, 4, 9];
        assert_eq!(indices, expected_indices);
    }
}
