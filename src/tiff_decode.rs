use std::num::NonZeroU32;

use anyhow::anyhow;
use imbuf::Image;
use tiff::decoder::{Decoder, DecodingResult, Limits};

pub(crate) trait TiffDecodable: Sized {
    fn extract(result: DecodingResult) -> anyhow::Result<Vec<Self>>;
}

impl TiffDecodable for f32 {
    fn extract(result: DecodingResult) -> anyhow::Result<Vec<Self>> {
        match result {
            DecodingResult::F32(data) => Ok(data),
            other => Err(anyhow!("Unsupported image format: {:?}", other)),
        }
    }
}

impl TiffDecodable for u16 {
    fn extract(result: DecodingResult) -> anyhow::Result<Vec<Self>> {
        match result {
            DecodingResult::U16(data) => Ok(data),
            other => Err(anyhow!("Unsupported image format: {:?}", other)),
        }
    }
}

pub(crate) fn decode_tiff<T: TiffDecodable + imbuf::PixelTypePrimitive, R: std::io::Read + std::io::Seek>(
    reader: R,
) -> anyhow::Result<Image<T, 1>> {
    let mut decoder = Decoder::new(reader)?.with_limits(Limits::unlimited());
    let dimensions = decoder.dimensions()?;
    let data = T::extract(decoder.read_image()?)?;
    Ok(Image::<T, 1>::new_vec(
        data,
        NonZeroU32::new(dimensions.0).ok_or(anyhow!("Invalid width"))?,
        NonZeroU32::new(dimensions.1).ok_or(anyhow!("Invalid height"))?,
    ))
}
