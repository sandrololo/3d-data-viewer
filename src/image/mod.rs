pub(crate) use crate::image::{amplitude_range::*, percentile_range::*};

use anyhow::anyhow;
use log::info;
#[cfg(not(target_arch = "wasm32"))]
use std::fs::File;
use std::num::NonZeroU32;
use tiff::decoder::{Decoder, DecodingResult, Limits};

mod amplitude_range;
mod percentile_range;

pub struct Image<T> {
    pub size: ImageSize,
    pub data: Vec<T>,
}

impl<T> Image<T>
where
    T: Copy,
{
    pub fn get_pixel(&self, x: u32, y: u32) -> T {
        self.data[(y * self.size.width.get() + x) as usize]
    }
}

impl TryFrom<Vec<u8>> for Image<f32> {
    type Error = anyhow::Error;
    fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
        let mut decoder =
            Decoder::new(std::io::Cursor::new(bytes))?.with_limits(Limits::unlimited());
        let dimensions = decoder.dimensions()?;
        let image = match decoder.read_image()? {
            DecodingResult::F32(data) => Ok(Image {
                size: ImageSize {
                    width: NonZeroU32::new(dimensions.0).ok_or(anyhow!("Invalid width"))?,
                    height: NonZeroU32::new(dimensions.1).ok_or(anyhow!("Invalid height"))?,
                },
                data,
            }),
            _ => Err(anyhow::anyhow!("Unsupported surface image format")),
        }?;
        Ok(Image {
            size: ImageSize {
                width: NonZeroU32::new(dimensions.0).unwrap(),
                height: NonZeroU32::new(dimensions.1).unwrap(),
            },
            data: image.data,
        })
    }
}

impl TryFrom<Vec<u8>> for Image<u16> {
    type Error = anyhow::Error;
    fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
        let mut decoder =
            Decoder::new(std::io::Cursor::new(bytes))?.with_limits(Limits::unlimited());
        let dimensions = decoder.dimensions()?;
        let image = match decoder.read_image()? {
            DecodingResult::U16(data) => Ok(Image {
                size: ImageSize {
                    width: NonZeroU32::new(dimensions.0).ok_or(anyhow!("Invalid width"))?,
                    height: NonZeroU32::new(dimensions.1).ok_or(anyhow!("Invalid height"))?,
                },
                data,
            }),
            _ => Err(anyhow::anyhow!("Unsupported image format")),
        }?;
        Ok(Image {
            size: ImageSize {
                width: NonZeroU32::new(dimensions.0).unwrap(),
                height: NonZeroU32::new(dimensions.1).unwrap(),
            },
            data: image.data,
        })
    }
}

pub struct SurfaceAmplitudeImage {
    pub surface: Image<f32>,
    pub amplitude: Image<f32>,
}

impl SurfaceAmplitudeImage {
    #[allow(dead_code)]
    pub async fn from_url(url: &str) -> anyhow::Result<Self> {
        let response = reqwest::get(url).await?;
        let body = response.bytes().await?;
        let mut decoder =
            Decoder::new(std::io::Cursor::new(body))?.with_limits(Limits::unlimited());
        let dimensions = decoder.dimensions()?;
        let surface = match decoder.read_image()? {
            DecodingResult::F32(data) => Ok(Image {
                size: ImageSize {
                    width: NonZeroU32::new(dimensions.0).ok_or(anyhow!("Invalid width"))?,
                    height: NonZeroU32::new(dimensions.1).ok_or(anyhow!("Invalid height"))?,
                },
                data,
            }),
            _ => Err(anyhow::anyhow!("Unsupported surface image format")),
        }?;
        decoder.next_image()?;
        let dimensions = decoder.dimensions()?;
        let amplitude = match decoder.read_image()? {
            DecodingResult::F32(data) => Ok(Image {
                size: ImageSize {
                    width: NonZeroU32::new(dimensions.0).ok_or(anyhow!("Invalid width"))?,
                    height: NonZeroU32::new(dimensions.1).ok_or(anyhow!("Invalid height"))?,
                },
                data,
            }),
            _ => Err(anyhow::anyhow!("Unsupported amplitude image format")),
        }?;
        info!(
            "Loaded surface & amplitude image with size {}x{} from {}",
            surface.size.width, surface.size.height, url,
        );
        Ok(Self { surface, amplitude })
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn from_file(path: &str) -> anyhow::Result<Self> {
        let img_file = File::open(path)?;
        let mut decoder = Decoder::new(img_file)?.with_limits(Limits::unlimited());
        let dimensions = decoder.dimensions()?;
        let surface = match decoder.read_image()? {
            DecodingResult::F32(data) => Ok(Image {
                size: ImageSize {
                    width: NonZeroU32::new(dimensions.0).ok_or(anyhow!("Invalid width"))?,
                    height: NonZeroU32::new(dimensions.1).ok_or(anyhow!("Invalid height"))?,
                },
                data,
            }),
            _ => Err(anyhow::anyhow!("Unsupported surface image format")),
        }?;
        decoder.next_image()?;
        let dimensions = decoder.dimensions()?;
        let amplitude = match decoder.read_image()? {
            DecodingResult::F32(data) => Ok(Image {
                size: ImageSize {
                    width: NonZeroU32::new(dimensions.0).ok_or(anyhow!("Invalid width"))?,
                    height: NonZeroU32::new(dimensions.1).ok_or(anyhow!("Invalid height"))?,
                },
                data,
            }),
            _ => Err(anyhow::anyhow!("Unsupported amplitude image format")),
        }?;
        info!(
            "Loaded surface & amplitude image with size {}x{} from {}",
            surface.size.width, surface.size.height, path,
        );
        Ok(Self { surface, amplitude })
    }
}

#[derive(Clone)]
pub(crate) struct ImageSize {
    pub width: NonZeroU32,
    pub height: NonZeroU32,
}

impl ImageSize {
    pub(crate) fn create_buffer(device: &wgpu::Device) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("image_dims_buffer"),
            size: std::mem::size_of::<[u32; 2]>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    pub(crate) fn write_buffer(&self, queue: &wgpu::Queue, buffer: &wgpu::Buffer) {
        queue.write_buffer(
            buffer,
            0,
            bytemuck::cast_slice(&[self.width.get(), self.height.get()]),
        );
    }

    pub fn get_bind_group_entry(buffer: &wgpu::Buffer) -> wgpu::BindGroupEntry {
        wgpu::BindGroupEntry {
            binding: 0,
            resource: buffer.as_entire_binding(),
        }
    }

    pub fn get_bind_group_layout_entry() -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding: 0,
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
