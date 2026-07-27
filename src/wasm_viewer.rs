use std::{num::NonZeroU32, ops::Deref, sync::Arc};

use data_viewer_3d::{
    interaction::Orientation,
    render::pipeline::FragmentShaderVariant,
    scene::Region,
    tiff_decode::decode_tiff,
    view::{projection::Translation, transformation::EulerRotationDeg},
};
use imask::{NonZeroRange, RangeUnchecked, WithBounds};
use wasm_bindgen::{JsValue, prelude::*};
use winit::event_loop::EventLoop;

use crate::{
    ImageViewer3D,
    events::{Event, UserEvent},
};

#[wasm_bindgen]
pub struct WasmBindgenPixelRange(NonZeroRange<u64>);

#[wasm_bindgen]
impl WasmBindgenPixelRange {
    pub fn new(start: u64, end: u64) -> Result<Self, JsValue> {
        let range = NonZeroRange::try_from(RangeUnchecked { start, end }).map_err(|e| {
            JsValue::from_str(&format!("Error initializing WasmBindgenPixelRange: {}", e))
        })?;
        Ok(Self(range))
    }
}

impl Deref for WasmBindgenPixelRange {
    type Target = NonZeroRange<u64>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[wasm_bindgen]
#[derive(Copy, Clone, Debug)]
pub struct RegionColor([u8; 4]);

#[wasm_bindgen]
impl RegionColor {
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self([r, g, b, a])
    }
}

#[wasm_bindgen(js_name = Region)]
pub struct WasmRegion {
    pixelrange: Vec<WasmBindgenPixelRange>,
    color: RegionColor,
    image_width: u32,
    image_height: u32,
}

#[wasm_bindgen(js_class = Region)]
impl WasmRegion {
    pub fn new(
        pixelrange: Vec<WasmBindgenPixelRange>,
        color: RegionColor,
        image_width: u32,
        image_height: u32,
    ) -> Self {
        Self {
            pixelrange,
            color,
            image_width,
            image_height,
        }
    }
}

impl TryFrom<WasmRegion> for Region {
    type Error = JsValue;

    fn try_from(region: WasmRegion) -> Result<Self, JsValue> {
        let width = NonZeroU32::new(region.image_width)
            .ok_or_else(|| JsValue::from_str("image_width must be non-zero"))?;
        let height = NonZeroU32::new(region.image_height)
            .ok_or_else(|| JsValue::from_str("image_height must be non-zero"))?;

        Region::new(
            WithBounds::new(region.pixelrange.into_iter().map(|r| *r), width, height),
            region.color.0,
        )
        .map_err(|e| JsValue::from_str(&format!("Error converting WasmRegion to Region: {}", e)))
    }
}

#[wasm_bindgen(js_name = Orientation)]
#[derive(Copy, Clone)]
pub struct WasmOrientation {
    zoom: f32,
    translation_x: f32,
    translation_y: f32,
    pitch: f32,
    yaw: f32,
    roll: f32,
}

#[wasm_bindgen(js_class = Orientation)]
impl WasmOrientation {
    pub fn new(zoom: f32, x: f32, y: f32, pitch: f32, yaw: f32, roll: f32) -> Self {
        Self {
            zoom,
            translation_x: x,
            translation_y: y,
            pitch,
            yaw,
            roll,
        }
    }
}

impl From<WasmOrientation> for Orientation {
    fn from(o: WasmOrientation) -> Self {
        Orientation::new(
            o.zoom,
            Translation::new(o.translation_x, o.translation_y),
            EulerRotationDeg::new(o.pitch, o.yaw, o.roll),
        )
    }
}

#[wasm_bindgen(js_name = PixelValue)]
#[derive(Copy, Clone)]
pub struct WasmPixelValue {
    pub x: u32,
    pub y: u32,
    pub z: f32,
    pub texture: u16,
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
        self.send_event(UserEvent::ResetView)
    }

    pub async fn set_topology(&self, data: Vec<u8>) -> Result<(), JsValue> {
        let image = decode_tiff::<f32, _>(std::io::Cursor::new(&data))
            .map_err(|e| JsValue::from_str(&format!("Error: {}", e)))?;
        self.send_event(UserEvent::SetTopology(image))
    }

    /// Sets a topology with a validity mask. The mask is a flat array of bytes (0=invalid, 1=valid)
    /// with the same dimensions (width*height) as the topology image.
    /// Invalid pixels will create holes in the rendered mesh.
    pub async fn set_topology_masked(&self, data: Vec<u8>, mask: Vec<u8>) -> Result<(), JsValue> {
        let image = decode_tiff::<f32, _>(std::io::Cursor::new(&data))
            .map_err(|e| JsValue::from_str(&format!("Error: {}", e)))?;
        let expected_len = (image.width().get() * image.height().get()) as usize;
        if mask.len() != expected_len {
            return Err(JsValue::from_str(&format!(
                "Mask length ({}) does not match image dimensions ({}x{} = {})",
                mask.len(),
                image.width().get(),
                image.height().get(),
                expected_len,
            )));
        }
        self.send_event(UserEvent::SetTopologyMasked(image, mask))
    }

    pub async fn set_texture(&self, data: Vec<u8>) -> Result<(), JsValue> {
        let image = decode_tiff::<u16, _>(std::io::Cursor::new(&data))
            .map_err(|e| JsValue::from_str(&format!("Error: {}", e)))?;
        self.send_event(UserEvent::SetTexture(image))
    }

    pub async fn get_pixel_value(&self) -> Result<WasmPixelValue, JsValue> {
        let (sender, receiver) = futures::channel::oneshot::channel();
        self.send_event(UserEvent::GetPixel(sender))?;
        let pixel = receiver
            .await
            .map_err(|e| JsValue::from_str(&format!("Error: {}", e)))?
            .await
            .map_err(|e| JsValue::from_str(&format!("Error: {}", e)))?;
        Ok(WasmPixelValue {
            x: pixel.x,
            y: pixel.y,
            z: pixel.z,
            texture: pixel.texture,
        })
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

    pub fn set_overlays(&self, overlays: Vec<WasmRegion>) -> Result<(), JsValue> {
        let overlays = overlays
            .into_iter()
            .map(|r| Region::try_from(r))
            .collect::<Result<Vec<_>, _>>()?;
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

    pub fn set_orientation(&self, orientation: WasmOrientation) -> Result<(), JsValue> {
        self.send_event(UserEvent::SetOrientation(orientation.into()))
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

    pub fn set_mip_override(&self, level: Option<u32>) -> Result<(), JsValue> {
        self.send_event(UserEvent::SetMipOverride(level))
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
