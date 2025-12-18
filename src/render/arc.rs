// from GGEZ https://github.com/ggez/ggez

use std::{any::Any, sync::Arc};

/// Arc'd WGPU handles are used widely across the graphics module.
///
/// Beyond allowing for Clone, they also allow different GPU resources to be
/// unique identified via `id` - primarily used when caching (see the other `gpu` modules).
#[derive(Debug)]
pub struct ArcHandle<T: ?Sized + 'static> {
    pub handle: Arc<T>,
    id: u64,
}

impl<T: 'static> ArcHandle<T> {
    // TODO: maybe take ctx directly instead?
    pub fn new(id: u64, handle: T) -> Self {
        ArcHandle {
            handle: Arc::new(handle),
            id,
        }
    }

    #[inline]
    pub fn id(&self) -> u64 {
        self.id
    }
}

impl<T: ?Sized + 'static> Clone for ArcHandle<T> {
    fn clone(&self) -> Self {
        ArcHandle {
            handle: Arc::clone(&self.handle),
            id: self.id,
        }
    }
}

impl<T: 'static> PartialEq for ArcHandle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<T: 'static> Eq for ArcHandle<T> {}

impl<T: 'static> std::hash::Hash for ArcHandle<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl<T: 'static> std::ops::Deref for ArcHandle<T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.handle.as_ref()
    }
}

impl<T: 'static> AsRef<T> for ArcHandle<T> {
    fn as_ref(&self) -> &T {
        self.handle.as_ref()
    }
}
// Convert from and to any

impl<T: Any + 'static> ArcHandle<T> {
    pub fn upcast(self) -> ArcHandle<dyn Any> {
        ArcHandle {
            handle: self.handle as Arc<dyn Any>,
            id: self.id,
        }
    }
}

impl ArcHandle<dyn Any> {
    pub fn downcast<G: Clone>(&self) -> Option<ArcHandle<G>> {
        if let Some(handle) = self.handle.clone().downcast_ref::<G>() {
            Some(ArcHandle {
                handle: handle.clone().into(),
                id: self.id,
            })
        } else {
            tracing::error!("could not downcast handle");
            None
        }
    }
}

pub type ArcBuffer = ArcHandle<wgpu::Buffer>;
pub type ArcTexture = ArcHandle<wgpu::Texture>;
pub type ArcTextureView = ArcHandle<wgpu::TextureView>;
pub type ArcBindGroupLayout = ArcHandle<wgpu::BindGroupLayout>;
pub type ArcBindGroup = ArcHandle<wgpu::BindGroup>;
pub type ArcPipelineLayout = ArcHandle<wgpu::PipelineLayout>;
pub type ArcRenderPipeline = ArcHandle<wgpu::RenderPipeline>;
pub type ArcComputePipeline = ArcHandle<wgpu::ComputePipeline>;
pub type ArcSampler = ArcHandle<wgpu::Sampler>;
pub type ArcShaderModule = ArcHandle<wgpu::ShaderModule>;
