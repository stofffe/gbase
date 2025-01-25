Engine
[x] make all functions in callback trait required || check which functions are defined and only load those to hot reload
[x] fix new web build support in examples folder
[x] put hot reload specific imports behind feature flag
[x] change all helper/core filesystem loads to load_b/s?
[x] add view, proj, view_proj + all inverse to camera uniform
[x] pass new size in resize callback /  remove it completely and only use render::resized(ctx) kinda thing
[] use glam in core, input, window...
[] catch wgpu panics
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

[] 2d sphere collision
[] sprite transforms
[] random wrapper (hash/rand)

Post processing
[] bloom
[] tonemapping

Flappy bird
[] highscore
[] rotate bird
[] sound

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
Links:
    Clay https://youtu.be/DYWTw19_8r4?si=BgJYoPEBPdVntybD

Hot reload command:
    nmap <c-p> <cmd>silent exec "! cd examples/hot_reload && make compile"<cr
