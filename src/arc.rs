use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

use crate::Context;

pub(crate) struct ArcContext {
    pub runtime: ArcHandleRuntime,
}
impl ArcContext {
    pub(crate) fn new() -> Self {
        let runtime = ArcHandleRuntime::new();
        Self { runtime }
    }

    pub(crate) fn runtime(&self) -> ArcHandleRuntime {
        self.runtime.clone()
    }
}

#[derive(Clone)]
pub struct ArcHandleRuntime {
    id: Arc<AtomicU64>,
}

impl ArcHandleRuntime {
    pub(crate) fn new() -> Self {
        Self {
            id: Arc::new(AtomicU64::new(0)),
        }
    }

    pub(crate) fn next_id(&self) -> u64 {
        self.id.fetch_add(1, Ordering::Relaxed)
    }
}

//
// Commands
//

pub fn runtime(ctx: &Context) -> &ArcHandleRuntime {
    &ctx.arc.runtime
}
