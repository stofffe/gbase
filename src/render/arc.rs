// from GGEZ https://github.com/ggez/ggez

use crate::{arc::ArcHandleRuntime, Context};
use std::{any::Any, sync::Arc};

/// Arc'd WGPU handles are used widely across the graphics module.
///
/// Beyond allowing for Clone, they also allow different GPU resources to be
/// unique identified via `id` - primarily used when caching (see the other `gpu` modules).
#[derive(Debug)]
pub struct ArcHandle<T: ?Sized + 'static> {
    pub value: Arc<T>,
    id: u64,
}

impl AsRef<ArcHandleRuntime> for &ArcHandleRuntime {
    fn as_ref(&self) -> &ArcHandleRuntime {
        self
    }
}

impl AsRef<ArcHandleRuntime> for &Context {
    fn as_ref(&self) -> &ArcHandleRuntime {
        &self.arc.runtime
    }
}

impl AsRef<ArcHandleRuntime> for &mut Context {
    fn as_ref(&self) -> &ArcHandleRuntime {
        &self.arc.runtime
    }
}

impl<T: 'static> ArcHandle<T> {
    pub fn new(arc_runtime: impl AsRef<ArcHandleRuntime>, value: T) -> Self {
        ArcHandle {
            value: Arc::new(value),
            id: arc_runtime.as_ref().next_id(),
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
            value: Arc::clone(&self.value),
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
        self.value.as_ref()
    }
}

impl<T: 'static> AsRef<T> for ArcHandle<T> {
    fn as_ref(&self) -> &T {
        self.value.as_ref()
    }
}
// Convert from and to any

impl<T: Any + Send + Sync + 'static> ArcHandle<T> {
    pub fn upcast(self) -> ArcHandle<dyn Any + Send + Sync> {
        ArcHandle {
            value: self.value as Arc<dyn Any + Send + Sync>,
            id: self.id,
        }
    }
}

impl ArcHandle<dyn Any + Send + Sync> {
    pub fn downcast<G: Any + Send + Sync>(&self) -> Option<ArcHandle<G>> {
        if let Ok(handle) = self.value.clone().downcast::<G>() {
            Some(ArcHandle {
                value: handle,
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
