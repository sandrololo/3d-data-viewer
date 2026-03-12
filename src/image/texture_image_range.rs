use wgpu::util::DeviceExt;

pub(crate) struct TextureImageRangeBuffer {
    buffer: wgpu::Buffer,
    start: u16,
    end: u16,
}

impl TextureImageRangeBuffer {
    pub fn new(device: &wgpu::Device) -> Self {
        Self {
            buffer: Self::create_buffer(device, 0, 2000),
            start: 0,
            end: 2000,
        }
    }

    pub fn get_bind_group_entry(&self) -> wgpu::BindGroupEntry {
        wgpu::BindGroupEntry {
            binding: 2,
            resource: self.buffer.as_entire_binding(),
        }
    }

    pub fn update(&mut self, queue: &wgpu::Queue, start: u16, end: u16) {
        assert!(start < end, "Start must be less than end");
        log::info!("Updating texture range: {} - {}", start, end);
        self.start = start;
        self.end = end;
        queue.write_buffer(
            &self.buffer,
            0,
            bytemuck::cast_slice(&[self.start as u32, self.end as u32]),
        );
    }

    fn create_buffer(device: &wgpu::Device, start: u16, end: u16) -> wgpu::Buffer {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("texture_range_buffer"),
            contents: bytemuck::cast_slice(&[start as u32, end as u32]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        })
    }

    pub fn get_bind_group_layout_entry() -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding: 2,
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
