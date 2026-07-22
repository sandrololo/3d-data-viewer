use std::sync::Arc;

use imbuf::Image;

pub struct TopologyPercentileRangeBuffer {
    buffer: wgpu::Buffer,
    percentile: f32,
    z_min: f32,
    z_max: f32,
}

impl TopologyPercentileRangeBuffer {
    pub fn new(device: &wgpu::Device) -> Self {
        Self {
            buffer: Self::create_buffer(device),
            percentile: 0.98,
            z_min: 0.0,
            z_max: 0.0,
        }
    }

    pub fn z_range(&self) -> (f32, f32) {
        (self.z_min, self.z_max)
    }

    pub fn get_bind_group_entry(&self) -> wgpu::BindGroupEntry<'_> {
        wgpu::BindGroupEntry {
            binding: 1,
            resource: self.buffer.as_entire_binding(),
        }
    }

    pub fn update_percentile(
        &mut self,
        queue: &wgpu::Queue,
        percentile: f32,
        data: Option<Arc<Image<f32, 1>>>,
    ) {
        assert!(percentile >= 0.5, "Percentile must be greater than 0.5");
        assert!(percentile < 1.0, "Percentile must be less than 1.0");
        log::info!("Updating percentile: {}", percentile);
        self.percentile = percentile;
        if let Some(data) = data {
            self.update_data(queue, data.buffer());
        }
    }

    pub fn update_data(&mut self, queue: &wgpu::Queue, data: &[f32]) {
        let mut vec = data.to_vec();
        let total_pixels = vec.len();
        let (_, lower, _) = vec.select_nth_unstable_by(
            (total_pixels as f32 * (1. - self.percentile)) as usize,
            |a, b| a.total_cmp(b),
        );
        let lower = *lower;
        let (_, upper, _) = vec
            .select_nth_unstable_by((total_pixels as f32 * self.percentile) as usize, |a, b| {
                a.total_cmp(b)
            });
        self.z_min = lower;
        self.z_max = *upper;
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[lower, *upper]));
    }

    fn create_buffer(device: &wgpu::Device) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("percentile_range_buffer"),
            size: std::mem::size_of::<[f32; 2]>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }
}
