use anyhow::anyhow;
use futures::{FutureExt, future::Shared};
use std::sync::Arc;

use crate::{
    State,
    image::Image,
    pixel_picker::{PixelResult, PixelValue},
    texture::{Overlay, Texture},
};

#[non_exhaustive]
#[allow(dead_code)]
pub(crate) enum UserEvent {
    ResetView,
    SetSurface(Image<f32>),
    SetAmplitude(Image<u16>),
    SetState(State),
    ResetOrientation,
    SetAmplitudeShader,
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
    SetAmplitudeRange(u16, u16),
}

pub(crate) trait UserEventHandler {
    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>);
    fn reset_view(&mut self);
    fn set_surface(&mut self, data: Image<f32>);
    fn set_amplitude(&mut self, data: Image<u16>);
    fn get_pixel_value(
        &mut self,
        sender: futures::channel::oneshot::Sender<
            Shared<std::pin::Pin<Box<dyn std::future::Future<Output = PixelResult>>>>,
        >,
    );
    fn set_amplitude_shader(&mut self);
    fn set_height_shader(&mut self);
    fn set_overlays(&mut self, overlays: Arc<Vec<Overlay>>);
    fn clear_overlays(&mut self);
    fn reset_orientation(&mut self);
    fn zoom_in(&mut self);
    fn zoom_out(&mut self);
    fn set_percentile(&mut self, percentile: f32);
    fn set_amplitude_range(&mut self, start: u16, end: u16);
}

impl UserEventHandler for State {
    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.configure_surface();
        // Resize the picking texture to match the new window size
        self.pixel_picker.resize(&self.device, new_size);
    }

    fn reset_view(&mut self) {
        self.projection.reset();
        self.transformation.reset();
        self.mouse.reset_zoom();
        self.texture = None;
        self.mip.reset();
    }

    fn set_surface(&mut self, data: Image<f32>) {
        log::info!("Setting new surface image");
        self.percentile_range_buffer
            .update_data(&self.queue, &data.data);

        self.mip.set_image(&data.size, &self.device);

        let texture = Texture::new(&self.device, data, &self.texture_bind_group_layout);
        texture.surface.write_to_queue(&self.queue);
        self.texture = Some(texture);
    }

    fn set_amplitude(&mut self, data: Image<u16>) {
        log::info!("Setting new amplitude image");
        if let Some(texture) = &mut self.texture {
            texture.amplitude.set_image(data);
            texture.amplitude.write_to_queue(&self.queue);
        } else {
            log::warn!("Can't set amplitude image, texture not initialized");
        }
    }

    fn get_pixel_value(
        &mut self,
        sender: futures::channel::oneshot::Sender<
            Shared<std::pin::Pin<Box<dyn std::future::Future<Output = PixelResult>>>>,
        >,
    ) {
        if let Some(texture) = &self.texture {
            if let Some(amplitude) = &texture.amplitude.image {
                self.pixel_picker.write_to_channel(
                    self.device.clone(),
                    texture.surface.image.clone(),
                    amplitude.clone(),
                    sender,
                );
            } else {
                let future: std::pin::Pin<Box<dyn std::future::Future<Output = PixelResult>>> =
                    Box::pin(async move {
                        Err::<PixelValue, Arc<anyhow::Error>>(Arc::new(anyhow!(
                            "Amplitude image not initialized"
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

    fn set_amplitude_shader(&mut self) {
        log::info!("Setting amplitude shader");
        self.use_height_shader = false;
    }

    fn set_height_shader(&mut self) {
        log::info!("Setting height shader");
        self.use_height_shader = true;
    }

    fn set_overlays(&mut self, overlays: Arc<Vec<Overlay>>) {
        log::info!("Setting overlays");
        if let Some(texture) = &mut self.texture {
            texture.overlay.set_overlays(overlays);
            texture.overlay.write_to_queue(&self.queue);
        }
    }

    fn clear_overlays(&mut self) {
        log::info!("Clearing overlays");
        if let Some(texture) = &mut self.texture {
            texture.overlay.set_overlays(Arc::new(Vec::new()));
            texture.overlay.write_to_queue(&self.queue);
        }
    }

    fn reset_orientation(&mut self) {
        self.projection.reset();
        self.transformation.reset();
        self.mouse.reset_zoom();
    }

    fn zoom_in(&mut self) {
        self.mouse.zoom_in();
        self.projection.zoom(self.mouse.get_zoom());
        self.mip.set_zoom(self.mouse.get_zoom());
    }

    fn zoom_out(&mut self) {
        self.mouse.zoom_out();
        self.projection.zoom(self.mouse.get_zoom());
        self.mip.set_zoom(self.mouse.get_zoom());
    }

    fn set_percentile(&mut self, percentile: f32) {
        let surface = self
            .texture
            .as_ref()
            .and_then(|texture| Some(&texture.surface.image.data));
        self.percentile_range_buffer
            .update_percentile(&self.queue, percentile, surface);
    }

    fn set_amplitude_range(&mut self, start: u16, end: u16) {
        self.amplitude_range_buffer.update(&self.queue, start, end);
    }
}
