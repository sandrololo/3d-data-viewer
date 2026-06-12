use imbuf::Image;
use std::sync::Arc;

use crate::{
    events::SharedFuture,
    gpu_data::{CaptureResult, pixel_picker::PixelResult},
    interaction::Orientation,
    render::pipeline::FragmentShaderVariant,
    scene::Overlay,
};

#[non_exhaustive]
#[allow(dead_code)]
pub(crate) enum UserEvent {
    ResetView,
    SetTopology(Image<f32, 1>),
    SetTopologyMasked(Image<f32, 1>, Vec<u8>),
    SetTexture(Image<u16, 1>),
    SetOrientation(Orientation),
    ResetOrientation,
    SetFragmentShader(FragmentShaderVariant),
    SetOverlays(Arc<Vec<Overlay>>),
    ClearOverlays,
    GetPixel(futures::channel::oneshot::Sender<SharedFuture<PixelResult>>),
    ZoomIn,
    ZoomOut,
    SetPercentile(f32),
    SetTextureRange(u16, u16),
    SetMipOverride(Option<u32>),
    DisplayGrid(bool),
    CaptureImage(futures::channel::oneshot::Sender<SharedFuture<CaptureResult>>),
}
