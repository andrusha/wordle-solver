# Wordle Solver

First pass of the information optimal [Wordle](https://www.nytimes.com/games/wordle/index.html) solver, implementation of the [3Blue1Brown - Solving Wordle using information theory](https://www.youtube.com/watch?v=v68zYyaEmEA) video.

## Requirements
- [MoltenVK](https://moltengl.com/moltenvk/) if you plan to run GPU implementation on macOS Metal.

## Implementations

* CPU - uses bitpacking to reduce the number of operations for checking for the letter presence, exact positional letter matches, etc
* SIMD - uses [portable-simd](https://github.com/rust-lang/portable-simd) to bundle bitpacked operations together to be executed in parallel, depending on your CPU architecture might result in significant speedup.
* GPU - uses [rust-gpu](https://github.com/EmbarkStudios/rust-gpu/) to cross-compile Rust into [SPIR-V](https://www.khronos.org/registry/SPIR-V/specs/unified1/SPIRV.html#_introduction) shader, which is then executed by [wgpu](https://github.com/gfx-rs/wgpu) through Vulkan.