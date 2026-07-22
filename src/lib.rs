//! 3D height-map rendering engine.
//!
//! The engine renders into a caller-provided offscreen color texture and is
//! driven through an imperative, window-toolkit-agnostic API ([`Interaction`],
//! [`render::Renderer`]), so it can be embedded in any host — a winit app (see
//! the `data-viewer-3d` binary in this crate) or an egui/eframe canvas sharing
//! an existing wgpu device.

pub mod gpu_data;
pub mod index_buffer;
pub mod interaction;
pub mod mip;
pub mod render;
pub mod scene;
pub mod tiff_decode;
pub mod vertex_buffer;
pub mod view;

pub use interaction::{Interaction, Orientation};
pub use render::Renderer;
pub use scene::{Overlay, Scene};

/// A `Shared` boxed future used to hand GPU readbacks (pixel pick / capture) to callers.
pub type SharedFuture<T> =
    futures::future::Shared<std::pin::Pin<Box<dyn std::future::Future<Output = T>>>>;
