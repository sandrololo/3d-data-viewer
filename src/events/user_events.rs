use anyhow::anyhow;
use futures::{FutureExt, future::Shared};
use imbuf::Image;
use std::{pin::Pin, sync::Arc};

use crate::{
    State,
    events::SharedFuture,
    gpu_data::{CaptureResult, pixel_picker::PixelResult},
    interaction::Orientation,
    render::pipeline::FragmentShaderVariant,
    scene::{Overlay, Scene},
};

#[non_exhaustive]
#[allow(dead_code)]
pub(crate) enum UserEvent {
    ResetView,
    SetTopology(Image<f32, 1>),
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
    DisplayGrid(bool),
    CaptureImage(futures::channel::oneshot::Sender<SharedFuture<CaptureResult>>),
}

impl UserEvent {
    pub(crate) fn apply(self, state: &mut State) {
        match self {
            UserEvent::ResetView => {
                state.interaction.reset();
                state.scene = None;
            }
            UserEvent::GetPixel(sender) => {
                if let Some(scene) = &state.scene
                    && let Some(texture_image) = scene.get_texture_image()
                {
                    state.interaction.pixel_picker.write_to_channel(
                        state.device.clone(),
                        scene.get_topology_image(),
                        texture_image,
                        sender,
                    );
                } else {
                    send_err(sender, "Texture not initialized")
                }
            }
            UserEvent::CaptureImage(sender) => {
                if state.scene.is_some() {
                    state
                        .interaction
                        .image_capture
                        .write_to_channel(state.device.clone(), sender)
                } else {
                    send_err(sender, "Texture not initialized");
                }
            }
            UserEvent::SetFragmentShader(variant) => {
                log::info!("Setting shader: {:?}", variant);
                state.interaction.set_fragment_shader_variant(variant);
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
            UserEvent::SetTopology(data) => {
                log::info!("Setting new topology image");
                state
                    .interaction
                    .percentile_range_buffer
                    .update_data(&state.queue, data.buffer());

                state
                    .interaction
                    .mip
                    .set_image(&data.dimensions().into(), &state.device);

                state.renderer.update_axes_origin(
                    data.dimensions(),
                    state.interaction.percentile_range_buffer.z_range(),
                );

                state.scene = Some(Scene::new_topology(
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
                    log::warn!("Can't set texture image, topology not initialized");
                }
            }
            UserEvent::ZoomIn => {
                state.interaction.zoom_in();
            }
            UserEvent::ZoomOut => {
                state.interaction.zoom_out();
            }
            UserEvent::SetPercentile(percentile) => {
                let topology = state
                    .scene
                    .as_ref()
                    .map(|scene| scene.get_topology_image().buffer().to_vec());
                state.interaction.percentile_range_buffer.update_percentile(
                    &state.queue,
                    percentile,
                    topology.as_ref().map(|v| v.as_slice()),
                );
                state
                    .renderer
                    .update_z_range(state.interaction.percentile_range_buffer.z_range());
            }
            UserEvent::SetTextureRange(start, end) => {
                state
                    .interaction
                    .texture_range_buffer
                    .update(&state.queue, start, end);
            }
            UserEvent::DisplayGrid(visible) => {
                state.renderer.display_grid(visible);
            }
        }
    }
}

fn send_err<T>(
    sender: futures::channel::oneshot::Sender<
        Shared<Pin<Box<dyn Future<Output = Result<T, Arc<anyhow::Error>>>>>>,
    >,
    msg: &str,
) where
    T: Clone,
{
    let msg = msg.to_owned();
    let future: std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<T, Arc<anyhow::Error>>>>,
    > = Box::pin(async move { Err(Arc::new(anyhow!("{}", msg))) });
    if sender.send(future.shared()).is_err() {
        log::error!("Failed to return error message");
    }
}
