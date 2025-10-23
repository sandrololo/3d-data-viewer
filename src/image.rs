use std::fs::File;
use tiff::decoder::Decoder;

pub struct SurfaceAmplitudeImage {
    pub surface: Vec<f32>,
    pub amplitude: Vec<f32>,
}

impl SurfaceAmplitudeImage {
    pub fn from_file(path: &str) -> anyhow::Result<Self> {
        let img_file = File::open(path)?;
        let mut decoder = Decoder::new(img_file)?;
        let surface = match decoder.read_image()? {
            tiff::decoder::DecodingResult::F32(data) => Ok(data),
            _ => Err(anyhow::anyhow!("Unsupported image format")),
        }?;
        decoder.next_image()?;
        let amplitude = match decoder.read_image()? {
            tiff::decoder::DecodingResult::F32(data) => Ok(data),
            _ => Err(anyhow::anyhow!("Unsupported image format")),
        }?;
        Ok(Self { surface, amplitude })
    }
}
