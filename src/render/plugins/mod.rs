// This module contains boilerplate code for rendering using wgpu

mod vertex;
pub use vertex::*;

mod camera;
pub use camera::*;

mod transform;
pub use transform::*;

mod depth_buffer;
pub use depth_buffer::*;

mod texture;
pub use texture::*;

mod instance;
pub use instance::*;