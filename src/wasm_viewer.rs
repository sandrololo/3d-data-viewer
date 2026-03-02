use std::sync::Arc;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
use winit::event_loop::EventLoop;

use crate::{
    ImageViewer3D, image::Image, pixel_picker::PixelValue, texture::Overlay, user_events::UserEvent,
};

#[cfg(target_arch = "wasm32")]
mod wasm_commands {
    use std::cell::RefCell;
    use std::sync::Arc;
    use winit::window::Window;

    thread_local! {
        /// Reference to the window for requesting redraws
        pub static WINDOW: RefCell<Option<Arc<Window>>> = RefCell::new(None);
    }

    pub fn set_window(window: Arc<Window>) {
        WINDOW.with(|w| *w.borrow_mut() = Some(window));
    }
}

#[wasm_bindgen]
pub struct WasmViewer {
    proxy: Option<winit::event_loop::EventLoopProxy<UserEvent>>,
    canvas_id: String,
}

#[wasm_bindgen]
impl WasmViewer {
    pub fn new(canvas_id: String) -> Result<Self, wasm_bindgen::JsValue> {
        Ok(Self {
            proxy: None,
            canvas_id,
        })
    }

    pub fn run(&mut self) -> Result<(), wasm_bindgen::JsValue> {
        console_log::init_with_level(log::Level::Info).map_err(|e| {
            wasm_bindgen::JsValue::from_str(&format!("Error initializing console_log: {}", e))
        })?;
        console_error_panic_hook::set_once();

        let event_loop = EventLoop::with_user_event().build().map_err(|e| {
            wasm_bindgen::JsValue::from_str(&format!("Error building event loop: {}", e))
        })?;
        self.proxy = Some(event_loop.create_proxy());
        let canvas_id = self.canvas_id.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let mut app = ImageViewer3D::new(&event_loop, canvas_id);
            event_loop
                .run_app(&mut app)
                .map_err(|e| {
                    wasm_bindgen::JsValue::from_str(&format!("Error running event loop: {}", e))
                })
                .unwrap_throw();
        });
        Ok(())
    }

    pub async fn reset_view(&self) -> Result<(), wasm_bindgen::JsValue> {
        if let Some(proxy) = &self.proxy {
            proxy
                .send_event(UserEvent::ResetView)
                .map_err(|e| wasm_bindgen::JsValue::from_str(&format!("Error: {}", e)))?;
            Ok(())
        } else {
            Err(wasm_bindgen::JsValue::from_str(
                "Event loop proxy not initialized",
            ))
        }
    }

    pub async fn set_surface(&self, data: Vec<u8>) -> Result<(), wasm_bindgen::JsValue> {
        if let Some(proxy) = &self.proxy {
            let image = Image::<f32>::try_from(data)
                .map_err(|e| wasm_bindgen::JsValue::from_str(&format!("Error: {}", e)))?;
            proxy
                .send_event(UserEvent::SetSurface(image))
                .map_err(|e| wasm_bindgen::JsValue::from_str(&format!("Error: {}", e)))?;
            Ok(())
        } else {
            Err(wasm_bindgen::JsValue::from_str(
                "Event loop proxy not initialized",
            ))
        }
    }

    pub async fn set_amplitude(&self, data: Vec<u8>) -> Result<(), wasm_bindgen::JsValue> {
        if let Some(proxy) = &self.proxy {
            let image = Image::<u16>::try_from(data)
                .map_err(|e| wasm_bindgen::JsValue::from_str(&format!("Error: {}", e)))?;
            proxy
                .send_event(UserEvent::SetAmplitude(image))
                .map_err(|e| wasm_bindgen::JsValue::from_str(&format!("Error: {}", e)))?;
            Ok(())
        } else {
            Err(wasm_bindgen::JsValue::from_str(
                "Event loop proxy not initialized",
            ))
        }
    }

    pub async fn get_pixel_value(&self) -> Result<PixelValue, wasm_bindgen::JsValue> {
        if let Some(proxy) = &self.proxy {
            let (sender, receiver) = futures::channel::oneshot::channel();
            proxy
                .send_event(UserEvent::GetPixel(sender))
                .map_err(|e| wasm_bindgen::JsValue::from_str(&format!("Error: {}", e)))?;
            let pixels = receiver
                .await
                .map_err(|e| wasm_bindgen::JsValue::from_str(&format!("Error: {}", e)))?
                .await
                .map_err(|e| wasm_bindgen::JsValue::from_str(&format!("Error: {}", e)))?;
            Ok(pixels)
        } else {
            wasm_bindgen::throw_str("Event loop proxy not initialized");
        }
    }

    pub fn set_height_shader(&self) -> Result<(), wasm_bindgen::JsValue> {
        if let Some(proxy) = &self.proxy {
            proxy
                .send_event(UserEvent::SetHeightShader)
                .map_err(|e| e.to_string())?;
            Ok(())
        } else {
            Err(wasm_bindgen::JsValue::from_str(
                "Event loop proxy not initialized",
            ))
        }
    }

    pub fn set_amplitude_shader(&self) -> Result<(), wasm_bindgen::JsValue> {
        if let Some(proxy) = &self.proxy {
            proxy
                .send_event(UserEvent::SetAmplitudeShader)
                .map_err(|e| e.to_string())?;
            Ok(())
        } else {
            Err(wasm_bindgen::JsValue::from_str(
                "Event loop proxy not initialized",
            ))
        }
    }

    pub fn set_overlays(&self, overlays: Vec<Overlay>) -> Result<(), wasm_bindgen::JsValue> {
        if let Some(proxy) = &self.proxy {
            proxy
                .send_event(UserEvent::SetOverlays(Arc::new(overlays)))
                .map_err(|e| e.to_string())?;
            Ok(())
        } else {
            Err(wasm_bindgen::JsValue::from_str(
                "Event loop proxy not initialized",
            ))
        }
    }

    pub fn clear_overlays(&self) -> Result<(), wasm_bindgen::JsValue> {
        if let Some(proxy) = &self.proxy {
            proxy
                .send_event(UserEvent::ClearOverlays)
                .map_err(|e| e.to_string())?;
            Ok(())
        } else {
            Err(wasm_bindgen::JsValue::from_str(
                "Event loop proxy not initialized",
            ))
        }
    }

    pub fn zoom_in(&self) -> Result<(), wasm_bindgen::JsValue> {
        if let Some(proxy) = &self.proxy {
            proxy
                .send_event(UserEvent::ZoomIn)
                .map_err(|e| e.to_string())?;
            Ok(())
        } else {
            Err(wasm_bindgen::JsValue::from_str(
                "Event loop proxy not initialized",
            ))
        }
    }

    pub fn zoom_out(&self) -> Result<(), wasm_bindgen::JsValue> {
        if let Some(proxy) = &self.proxy {
            proxy
                .send_event(UserEvent::ZoomOut)
                .map_err(|e| e.to_string())?;
            Ok(())
        } else {
            Err(wasm_bindgen::JsValue::from_str(
                "Event loop proxy not initialized",
            ))
        }
    }

    pub fn reset_orientation(&self) -> Result<(), wasm_bindgen::JsValue> {
        if let Some(proxy) = &self.proxy {
            proxy
                .send_event(UserEvent::ResetOrientation)
                .map_err(|e| e.to_string())?;
            Ok(())
        } else {
            Err(wasm_bindgen::JsValue::from_str(
                "Event loop proxy not initialized",
            ))
        }
    }

    pub fn set_percentile(&self, percentile: f32) -> Result<(), wasm_bindgen::JsValue> {
        if let Some(proxy) = &self.proxy {
            proxy
                .send_event(UserEvent::SetPercentile(percentile))
                .map_err(|e| e.to_string())?;
            Ok(())
        } else {
            Err(wasm_bindgen::JsValue::from_str(
                "Event loop proxy not initialized",
            ))
        }
    }

    pub fn set_amplitude_range(&self, start: u16, end: u16) -> Result<(), wasm_bindgen::JsValue> {
        if let Some(proxy) = &self.proxy {
            proxy
                .send_event(UserEvent::SetAmplitudeRange(start, end))
                .map_err(|e| e.to_string())?;
            Ok(())
        } else {
            Err(wasm_bindgen::JsValue::from_str(
                "Event loop proxy not initialized",
            ))
        }
    }
}
