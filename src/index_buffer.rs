use std::num::NonZeroU32;

use wgpu::util::DeviceExt;

use crate::gpu_data::DataSize;

pub(crate) struct IndexBufferBuilder {
    // The index of the first Vec is the mip level
    mip_level_indices: Vec<Vec<u32>>,
}

impl IndexBufferBuilder {
    pub(crate) fn triangle_list_length(image_size: DataSize, mip_level: u32) -> u64 {
        let width = image_size.width.get() / 2u32.pow(mip_level);
        let height = image_size.height.get() / 2u32.pow(mip_level);
        ((width - 1) * (height - 1) * 6) as u64
    }

    pub(crate) fn new_triangle_list(image_size: DataSize, mip_levels: &Vec<u32>) -> Self {
        log::info!("Creating index buffer for mip levels: {:?}", mip_levels);
        let mip_level_indices = mip_levels
            .iter()
            .map(|mip_level| {
                let indices = triangle_list(&DataSize {
                    width: NonZeroU32::new(image_size.width.get() / 2u32.pow(*mip_level))
                        .expect("Can't be zero"),
                    height: NonZeroU32::new(image_size.height.get() / 2u32.pow(*mip_level))
                        .expect("Can't be zero"),
                });
                log::info!(
                    "MIP level {:?} index buffer length: {:?}",
                    mip_level,
                    indices.len()
                );
                log::info!("Number of triangles: {:?}", indices.len() / 3);
                indices
            })
            .collect();
        Self { mip_level_indices }
    }

    /// Creates a triangle list index buffer where triangles touching any invalid (0) pixel
    /// in the mask are excluded, creating holes in the mesh.
    /// The mask must have the same dimensions as the image.
    pub(crate) fn new_triangle_list_masked(image_size: DataSize, mask: &[u8]) -> Self {
        log::info!("Creating masked index buffer");
        let indices = triangle_list_masked(&image_size, mask);
        log::info!(
            "Masked index buffer length: {:?}, triangles: {:?}",
            indices.len(),
            indices.len() / 3
        );
        Self {
            mip_level_indices: vec![indices],
        }
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

/// Generates triangle list indices for a grid of vertices.
/// Each quad (2x2 vertices) produces 2 triangles (6 indices).
fn triangle_list(image_size: &DataSize) -> Vec<u32> {
    let w = image_size.width.get();
    let h = image_size.height.get();
    let mut indices: Vec<u32> = Vec::with_capacity(((w - 1) * (h - 1) * 6) as usize);
    for row in 0..h - 1 {
        for col in 0..w - 1 {
            let tl = row * w + col;
            let tr = row * w + col + 1;
            let bl = (row + 1) * w + col;
            let br = (row + 1) * w + col + 1;
            // Triangle 1: TL, BL, TR
            indices.push(tl);
            indices.push(bl);
            indices.push(tr);
            // Triangle 2: TR, BL, BR
            indices.push(tr);
            indices.push(bl);
            indices.push(br);
        }
    }
    indices
}

/// Generates triangle list indices, skipping triangles where any vertex is invalid (mask == 0).
/// The mask is a flat array of u8 values (0 = invalid, non-zero = valid) with the same
/// dimensions as the image.
fn triangle_list_masked(image_size: &DataSize, mask: &[u8]) -> Vec<u32> {
    let w = image_size.width.get();
    let h = image_size.height.get();
    let mut indices: Vec<u32> = Vec::with_capacity(((w - 1) * (h - 1) * 6) as usize);
    for row in 0..h - 1 {
        for col in 0..w - 1 {
            let tl = row * w + col;
            let tr = row * w + col + 1;
            let bl = (row + 1) * w + col;
            let br = (row + 1) * w + col + 1;
            // Triangle 1: TL, BL, TR — only if all 3 vertices are valid
            if mask[tl as usize] != 0 && mask[bl as usize] != 0 && mask[tr as usize] != 0 {
                indices.push(tl);
                indices.push(bl);
                indices.push(tr);
            }
            // Triangle 2: TR, BL, BR — only if all 3 vertices are valid
            if mask[tr as usize] != 0 && mask[bl as usize] != 0 && mask[br as usize] != 0 {
                indices.push(tr);
                indices.push(bl);
                indices.push(br);
            }
        }
    }
    indices
}

#[cfg(test)]
mod test {
    use crate::{
        gpu_data::DataSize,
        index_buffer::{triangle_list, triangle_list_masked},
    };

    #[test]
    fn test_triangle_list_minimal() {
        let image_size = DataSize {
            width: std::num::NonZeroU32::new(3).unwrap(),
            height: std::num::NonZeroU32::new(2).unwrap(),
        };
        let indices = triangle_list(&image_size);
        // 2 quads * 6 indices = 12 indices
        let expected_indices = vec![
            0, 3, 1, 1, 3, 4, // quad (0,0)
            1, 4, 2, 2, 4, 5, // quad (0,1)
        ];
        assert_eq!(indices, expected_indices);
    }

    #[test]
    fn test_triangle_list_3x3() {
        let image_size = DataSize {
            width: std::num::NonZeroU32::new(3).unwrap(),
            height: std::num::NonZeroU32::new(3).unwrap(),
        };
        let indices = triangle_list(&image_size);
        // 4 quads * 6 indices = 24 indices
        assert_eq!(indices.len(), 24);
    }

    #[test]
    fn test_triangle_list_masked_all_valid() {
        let image_size = DataSize {
            width: std::num::NonZeroU32::new(3).unwrap(),
            height: std::num::NonZeroU32::new(2).unwrap(),
        };
        let mask = vec![1u8; 6];
        let indices = triangle_list_masked(&image_size, &mask);
        let expected_indices = vec![0, 3, 1, 1, 3, 4, 1, 4, 2, 2, 4, 5];
        assert_eq!(indices, expected_indices);
    }

    #[test]
    fn test_triangle_list_masked_hole() {
        // 3x2 grid, mask out vertex 1 (top middle)
        // 0  1  2
        // 3  4  5
        // Vertex 1 is invalid, so triangles using vertex 1 are skipped
        let image_size = DataSize {
            width: std::num::NonZeroU32::new(3).unwrap(),
            height: std::num::NonZeroU32::new(2).unwrap(),
        };
        let mask = vec![1, 0, 1, 1, 1, 1];
        let indices = triangle_list_masked(&image_size, &mask);
        // quad (0,0): TL=0, TR=1, BL=3, BR=4
        //   tri1 (0,3,1): vertex 1 invalid -> skip
        //   tri2 (1,3,4): vertex 1 invalid -> skip
        // quad (0,1): TL=1, TR=2, BL=4, BR=5
        //   tri1 (1,4,2): vertex 1 invalid -> skip
        //   tri2 (2,4,5): all valid -> keep
        let expected_indices = vec![2, 4, 5];
        assert_eq!(indices, expected_indices);
    }

    #[test]
    fn test_triangle_list_masked_all_invalid() {
        let image_size = DataSize {
            width: std::num::NonZeroU32::new(3).unwrap(),
            height: std::num::NonZeroU32::new(2).unwrap(),
        };
        let mask = vec![0u8; 6];
        let indices = triangle_list_masked(&image_size, &mask);
        assert_eq!(indices.len(), 0);
    }
}
