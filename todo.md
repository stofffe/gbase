[] whatevs
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
    [x] encase storage buffers instead of bytemuck
    [x] extend camera to support non aspect ratio shapes
    [x] merge update and render callbacks
    [x] add HDR?
    [x] fix mesh example
    [x] convert notify to notify debounce for reload
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
    [] pingpong frambuffer for post processing
    [] make format of frambuffer a generic thing
    [] make format and or dimension of texture a generic thing
    [] custom offset instance buffers
    [] file system with caching for READ ONLY buffers/textures
    [] make buffer usage more uniform
    [] make screen view a buffer
    [] look into submitting all command encoders at once (probably fine without)
    [] add sinlge pixel images + full screen quad as cached stuff in context?
    [] rework deferred
    [] pbr renderer not caching textures? 
    [] learn about mip mapping
    [] fix super-debug mode (hot reload, asset reload) which is not available for wasm
        #[cfg(all(feature = "super-debug", target_arch = "wasm32"))]
        compile_error!("The 'super-debug' feature is not supported on wasm32");
    [] remove all asset convert unwraps in renderers
    [] convert all util and core to use include_bytes!()
    [] remove framebuffers and replace with normal textures?
    [] look into removing builder pattern from buffers (look at bevy_render/src/uniform_buffer.rs)
    [] use single buffer for multiple vertex attributes?
    [] tracing doesnt work in new() for hot reload
    [] move more things to FxHasher?
        [x] asset cache
        [x] render cache
        [] input?
    [] use more references for asset handles, vectors etc
    [] make camera matrices cached

[] shadows
    [] fade out when reaching limit
    [] pcss
    [x] frustum culling for light cameras
    [x] remove non comparison sampler from mesh.wgsl
    [x] frustum fitting
    [x] cascades
    [x] look into depth bias state 
    [x] check if you actually wanna saturate the pixel or just return non shaded
    [x] compare front+back+bias vs only back faces

[] assets 
    [] add gltf loader
    [] make non loading separate type without unwrap
    [] add imports to shaders
    [] add sub model loading, file.glft#node1
    [] check if asset waiting queus up work (like in pbr)
    [] make uniform/storage/mesh (vertex) grow dynamically
    [] use more references to avoid clone? or use copy?
    [] allow load to error?
    [x] look into putting assets in app and not context

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

[x] tracing
    [x] tracing wasm
    [x] tracy
    [] tracy_client has gpu support?

[] remove vsync, log level, (asset path) from init_ctx()
    some are needed for wasm (log level)

[] gltf
    [] use same texture if metal/rough and occlusion use the same
    [x] have list of required attr and add/remove if necessary
    [] auto generate
        [] normals
        [] tangents
        [x] color

[] post processing
    [] bloom
    [] tonemapping
    [] gaussian blur is most up to date, copy to all other

[x] flappy bird
    [x] rotate bird
    [x] sound
    [x] circle collisions
    [x] highscore
    [x] use entites

[] clouds
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
    [] temporal? 

[] gizmos
    [/] only create unit shapes and apply transform
        - currently taking radius and stuff into account

[] UI
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


[] example template
    [] toml
    [] soft link assets
    [] index
    [] makefile

Hot reload command:
    nmap <c-p> <cmd>silent exec "! cd examples/FOLDER && make hot_reload_compile"<cr

Links
Shadow mapping: https://web.archive.org/web/20230210095515/http://the-witness.net/news/2013/09/shadow-mapping-summary-part-1/
Clay: https://youtu.be/DYWTw19_8r4?si=BgJYoPEBPdVntybD
