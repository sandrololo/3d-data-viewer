use std::{fs::File, ops::Range};
use tiff::decoder::{Decoder, DecodingResult};

pub struct Image<T> {
    pub height: u32,
    pub width: u32,
    pub data: Vec<T>,
}

impl Image<f32> {
    pub fn to_xyz_scaled(
        &self,
        x_range: Range<f32>,
        y_range: Range<f32>,
        z_range: Range<f32>,
    ) -> Vec<[f32; 3]> {
        let z_min = self
            .data
            .iter()
            .cloned()
            .fold(f32::INFINITY, |a, b| a.min(b));
        let z_max = self
            .data
            .iter()
            .cloned()
            .fold(f32::NEG_INFINITY, |a, b| a.max(b));
        println!("z_min: {}, z_max: {}", z_min, z_max);
        let mut result = Vec::with_capacity((self.height * self.width) as usize);
        for y in 0..self.height {
            for x in 0..self.width {
                let index = (y * self.width + x) as usize;
                let z: f32 = self.data[index].into();
                result.push([
                    x_range.start + x as f32 * (x_range.end - x_range.start) / self.width as f32,
                    y_range.start + y as f32 * (y_range.end - y_range.start) / self.height as f32,
                    z_range.start + (z - z_min) / (z_max - z_min) * (z_range.end - z_range.start),
                ]);
            }
        }
        result
    }
}

pub struct SurfaceAmplitudeImage {
    pub surface: Image<f32>,
    pub amplitude: Image<f32>,
}

impl SurfaceAmplitudeImage {
    pub fn from_file(path: &str) -> anyhow::Result<Self> {
        let img_file = File::open(path)?;
        let mut decoder = Decoder::new(img_file)?;
        let dimensions = decoder.dimensions()?;
        let surface = match decoder.read_image()? {
            DecodingResult::F32(data) => Ok(Image {
                height: dimensions.1,
                width: dimensions.0,
                data,
            }),
            _ => Err(anyhow::anyhow!("Unsupported image format")),
        }?;
        decoder.next_image()?;
        let dimensions = decoder.dimensions()?;
        let amplitude = match decoder.read_image()? {
            DecodingResult::F32(data) => Ok(Image {
                height: dimensions.1,
                width: dimensions.0,
                data,
            }),
            _ => Err(anyhow::anyhow!("Unsupported image format")),
        }?;
        Ok(Self { surface, amplitude })
    }
}
