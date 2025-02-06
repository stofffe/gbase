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
[x] upgrade to wgpu 24
[] use glam in core, input, window...
[/] catch wgpu panics
    [x] shaders
    [] bindgroups
    [] pipelines
    ...
[/] fix logging not working in dlls
    - have to call init logging in hot reload callback
[] combine full screen post processing into one with uniform args
[] explore drop on commandencoder to not miss submitting
[] hot_reload callback?
[] look into scale factor dpi
[] convert all builder to use lifetimes?
[] feature based derives? #\[cfg_attr = "serde", derive(...)\]
[] move collisions to utils?
[] helper crate with re exported macros
[] fix mesh example
    [] new glb loader
    [] use mega entity instead?
[] add ability to choose gamma corrected or not on surface (currently always choose gamma corrected (srgb))
[] modify framebuffer from main and automatically apply gamma correction if needed (only necessary if format does not support srgb)
[] double frambuffer for post processing
[] make format of frambuffer a generic thing
[] make format and or dimension of texture a generic thing

Post processing
[] bloom
[] tonemapping
[] gaussian blur is most up to date, copy to all other

Flappy bird
[x] rotate bird
[x] sound
[x] circle collisions
[] highscore

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
[] only create unit shapes and apply transform
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
