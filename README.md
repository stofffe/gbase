### Gbase

Graphics engine for Rust and WebGPU focusing in simplicity. The main idea of the engine is to have a super simple core that abstracts the 
low level GPU concenpts to work on a wide variety of platforms.

But the repo also contains many utilities which are not a part the core. You will have to create and call these utilities every frame, this 
leads to some boilerplate but it allows for better control of things such as draw order. Some examples of utilities are
- GLB loader and renderer
- Custom immediate UI
- Gizmo renderer
- Sprite rendering
- Much more

#### Features

- Windows/Mac/Linux support
- WASM support
    - See [web example](examples/web)
- Hot reload support
    - Engine handles loading and unloading of DLL
    - See [hot reload example](examples/hot_reload)
- Wrappers over low level features in wgpu
    - No loss of control
    - Heavy use of builder pattern
- Utilities

