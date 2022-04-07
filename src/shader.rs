use wgpu::util::DeviceExt;

use crate::{Patterns, Word, WORD_COUNT};
use std::{convert::TryInto, num::NonZeroU64};
use wgpu::{BufferAsyncError, Device, Queue, RequestDeviceError, ShaderModule};

async fn init_device() -> Result<(Device, Queue), RequestDeviceError> {
    let instance = wgpu::Instance::new(wgpu::Backends::PRIMARY);
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: None,
        })
        .await
        .expect("Failed to find an appropriate adapter");

    adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                features: wgpu::Features::TIMESTAMP_QUERY
                    | wgpu::Features::SPIRV_SHADER_PASSTHROUGH,
                limits: wgpu::Limits::default(),
            },
            None,
        )
        .await
}

fn load_shader_module(device: &Device) -> ShaderModule {
    let shader_bytes: &[u8] = include_bytes!(env!("wordle_shader.spv"));
    let spirv = std::borrow::Cow::Owned(wgpu::util::make_spirv_raw(shader_bytes).into_owned());
    let shader_binary = wgpu::ShaderModuleDescriptorSpirV {
        label: None,
        source: spirv,
    };

    // Load the shaders from disk
    unsafe { device.create_shader_module_spirv(&shader_binary) }
}

pub async fn match_freq(
    words: &[Word],
    patterns: &[Patterns],
) -> Result<Vec<f32>, BufferAsyncError> {
    const RESULT_BUFFER_SIZE: usize = WORD_COUNT * 4; // f32 = u8 * 4
    let (device, queue) = init_device().await.expect("Failed to create device");
    let module = load_shader_module(&device);

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[
            // XXX - some graphics cards do not support empty bind layout groups, so
            // create a dummy entry.
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                count: None,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    has_dynamic_offset: false,
                    min_binding_size: Some(NonZeroU64::new(1).unwrap()),
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                },
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                count: None,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    has_dynamic_offset: false,
                    min_binding_size: Some(NonZeroU64::new(1).unwrap()),
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                },
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                count: None,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    has_dynamic_offset: false,
                    min_binding_size: Some(NonZeroU64::new(1).unwrap()),
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                },
            },
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: None,
        layout: Some(&pipeline_layout),
        module: &module,
        entry_point: "main",
    });

    let readback_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: RESULT_BUFFER_SIZE as wgpu::BufferAddress,
        // Can be read to the CPU, and can be copied from the shader's storage buffer
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let result_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Word Entropies"),
        contents: &[0; RESULT_BUFFER_SIZE],
        usage: wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_DST
            | wgpu::BufferUsages::COPY_SRC,
    });

    let words_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Words"),
        contents: bytemuck::cast_slice(words),
        usage: wgpu::BufferUsages::STORAGE,
    });

    let patterns_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Patterns"),
        contents: bytemuck::cast_slice(patterns),
        usage: wgpu::BufferUsages::STORAGE,
    });

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: result_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: words_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: patterns_buffer.as_entire_binding(),
            },
        ],
    });

    let mut encoder =
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

    {
        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: None });
        cpass.set_bind_group(0, &bind_group, &[]);
        cpass.set_pipeline(&compute_pipeline);
        cpass.dispatch(WORD_COUNT as u32 / 64, 1, 1);
    }

    encoder.copy_buffer_to_buffer(
        &result_buffer,
        0,
        &readback_buffer,
        0,
        RESULT_BUFFER_SIZE as wgpu::BufferAddress,
    );

    queue.submit(Some(encoder.finish()));
    let buffer_slice = readback_buffer.slice(..);
    let buffer_future = buffer_slice.map_async(wgpu::MapMode::Read);
    device.poll(wgpu::Maintain::Wait);

    buffer_future.await.map(|_| {
        buffer_slice
            .get_mapped_range()
            .chunks_exact(4)
            .map(|b| f32::from_ne_bytes(b.try_into().unwrap()))
            .collect::<Vec<_>>()
    })
}
