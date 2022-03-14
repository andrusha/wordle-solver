use spirv_builder::SpirvBuilder;
use std::path::Path;

fn main() {
    SpirvBuilder::new(Path::new("shader"), "spirv-unknown-vulkan1.1")
        .build()
        .expect("Shader failed to compile");
}
