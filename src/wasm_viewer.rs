use std::sync::Arc;
use wasm_bindgen::{JsValue, prelude::*};
use winit::event_loop::EventLoop;

use crate::{
    ImageViewer3D,
    events::{Event, UserEvent},
    gpu_data::pixel_picker::PixelValue,
    interaction::Orientation,
    render::pipeline::FragmentShaderVariant,
    scene::Overlay,
    tiff_decode::decode_tiff,
};

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
        self.send_event(UserEvent::ResetView)
    }

    pub async fn set_topology(&self, data: Vec<u8>) -> Result<(), JsValue> {
        let image = decode_tiff::<f32, _>(std::io::Cursor::new(&data))
            .map_err(|e| JsValue::from_str(&format!("Error: {}", e)))?;
        self.send_event(UserEvent::SetTopology(image))
    }

    pub async fn set_texture(&self, data: Vec<u8>) -> Result<(), JsValue> {
        let image = decode_tiff::<u16, _>(std::io::Cursor::new(&data))
            .map_err(|e| JsValue::from_str(&format!("Error: {}", e)))?;
        self.send_event(UserEvent::SetTexture(image))
    }

    pub async fn get_pixel_value(&self) -> Result<PixelValue, JsValue> {
        let (sender, receiver) = futures::channel::oneshot::channel();
        self.send_event(UserEvent::GetPixel(sender))?;
        let pixels = receiver
            .await
            .map_err(|e| JsValue::from_str(&format!("Error: {}", e)))?
            .await
            .map_err(|e| JsValue::from_str(&format!("Error: {}", e)))?;
        Ok(pixels)
    }

    pub async fn capture_image(&self) -> Result<Vec<u8>, JsValue> {
        let (sender, receiver) = futures::channel::oneshot::channel();
        self.send_event(UserEvent::CaptureImage(sender))?;
        let image = receiver
            .await
            .map_err(|e| JsValue::from_str(&format!("Error: {}", e)))?
            .await
            .map_err(|e| JsValue::from_str(&format!("Error: {}", e)))?;
        Ok(image)
    }

    pub fn set_height_shader(&self) -> Result<(), JsValue> {
        self.send_event(UserEvent::SetFragmentShader(FragmentShaderVariant::Height))
    }

    pub fn set_texture_shader(&self) -> Result<(), JsValue> {
        self.send_event(UserEvent::SetFragmentShader(FragmentShaderVariant::Texture))
    }

    pub fn set_turbo_colormap_shader(&self) -> Result<(), JsValue> {
        self.send_event(UserEvent::SetFragmentShader(
            FragmentShaderVariant::TurboColormap,
        ))
    }

    pub fn set_overlays(&self, overlays: Vec<Overlay>) -> Result<(), JsValue> {
        self.send_event(UserEvent::SetOverlays(Arc::new(overlays)))
    }

    pub fn clear_overlays(&self) -> Result<(), JsValue> {
        self.send_event(UserEvent::ClearOverlays)
    }

    pub fn zoom_in(&self) -> Result<(), JsValue> {
        self.send_event(UserEvent::ZoomIn)
    }

    pub fn zoom_out(&self) -> Result<(), JsValue> {
        self.send_event(UserEvent::ZoomOut)
    }

    pub fn set_orientation(&self, orientation: Orientation) -> Result<(), JsValue> {
        self.send_event(UserEvent::SetOrientation(orientation))
    }

    pub fn reset_orientation(&self) -> Result<(), JsValue> {
        self.send_event(UserEvent::ResetOrientation)
    }

    pub fn set_percentile(&self, percentile: f32) -> Result<(), JsValue> {
        self.send_event(UserEvent::SetPercentile(percentile))
    }

    pub fn set_texture_range(&self, start: u16, end: u16) -> Result<(), JsValue> {
        self.send_event(UserEvent::SetTextureRange(start, end))
    }

    pub fn display_grid(&self, visible: bool) -> Result<(), JsValue> {
        self.send_event(UserEvent::DisplayGrid(visible))
    }

    fn send_event(&self, event: UserEvent) -> Result<(), JsValue> {
        if let Some(proxy) = &self.proxy {
            proxy.send_event(event.into()).map_err(|e| e.to_string())?;
            Ok(())
        } else {
            Err(JsValue::from_str("Event loop proxy not initialized"))
        }
    }
}
