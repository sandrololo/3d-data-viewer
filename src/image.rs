use log::info;
use std::{fs::File, ops::Range};
use tiff::decoder::{Decoder, DecodingResult};

pub struct Image<T> {
    pub height: u32,
    pub width: u32,
    pub data: Vec<T>,
}

impl<T> Image<T>
where
    T: PartialOrd + Copy,
{
    pub fn value_range(&self) -> Range<T> {
        let mut min_value = self.data[0];
        let mut max_value = self.data[0];
        for &value in &self.data {
            if value < min_value {
                min_value = value;
            }
            if value > max_value {
                max_value = value;
            }
        }
        min_value..max_value
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
        info!(
            "Loaded surface & amplitude image with size {}x{} from {}",
            surface.width, surface.height, path,
        );
        Ok(Self { surface, amplitude })
    }
}
