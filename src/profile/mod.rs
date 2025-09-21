mod cpu_profiler;
mod gpu_profiler;

pub use cpu_profiler::*;
pub use gpu_profiler::*;

#[cfg(feature = "trace_tracy")]
mod tracy;
#[cfg(feature = "trace_tracy")]
pub use tracy::*;

use crate::{Context, ContextBuilder};

pub struct ProfileContext {
    pub cpu_profiler: ProfilerWrapper,
    pub gpu_profiler: GpuProfiler,

    #[cfg(feature = "trace_tracy")]
    pub tracy: tracy::TracyContext,
}

impl ProfileContext {
    pub fn new(
        context_builder: &ContextBuilder,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Self {
        let gpu_profiler = GpuProfiler::new(
            device,
            context_builder.gpu_profiler_capacity,
            context_builder.gpu_profiler_enabled,
        );
        let cpu_profiler = context_builder.profiler.clone();

        Self {
            cpu_profiler,
            gpu_profiler,

            #[cfg(feature = "trace_tracy")]
            tracy: tracy::TracyContext::new(device, queue),
        }
    }
}

//
// Commands
//

pub fn profiler(ctx: &Context) -> cpu_profiler::ProfilerWrapper {
    ctx.profile.cpu_profiler.clone()
}

pub fn enable_gpu_profiling(ctx: &mut Context, enabled: bool) {
    ctx.profile.gpu_profiler.set_enabled(enabled);
}
pub fn gpu_profiler(ctx: &mut Context) -> &mut GpuProfiler {
    &mut ctx.profile.gpu_profiler
}
