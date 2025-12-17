use winit::dpi::{PhysicalPosition, PhysicalSize};

pub struct PixelPicker {
    /// Texture that stores picking data (pixel_x, pixel_y) for each fragment
    picking_texture: wgpu::Texture,
    pub picking_texture_view: wgpu::TextureView,
    /// Buffer to copy a single pixel from the picking texture
    readback_buffer: wgpu::Buffer,
    mouse_position: PhysicalPosition<f64>,
    window_size: PhysicalSize<u32>,
}

impl PixelPicker {
    pub const PICKING_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rg32Uint;

    pub fn new(device: &wgpu::Device, window_size: PhysicalSize<u32>) -> Self {
        let (picking_texture, picking_texture_view) =
            Self::create_picking_texture(device, window_size);
        let readback_buffer = Self::create_readback_buffer(device);

        Self {
            picking_texture,
            picking_texture_view,
            readback_buffer,
            mouse_position: PhysicalPosition::new(0.0, 0.0),
            window_size,
        }
    }

    pub fn resize(&mut self, device: &wgpu::Device, window_size: PhysicalSize<u32>) {
        if self.window_size != window_size {
            let (picking_texture, picking_texture_view) =
                Self::create_picking_texture(device, window_size);
            self.picking_texture = picking_texture;
            self.picking_texture_view = picking_texture_view;
            self.window_size = window_size;
        }
    }

    pub fn update_mouse_position(&mut self, position: PhysicalPosition<f64>) {
        self.mouse_position = position;
    }

    /// Copy the pixel at the current mouse position from the picking texture to the readback buffer.
    /// Call this AFTER the render pass completes but BEFORE submitting the command buffer.
    pub fn copy_pixel_at_mouse(&self, encoder: &mut wgpu::CommandEncoder) {
        let x = (self.mouse_position.x as u32).min(self.window_size.width.saturating_sub(1));
        let y = (self.mouse_position.y as u32).min(self.window_size.height.saturating_sub(1));

        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self.picking_texture,
                mip_level: 0,
                origin: wgpu::Origin3d { x, y, z: 0 },
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &self.readback_buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(256),
                    rows_per_image: Some(1),
                },
            },
            wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
        );
    }

    /// Read the pixel value from the readback buffer.
    /// Returns (pixel_x, pixel_y).
    /// This will block until the GPU work is complete.
    pub fn read(&self, device: &wgpu::Device) -> anyhow::Result<(u32, u32)> {
        let (tx, rx) = std::sync::mpsc::channel();

        self.readback_buffer
            .map_async(wgpu::MapMode::Read, .., move |result| {
                tx.send(result).unwrap();
            });

        device.poll(wgpu::PollType::wait())?;
        rx.recv()??;

        let output_data = self.readback_buffer.get_mapped_range(..);

        let output = Some((
            bytemuck::cast_slice::<u8, u32>(&output_data)[0],
            bytemuck::cast_slice::<u8, u32>(&output_data)[1],
        ));
        drop(output_data);
        self.readback_buffer.unmap();

        output.ok_or_else(|| anyhow::anyhow!("Failed to read pixel value"))
    }

    fn create_picking_texture(
        device: &wgpu::Device,
        window_size: PhysicalSize<u32>,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("picking_texture"),
            size: wgpu::Extent3d {
                width: window_size.width.max(1),
                height: window_size.height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::PICKING_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        (texture, view)
    }

    fn create_readback_buffer(device: &wgpu::Device) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("picking_readback_buffer"),
            size: std::mem::size_of::<[u32; 2]>() as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        })
    }
}
