use anyhow::anyhow;
use futures::{FutureExt, future::Shared};
use imbuf::Image;
use std::sync::Arc;

use crate::{
    State,
    gpu_data::{
        CaptureResult,
        pixel_picker::{PixelResult, PixelValue},
    },
    interaction::Orientation,
    scene::{Overlay, Scene},
};

#[non_exhaustive]
#[allow(dead_code)]
pub(crate) enum UserEvent {
    ResetView,
    SetSurface(Image<f32, 1>),
    SetTexture(Image<u16, 1>),
    SetOrientation(Orientation),
    ResetOrientation,
    SetTextureShader,
    SetHeightShader,
    SetOverlays(Arc<Vec<Overlay>>),
    ClearOverlays,
    GetPixel(
        futures::channel::oneshot::Sender<
            Shared<std::pin::Pin<Box<dyn std::future::Future<Output = PixelResult>>>>,
        >,
    ),
    ZoomIn,
    ZoomOut,
    SetPercentile(f32),
    SetTextureRange(u16, u16),
    CaptureImage(
        futures::channel::oneshot::Sender<
            Shared<std::pin::Pin<Box<dyn std::future::Future<Output = CaptureResult>>>>,
        >,
    ),
}

impl UserEvent {
    pub(crate) fn apply(self, state: &mut State) {
        match self {
            UserEvent::ResetView => {
                state.interaction.reset();
                state.scene = None;
            }
            UserEvent::GetPixel(sender) => {
                if let Some(scene) = &state.scene {
                    if let Some(texture_image) = scene.get_texture_image() {
                        state.interaction.pixel_picker.write_to_channel(
                            state.device.clone(),
                            scene.get_surface_image(),
                            texture_image,
                            sender,
                        );
                    } else {
                        let future: std::pin::Pin<
                            Box<dyn std::future::Future<Output = PixelResult>>,
                        > = Box::pin(async move {
                            Err::<PixelValue, Arc<anyhow::Error>>(Arc::new(anyhow!(
                                "Texture image not initialized"
                            )))
                        });
                        if let Err(_) = sender.send(future.shared()) {
                            log::error!("Failed to return error message");
                        }
                    }
                } else {
                    let future: std::pin::Pin<Box<dyn std::future::Future<Output = PixelResult>>> =
                        Box::pin(async move {
                            Err::<PixelValue, Arc<anyhow::Error>>(Arc::new(anyhow!(
                                "Texture not initialized"
                            )))
                        });
                    if let Err(_) = sender.send(future.shared()) {
                        log::error!("Failed to return error message");
                    }
                }
            }
            UserEvent::CaptureImage(sender) => {
                if state.scene.is_some() {
                    state
                        .interaction
                        .image_capture
                        .write_to_channel(state.device.clone(), sender)
                } else {
                    let future: std::pin::Pin<
                        Box<dyn std::future::Future<Output = CaptureResult>>,
                    > = Box::pin(async move {
                        Err::<Vec<u8>, Arc<anyhow::Error>>(Arc::new(anyhow!(
                            "Texture not initialized"
                        )))
                    });
                    if let Err(_) = sender.send(future.shared()) {
                        log::error!("Failed to return error message");
                    }
                }
            }
            UserEvent::SetTextureShader => {
                log::info!("Setting texture shader");
                state.use_height_shader = false;
            }
            UserEvent::SetHeightShader => {
                log::info!("Setting height shader");
                state.use_height_shader = true;
            }
            UserEvent::SetOverlays(overlays) => {
                log::info!("Setting overlays");
                if let Some(scene) = &mut state.scene {
                    scene.set_overlays(overlays, &state.queue);
                }
            }
            UserEvent::ClearOverlays => {
                log::info!("Clearing overlays");
                if let Some(scene) = &mut state.scene {
                    scene.clear_overlays(&state.queue);
                }
            }
            UserEvent::SetOrientation(orientation) => {
                state.interaction.set_orientation(orientation);
            }
            UserEvent::ResetOrientation => {
                state.interaction.reset_orientation();
            }
            UserEvent::SetSurface(data) => {
                log::info!("Setting new surface image");
                state
                    .percentile_range_buffer
                    .update_data(&state.queue, data.buffer());

                state
                    .interaction
                    .mip
                    .set_image(&data.dimensions().into(), &state.device);

                state.scene = Some(Scene::new_surface(
                    data,
                    &state.device,
                    &state.queue,
                    &state.texture_bind_group_layout,
                ));
            }
            UserEvent::SetTexture(data) => {
                log::info!("Setting new texture image");
                if let Some(scene) = &mut state.scene {
                    scene.set_texture(data, &state.queue);
                } else {
                    log::warn!("Can't set texture image, surface texture not initialized");
                }
            }
            UserEvent::ZoomIn => {
                state.interaction.zoom_in();
            }
            UserEvent::ZoomOut => {
                state.interaction.zoom_out();
            }
            UserEvent::SetPercentile(percentile) => {
                let surface = state
                    .scene
                    .as_ref()
                    .map(|scene| scene.get_surface_image().buffer().to_vec());
                state.percentile_range_buffer.update_percentile(
                    &state.queue,
                    percentile,
                    surface.as_ref().map(|v| v.as_slice()),
                );
            }
            UserEvent::SetTextureRange(start, end) => {
                state.texture_range_buffer.update(&state.queue, start, end);
            }
        }
    }
}
