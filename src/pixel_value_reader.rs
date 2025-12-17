use glam::Vec2;
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use winit::dpi::PhysicalPosition;

pub struct PixelValueReader {
    pub mouse_position_buffer: wgpu::Buffer,
    pub output_buffer: wgpu::Buffer,
    pub temp_buffer: wgpu::Buffer,
}

impl PixelValueReader {
    pub fn new(device: &wgpu::Device) -> Self {
        let mouse_position_buffer = Self::create_mouse_position_buffer(device);
        let pixel_value_buffer = Self::create_pixel_value_buffer(device);
        let temp_buffer = Self::create_temp_buffer(device);
        Self {
            mouse_position_buffer,
            output_buffer: pixel_value_buffer,
            temp_buffer,
        }
    }

    pub fn update_gpu(&self, queue: &wgpu::Queue, mouse_pos: PhysicalPosition<f64>) {
        let mouse_pos = Vec2::new(mouse_pos.x as f32, mouse_pos.y as f32);
        queue.write_buffer(
            &self.mouse_position_buffer,
            0,
            bytemuck::cast_slice(&[mouse_pos.x, mouse_pos.y]),
        );
    }

    pub fn copy_temp_buffer_to_output_buffer(&self, encoder: &mut wgpu::CommandEncoder) -> () {
        encoder.copy_buffer_to_buffer(
            &self.output_buffer,
            0,
            &self.temp_buffer,
            0,
            self.output_buffer.size(),
        );
    }

    pub fn read(&self, device: &wgpu::Device) -> anyhow::Result<(f32, f32, f32)> {
        let mut output = None;
        {
            // The mapping process is async, so we'll need to create a channel to get
            // the success flag for our mapping
            let (tx, rx) = std::sync::mpsc::channel();

            // We send the success or failure of our mapping via a callback
            self.temp_buffer
                .map_async(wgpu::MapMode::Read, .., move |result| {
                    tx.send(result).unwrap()
                });

            // The callback we submitted to map async will only get called after the
            // device is polled or the queue submitted
            device.poll(wgpu::PollType::wait())?;

            // We check if the mapping was successful here
            let _ = rx.recv()?;

            // We then get the bytes that were stored in the buffer
            let output_data = self.temp_buffer.get_mapped_range(..);

            output = Some((
                bytemuck::cast_slice::<u8, f32>(&output_data)[0],
                bytemuck::cast_slice::<u8, f32>(&output_data)[1],
                bytemuck::cast_slice::<u8, f32>(&output_data)[2],
            ));
        }
        // We need to unmap the buffer to be able to use it again
        self.temp_buffer.unmap();
        output.ok_or_else(|| anyhow::anyhow!("Failed to read pixel value"))
    }

    pub fn get_mouse_position_bind_group_layout_entry() -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding: 2,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }
    }

    pub fn get_pixel_value_bind_group_layout_entry() -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding: 3,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: false },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }
    }

    pub fn get_mouse_position_bind_group_entry(&self) -> wgpu::BindGroupEntry {
        wgpu::BindGroupEntry {
            binding: 2,
            resource: self.mouse_position_buffer.as_entire_binding(),
        }
    }

    pub fn get_pixel_value_bind_group_entry(&self) -> wgpu::BindGroupEntry {
        wgpu::BindGroupEntry {
            binding: 3,
            resource: self.output_buffer.as_entire_binding(),
        }
    }
    fn create_mouse_position_buffer(device: &wgpu::Device) -> wgpu::Buffer {
        device.create_buffer_init(&BufferInitDescriptor {
            label: Some("mouse_position_buffer"),
            contents: bytemuck::cast_slice(&[0f32, 0f32]),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE,
        })
    }
    fn create_pixel_value_buffer(device: &wgpu::Device) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("pixel_value_output_buffer"),
            size: std::mem::size_of::<f32>() as u64 * 3,
            usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        })
    }

    fn create_temp_buffer(device: &wgpu::Device) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("temp"),
            size: std::mem::size_of::<f32>() as u64 * 3,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        })
    }
}
