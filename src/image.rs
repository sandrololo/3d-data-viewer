use anyhow::anyhow;
use log::info;
use std::{fs::File, num::NonZeroU32, ops::Range};
use tiff::decoder::{Decoder, DecodingResult};

pub struct Image<T> {
    pub size: ImageSize,
    pub data: Vec<T>,
}

impl<T> Image<T>
where
    T: PartialOrd + Copy,
{
    pub fn outlier_removed_data(&self, lower_percentile: f32, upper_percentile: f32) -> Vec<T>
    where
        T: num_traits::Float,
    {
        let mut sorted_data = self.data.clone();
        sorted_data.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let len = sorted_data.len();
        let lower_index = ((lower_percentile / 100.0) * len as f32).round() as usize;
        let upper_index = ((upper_percentile / 100.0) * len as f32).round() as usize;
        let min_value = sorted_data[lower_index];
        let max_value = sorted_data[upper_index];
        self.data
            .iter()
            .map(|&pixel| pixel.clamp(min_value, max_value))
            .collect()
    }

    pub fn scaled_data(&self, new_min: T, new_max: T) -> Vec<T>
    where
        T: num_traits::Float
            + std::ops::Sub<Output = T>
            + std::ops::Add<Output = T>
            + std::ops::Mul<Output = T>
            + std::ops::Div<Output = T>,
    {
        let value_range = value_range(&self.data);
        let old_min = value_range.start;
        let old_max = value_range.end;
        let scale = (new_max - new_min) / (old_max - old_min);
        self.data
            .iter()
            .map(|&value| new_min + (value - old_min) * scale)
            .collect()
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

pub(crate) struct ImageSize {
    pub width: NonZeroU32,
    pub height: NonZeroU32,
}

pub fn value_range<T: PartialOrd + Copy>(data: &Vec<T>) -> Range<T> {
    let mut min_value = data[0];
    let mut max_value = data[0];
    for &value in data {
        if value < min_value {
            min_value = value;
        }
        if value > max_value {
            max_value = value;
        }
    }
    min_value..max_value
}
