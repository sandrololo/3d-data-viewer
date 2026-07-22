use crate::SharedFuture;
use anyhow::anyhow;
use futures::FutureExt;
use std::future::Future;
use std::sync::{Arc, Mutex};

pub struct GPUDataReadback<T> {
    buffer: wgpu::Buffer,
    /// Cached shared future - if a read is in progress, subsequent calls get the same future
    pending_read: Arc<Mutex<Option<SharedFuture<Result<T, Arc<anyhow::Error>>>>>>,
}

impl<T: Clone + Send + 'static> GPUDataReadback<T> {
    pub fn new(buffer: wgpu::Buffer) -> Self {
        Self {
            buffer,
            pending_read: Arc::new(Mutex::new(None)),
        }
    }

    pub fn set_buffer(&mut self, buffer: wgpu::Buffer) {
        self.buffer = buffer;
    }

    pub fn get_buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    pub fn has_pending_read(&self) -> bool {
        self.pending_read.lock().unwrap().is_some()
    }

    pub fn get(
        &self,
        device: Arc<wgpu::Device>,
        extract_result: impl Fn(&[u8]) -> Result<T, Arc<anyhow::Error>> + Send + 'static,
    ) -> SharedFuture<Result<T, Arc<anyhow::Error>>> {
        let mut pending = self.pending_read.lock().unwrap();

        // If there's already a pending read, return a clone of it
        if let Some(ref shared) = *pending {
            return shared.clone();
        }

        // Create new read future
        let buffer = self.buffer.clone();
        let pending_read = self.pending_read.clone();
        let (tx, rx) = futures::channel::oneshot::channel::<Result<(), wgpu::BufferAsyncError>>();

        buffer.map_async(wgpu::MapMode::Read, .., move |result| {
            let _ = tx.send(result);
        });

        let future: std::pin::Pin<Box<dyn Future<Output = Result<T, Arc<anyhow::Error>>>>> =
            Box::pin(async move {
                let _ = device.poll(wgpu::PollType::Poll);

                rx.await
                    .map_err(|e| anyhow!("Channel error: {:?}", e))?
                    .map_err(|e| anyhow!("Buffer map error: {:?}", e))?;

                let output_data = buffer.get_mapped_range(..);
                let result = extract_result(&output_data)?;
                drop(output_data);
                buffer.unmap();

                // Clear the pending read so next call starts fresh
                *pending_read
                    .lock()
                    .map_err(|e| anyhow!("Lock error: {:?}", e))? = None;
                Ok(result)
            });

        let shared = future.shared();
        *pending = Some(shared.clone());
        shared
    }
}
