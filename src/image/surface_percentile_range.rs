pub(crate) struct SurfacePercentileRangeBuffer {
    buffer: wgpu::Buffer,
    percentile: f32,
}

impl SurfacePercentileRangeBuffer {
    pub fn new(device: &wgpu::Device) -> Self {
        Self {
            buffer: Self::create_buffer(device),
            percentile: 0.98,
        }
    }

    pub fn get_bind_group_entry(&self) -> wgpu::BindGroupEntry {
        wgpu::BindGroupEntry {
            binding: 1,
            resource: self.buffer.as_entire_binding(),
        }
    }

    pub fn update_percentile(
        &mut self,
        queue: &wgpu::Queue,
        percentile: f32,
        data: Option<&[f32]>,
    ) {
        assert!(percentile >= 0.5, "Percentile must be greater than 0.5");
        assert!(percentile < 1.0, "Percentile must be less than 1.0");
        log::info!("Updating percentile: {}", percentile);
        self.percentile = percentile;
        if let Some(data) = data {
            self.update_data(queue, data);
        }
    }

    pub fn update_data(&self, queue: &wgpu::Queue, data: &[f32]) {
        let mut vec = data.to_vec();
        let (_, lower, _) = vec.select_nth_unstable_by(
            (data.len() as f32 * (1. - self.percentile)) as usize,
            |a, b| a.total_cmp(b),
        );
        let lower = *lower;
        let (_, upper, _) = vec
            .select_nth_unstable_by((data.len() as f32 * self.percentile) as usize, |a, b| {
                a.total_cmp(b)
            });
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

    pub fn get_bind_group_layout_entry() -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding: 1,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }
    }
}
