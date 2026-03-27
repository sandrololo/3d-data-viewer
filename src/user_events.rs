use anyhow::anyhow;
use futures::{FutureExt, future::Shared};
use imbuf::Image;
use std::{num::NonZeroU32, sync::Arc};

use crate::{
    State,
    gpu_data::{
        CaptureResult, DataSize,
        pixel_picker::{PixelResult, PixelValue},
    },
    scene::{Overlay, Scene},
};

#[non_exhaustive]
#[allow(dead_code)]
pub(crate) enum UserEvent {
    ResetView,
    SetSurface(Image<f32, 1>),
    SetTexture(Image<u16, 1>),
    SetState(State),
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

pub(crate) trait UserEventHandler {
    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>);
    fn reset_view(&mut self);
    fn set_surface(&mut self, data: Image<f32, 1>);
    fn set_texture(&mut self, data: Image<u16, 1>);
    fn get_pixel_value(
        &mut self,
        sender: futures::channel::oneshot::Sender<
            Shared<std::pin::Pin<Box<dyn std::future::Future<Output = PixelResult>>>>,
        >,
    );
    fn set_texture_shader(&mut self);
    fn set_height_shader(&mut self);
    fn set_overlays(&mut self, overlays: Arc<Vec<Overlay>>);
    fn clear_overlays(&mut self);
    fn reset_orientation(&mut self);
    fn zoom_in(&mut self);
    fn zoom_out(&mut self);
    fn set_percentile(&mut self, percentile: f32);
    fn set_texture_range(&mut self, start: u16, end: u16);
    fn capture_image(
        &mut self,
        sender: futures::channel::oneshot::Sender<
            Shared<std::pin::Pin<Box<dyn std::future::Future<Output = CaptureResult>>>>,
        >,
    );
}

impl UserEventHandler for State {
    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.configure_surface();
        // Resize the picking texture to match the new window size
        self.pixel_picker.resize(&self.device, new_size);
        self.image_capture.resize(
            &self.device,
            DataSize {
                width: NonZeroU32::new(new_size.width).expect("Windows size should not be 0"),
                height: NonZeroU32::new(new_size.height).expect("Windows size should not be 0"),
            },
        );
    }

    fn reset_view(&mut self) {
        self.projection.reset();
        self.transformation.reset();
        self.mouse.reset_zoom();
        self.scene = None;
        self.mip.reset();
    }

    fn set_surface(&mut self, data: Image<f32, 1>) {
        log::info!("Setting new surface image");
        self.percentile_range_buffer
            .update_data(&self.queue, data.buffer());

        self.mip.set_image(&data.dimensions().into(), &self.device);

        self.scene = Some(Scene::new_surface(
            data,
            &self.device,
            &self.queue,
            &self.texture_bind_group_layout,
        ));
    }

    fn set_texture(&mut self, data: Image<u16, 1>) {
        log::info!("Setting new texture image");
        if let Some(scene) = &mut self.scene {
            scene.set_texture(data, &self.queue);
        } else {
            log::warn!("Can't set texture image, surface texture not initialized");
        }
    }

    fn get_pixel_value(
        &mut self,
        sender: futures::channel::oneshot::Sender<
            Shared<std::pin::Pin<Box<dyn std::future::Future<Output = PixelResult>>>>,
        >,
    ) {
        if let Some(scene) = &self.scene {
            if let Some(texture_image) = scene.get_texture_image() {
                self.pixel_picker.write_to_channel(
                    self.device.clone(),
                    scene.get_surface_image(),
                    texture_image,
                    sender,
                );
            } else {
                let future: std::pin::Pin<Box<dyn std::future::Future<Output = PixelResult>>> =
                    Box::pin(async move {
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

    fn set_texture_shader(&mut self) {
        log::info!("Setting texture shader");
        self.use_height_shader = false;
    }

    fn set_height_shader(&mut self) {
        log::info!("Setting height shader");
        self.use_height_shader = true;
    }

    fn set_overlays(&mut self, overlays: Arc<Vec<Overlay>>) {
        log::info!("Setting overlays");
        if let Some(scene) = &mut self.scene {
            scene.set_overlays(overlays, &self.queue);
        }
    }

    fn clear_overlays(&mut self) {
        log::info!("Clearing overlays");
        if let Some(scene) = &mut self.scene {
            scene.clear_overlays(&self.queue);
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
            .scene
            .as_ref()
            .map(|scene| scene.get_surface_image().buffer().to_vec());
        self.percentile_range_buffer.update_percentile(
            &self.queue,
            percentile,
            surface.as_ref().map(|v| v.as_slice()),
        );
    }

    fn set_texture_range(&mut self, start: u16, end: u16) {
        self.texture_range_buffer.update(&self.queue, start, end);
    }

    fn capture_image(
        &mut self,
        sender: futures::channel::oneshot::Sender<
            Shared<std::pin::Pin<Box<dyn std::future::Future<Output = CaptureResult>>>>,
        >,
    ) {
        if self.scene.is_some() {
            self.image_capture
                .write_to_channel(self.device.clone(), sender)
        } else {
            let future: std::pin::Pin<Box<dyn std::future::Future<Output = CaptureResult>>> =
                Box::pin(async move {
                    Err::<Vec<u8>, Arc<anyhow::Error>>(Arc::new(anyhow!("Texture not initialized")))
                });
            if let Err(_) = sender.send(future.shared()) {
                log::error!("Failed to return error message");
            }
        }
    }
}
