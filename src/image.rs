use std::fs::File;
use tiff::decoder::{Decoder, DecodingResult};

pub struct Image<T> {
    pub height: u32,
    pub width: u32,
    pub data: Vec<T>,
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
