use imbuf::Image;
use std::{num::NonZeroU32, sync::Arc};
use tiff::decoder::{Decoder, DecodingResult, Limits};
use wasm_bindgen::{JsValue, prelude::*, throw_str};
use winit::event_loop::EventLoop;

use crate::{
    ImageViewer3D,
    events::{Event, UserEvent},
    gpu_data::pixel_picker::PixelValue,
    interaction::Orientation,
    scene::Overlay,
};

mod wasm_commands {
    use std::cell::RefCell;
    use std::sync::Arc;
    use winit::window::Window;

    thread_local! {
        /// Reference to the window for requesting redraws
        pub static WINDOW: RefCell<Option<Arc<Window>>> = RefCell::new(None);
    }
}

#[wasm_bindgen]
pub struct WasmViewer {
    proxy: Option<winit::event_loop::EventLoopProxy<Event>>,
    canvas_id: String,
}

#[wasm_bindgen]
impl WasmViewer {
    pub fn new(canvas_id: String) -> Result<Self, JsValue> {
        Ok(Self {
            proxy: None,
            canvas_id,
        })
    }

    pub fn run(&mut self) -> Result<(), JsValue> {
        console_log::init_with_level(log::Level::Info)
            .map_err(|e| JsValue::from_str(&format!("Error initializing console_log: {}", e)))?;
        console_error_panic_hook::set_once();

        let event_loop = EventLoop::with_user_event()
            .build()
            .map_err(|e| JsValue::from_str(&format!("Error building event loop: {}", e)))?;
        self.proxy = Some(event_loop.create_proxy());
        let canvas_id = self.canvas_id.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let mut app = ImageViewer3D::new(&event_loop, canvas_id);
            event_loop
                .run_app(&mut app)
                .map_err(|e| JsValue::from_str(&format!("Error running event loop: {}", e)))
                .unwrap_throw();
        });
        Ok(())
    }

    pub async fn reset_view(&self) -> Result<(), JsValue> {
        if let Some(proxy) = &self.proxy {
            proxy
                .send_event(UserEvent::ResetView.into())
                .map_err(|e| JsValue::from_str(&format!("Error: {}", e)))?;
            Ok(())
        } else {
            Err(JsValue::from_str("Event loop proxy not initialized"))
        }
    }

    pub async fn set_surface(&self, data: Vec<u8>) -> Result<(), JsValue> {
        if let Some(proxy) = &self.proxy {
            let mut decoder = Decoder::new(std::io::Cursor::new(&data))
                .map_err(|e| JsValue::from_str(&format!("Error: {}", e)))?
                .with_limits(Limits::unlimited());
            let dimensions = decoder
                .dimensions()
                .map_err(|e| JsValue::from_str(&format!("Error: {}", e)))?;
            let decoded_data = match decoder
                .read_image()
                .map_err(|e| JsValue::from_str(&format!("Error: {}", e)))?
            {
                DecodingResult::F32(data) => Ok(data),
                x => Err(JsValue::from_str(&format!(
                    "Unsupported image format: {:?}",
                    x
                ))),
            }?;
            let image = Image::<f32, 1>::new_vec(
                decoded_data,
                NonZeroU32::new(dimensions.0).ok_or(JsValue::from_str("Invalid width"))?,
                NonZeroU32::new(dimensions.1).ok_or(JsValue::from_str("Invalid height"))?,
            );
            proxy
                .send_event(UserEvent::SetSurface(image).into())
                .map_err(|e| JsValue::from_str(&format!("Error: {}", e)))?;
            Ok(())
        } else {
            Err(JsValue::from_str("Event loop proxy not initialized"))
        }
    }

    pub async fn set_texture(&self, data: Vec<u8>) -> Result<(), JsValue> {
        if let Some(proxy) = &self.proxy {
            let mut decoder = Decoder::new(std::io::Cursor::new(&data))
                .map_err(|e| JsValue::from_str(&format!("Error: {}", e)))?
                .with_limits(Limits::unlimited());
            let dimensions = decoder
                .dimensions()
                .map_err(|e| JsValue::from_str(&format!("Error: {}", e)))?;
            let decoded_data = match decoder
                .read_image()
                .map_err(|e| JsValue::from_str(&format!("Error: {}", e)))?
            {
                DecodingResult::U16(data) => Ok(data),
                x => Err(JsValue::from_str(&format!(
                    "Unsupported image format: {:?}",
                    x
                ))),
            }?;
            let image = Image::<u16, 1>::new_vec(
                decoded_data,
                NonZeroU32::new(dimensions.0).ok_or(JsValue::from_str("Invalid width"))?,
                NonZeroU32::new(dimensions.1).ok_or(JsValue::from_str("Invalid height"))?,
            );
            proxy
                .send_event(UserEvent::SetTexture(image).into())
                .map_err(|e| JsValue::from_str(&format!("Error: {}", e)))?;
            Ok(())
        } else {
            Err(JsValue::from_str("Event loop proxy not initialized"))
        }
    }

    pub async fn get_pixel_value(&self) -> Result<PixelValue, JsValue> {
        if let Some(proxy) = &self.proxy {
            let (sender, receiver) = futures::channel::oneshot::channel();
            proxy
                .send_event(UserEvent::GetPixel(sender).into())
                .map_err(|e| JsValue::from_str(&format!("Error: {}", e)))?;
            let pixels = receiver
                .await
                .map_err(|e| JsValue::from_str(&format!("Error: {}", e)))?
                .await
                .map_err(|e| JsValue::from_str(&format!("Error: {}", e)))?;
            Ok(pixels)
        } else {
            throw_str("Event loop proxy not initialized");
        }
    }

    pub async fn capture_image(&self) -> Result<Vec<u8>, JsValue> {
        if let Some(proxy) = &self.proxy {
            let (sender, receiver) = futures::channel::oneshot::channel();
            proxy
                .send_event(UserEvent::CaptureImage(sender).into())
                .map_err(|e| JsValue::from_str(&format!("Error: {}", e)))?;
            let image = receiver
                .await
                .map_err(|e| JsValue::from_str(&format!("Error: {}", e)))?
                .await
                .map_err(|e| JsValue::from_str(&format!("Error: {}", e)))?;
            Ok(image)
        } else {
            throw_str("Event loop proxy not initialized");
        }
    }

    pub fn set_height_shader(&self) -> Result<(), JsValue> {
        if let Some(proxy) = &self.proxy {
            proxy
                .send_event(UserEvent::SetHeightShader.into())
                .map_err(|e| e.to_string())?;
            Ok(())
        } else {
            Err(JsValue::from_str("Event loop proxy not initialized"))
        }
    }

    pub fn set_texture_shader(&self) -> Result<(), JsValue> {
        if let Some(proxy) = &self.proxy {
            proxy
                .send_event(UserEvent::SetTextureShader.into())
                .map_err(|e| e.to_string())?;
            Ok(())
        } else {
            Err(JsValue::from_str("Event loop proxy not initialized"))
        }
    }

    pub fn set_overlays(&self, overlays: Vec<Overlay>) -> Result<(), JsValue> {
        if let Some(proxy) = &self.proxy {
            proxy
                .send_event(UserEvent::SetOverlays(Arc::new(overlays)).into())
                .map_err(|e| e.to_string())?;
            Ok(())
        } else {
            Err(JsValue::from_str("Event loop proxy not initialized"))
        }
    }

    pub fn clear_overlays(&self) -> Result<(), JsValue> {
        if let Some(proxy) = &self.proxy {
            proxy
                .send_event(UserEvent::ClearOverlays.into())
                .map_err(|e| e.to_string())?;
            Ok(())
        } else {
            Err(JsValue::from_str("Event loop proxy not initialized"))
        }
    }

    pub fn zoom_in(&self) -> Result<(), JsValue> {
        if let Some(proxy) = &self.proxy {
            proxy
                .send_event(UserEvent::ZoomIn.into())
                .map_err(|e| e.to_string())?;
            Ok(())
        } else {
            Err(JsValue::from_str("Event loop proxy not initialized"))
        }
    }

    pub fn zoom_out(&self) -> Result<(), JsValue> {
        if let Some(proxy) = &self.proxy {
            proxy
                .send_event(UserEvent::ZoomOut.into())
                .map_err(|e| e.to_string())?;
            Ok(())
        } else {
            Err(JsValue::from_str("Event loop proxy not initialized"))
        }
    }

    pub fn set_orientation(&self, orientation: Orientation) -> Result<(), JsValue> {
        if let Some(proxy) = &self.proxy {
            proxy
                .send_event(UserEvent::SetOrientation(orientation).into())
                .map_err(|e| e.to_string())?;
            Ok(())
        } else {
            Err(JsValue::from_str("Event loop proxy not initialized"))
        }
    }

    pub fn reset_orientation(&self) -> Result<(), JsValue> {
        if let Some(proxy) = &self.proxy {
            proxy
                .send_event(UserEvent::ResetOrientation.into())
                .map_err(|e| e.to_string())?;
            Ok(())
        } else {
            Err(JsValue::from_str("Event loop proxy not initialized"))
        }
    }

    pub fn set_percentile(&self, percentile: f32) -> Result<(), JsValue> {
        if let Some(proxy) = &self.proxy {
            proxy
                .send_event(UserEvent::SetPercentile(percentile).into())
                .map_err(|e| e.to_string())?;
            Ok(())
        } else {
            Err(JsValue::from_str("Event loop proxy not initialized"))
        }
    }

    pub fn set_texture_range(&self, start: u16, end: u16) -> Result<(), JsValue> {
        if let Some(proxy) = &self.proxy {
            proxy
                .send_event(UserEvent::SetTextureRange(start, end).into())
                .map_err(|e| e.to_string())?;
            Ok(())
        } else {
            Err(JsValue::from_str("Event loop proxy not initialized"))
        }
    }
}
