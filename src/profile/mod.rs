mod profiler;
use crate::{Context, ContextBuilder};
use profiler::GpuProfiler;

pub struct ProfileContext {
    pub gpu_profiler: GpuProfiler,
}

impl ProfileContext {
    pub fn new(context_builder: &ContextBuilder, device: &wgpu::Device) -> Self {
        let gpu_profiler = GpuProfiler::new(
            device,
            context_builder.gpu_profiler_capacity,
            context_builder.gpu_profiler_enabled,
        );
        Self { gpu_profiler }
    }
}

//
// Commands
//

pub fn enable_gpu_profiling(ctx: &mut Context, enabled: bool) {
    ctx.profile.gpu_profiler.set_enabled(enabled);
}
pub fn gpu_profiler(ctx: &mut Context) -> &mut GpuProfiler {
    &mut ctx.profile.gpu_profiler
}
