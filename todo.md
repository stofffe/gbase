Engine
[x] make all functions in callback trait required || check which functions are defined and only load those to hot reload
[x] fix new web build support in examples folder
[x] put hot reload specific imports behind feature flag
[x] change all helper/core filesystem loads to load_b/s?
[x] add view, proj, view_proj + all inverse to camera uniform
[x] pass new size in resize callback /  remove it completely and only use render::resized(ctx) kinda thing
[x] remove texture caching
[x] 2d sphere collision
[x] sprite transforms
[x] random wrapper (hash/rand)
[x] add gamma correction by default?
[x] hot_reload callback?
[x] upgrade to wgpu 24
[x] remove average fps from time module?
[x] add proper BRDF to pbr
    [x] add HDR?
[x] fix mesh example
    [x] new glb loader
[/] catch wgpu panics
    [x] shaders
    [] bindgroups
    [] pipelines
    ...
[/] fix logging not working in dlls
    - have to call init logging in hot reload callback
[] use glam in core, input, window...
[] combine full screen post processing into one with uniform args
[] explore drop on commandencoder to not miss submitting
[] look into scale factor dpi
[] convert all builder to use lifetimes? (cant cache with lifetimes)
[] feature based derives? #\[cfg_attr = "serde", derive(...)\]
[] move collisions to utils?
[] helper crate with re exported macros
[] add ability to choose gamma corrected or not on surface (currently always choose gamma corrected (srgb))
[] modify framebuffer from main and automatically apply gamma correction if needed (only necessary if format does not support srgb)
[] double frambuffer for post processing
[] make format of frambuffer a generic thing
[] make format and or dimension of texture a generic thing
[] custom offset instance buffers
[] file system with caching for READ ONLY buffers/textures
[] make buffer usage more uniform
[] make screen view a buffer
[] look into submitting all command encoders at once (probably fine without)
[] add sinlge pixel images + full screen quad as cached stuff in context?
[] rework deferred
<!-- [] add ability to reuse encoders for all renderers -->
[] pbr renderer not caching textures? 
[] learn about mip mapping
[x] convert notify to notify debounce for reload
[] fix super-debug mode (hot reload, asset reload) which is not available for wasm
    #[cfg(all(feature = "super-debug", target_arch = "wasm32"))]
    compile_error!("The 'super-debug' feature is not supported on wasm32");
[] remove all asset convert unwraps in renderers
[] convert all util and core to use include_bytes!()
[] extend camera to support non aspect ratio shapes
[] remove framebuffers and replace with normal textures?
[] encase storage buffers instead of bytemuck

[] shadows
    [] fade out when reaching limit
    [] pcss
    [] remove non comparison sampler from mesh.wgsl
    [] frustum culling for light cameras
    [x] frustum fitting
    [x] cascades
    [x] look into depth bias state 
    [x] check if you actually wanna saturate the pixel or just return non shaded
    [x] compare front+back+bias vs only back faces

[] assets 
    [] add imports to shaders
    [] add gltf loader
    [] add sub model loading, file.glft#node1
    [] check if asset waiting queus up work (like in pbr)

[] bloom
    [] move pixel cache to ctx
    [] add temp buffer creation stuff, pooling
    [] replace extract with karis
    [] combine last upsample and combine shaders
    [] add threshold/blur radius params
    [] use single triangle (https://wallisc.github.io/rendering/2021/04/18/Fullscreen-Pass.html)

[] profiling
    [x] cpu with mutex?
    [x] put profiling in ctx
    [] async timestamp query readback?
    [] connect cpu to tracing?
    [] separate gpu and cpu profiling
[x] move to tracing
    [x] tracing wasm
    [x] tracy
[] remove vsync, log level, (asset path) from init_ctx()
    some are needed for wasm (log level)

Gltf
[] use same texture if metal/rough and occlusion use the same
[x] have list of required attr and add/remove if necessary
[] auto generate
    [] normals
    [] tangents
    [x] color

Post processing
[] bloom
[] tonemapping
[] gaussian blur is most up to date, copy to all other

Flappy bird
[x] rotate bird
[x] sound
[x] circle collisions
[x] highscore
[x] use entites

Clouds
[x] ray box intersection
[x] light march
[x] add colored clouds / light
[x] octave scattering
[x] blue noise
[x] animate with time
[x] write screen pixels to png
[x] change resolution
[x] blur 
[/] powder effect
    - adding it instead of multiplying
[] directional light instead of point light
[] temporal? (no) 

Links
    
Gizmos
[/] only create unit shapes and apply transform
    - currently taking radius and stuff into account

UI
[x] implement axis
[x] padding
[x] margin
[x] gap
[x] fit content
[x] use width and height for sizing instead?
[x] child alignment
[x] use pixels instead of [0,1]
[x] sliders input boxes
[x] text based size 
[] padding on textsize?
[] solve violations
[] text alignment
[] bring back UiID
[] text input boxes
[] number input boxes
[] scroll
[] 4-way padding/margin?
[] min/max size for elements?
[] rounded corners
[] (){} weird sizes
Clay: https://youtu.be/DYWTw19_8r4?si=BgJYoPEBPdVntybD

Example template
[] toml
[] soft link assets
[] index
[] makefile

Hot reload command:
    nmap <c-p> <cmd>silent exec "! cd examples/hot_reload && make compile"<cr

Idea for vertex buffer
Assign id to cache wgpu::Buffers
Use single buffer for multiple vertex attributes

Asset vs Render type

Asset<From, To> {
    data T,
    changed: bool,
    source: path | bytes
}

Asset
    Image
        Texture builder
        Sampler builder
    Mesh
    Font
    Audio
    Cpu Shader
        ShaderBuilder

Render type
    Buffers
    Pipelines
    Bindgroup/Bindgroup layout
    Texture
    Sampler
    Texture view
    Shader
