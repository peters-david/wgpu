use std::{borrow::Cow, mem};
use wgpu::util::DeviceExt;

use lazy_static::lazy_static;

lazy_static! {
    static ref COMPUTE: Compute = pollster::block_on(Compute::new()).unwrap();
}

struct Compute {
    device: wgpu::Device,
    queue: wgpu::Queue
}

impl Compute {
    async fn new() -> Option<Self> {
        let instance: wgpu::Instance = wgpu::Instance::new(wgpu::Backends::all());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await?;
        let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                features: wgpu::Features::empty(),
                limits: wgpu::Limits::downlevel_defaults(),
            },
            None,
        )
        .await
        .unwrap(); 
        Some(Self {
            device,
            queue
        })
    }
}

async fn use_gpu() -> Vec<u32> {

    let device = &COMPUTE.device;
    let queue = &COMPUTE.queue;

    let load = vec![0; 1*256].into_boxed_slice();
    
    let shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("long_running_shader.wgsl"))),
    });

    let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: None,
        layout: None,
        module: &shader,
        entry_point: "main",
    });

    let cpu_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("cpu buffer"),
        size: mem::size_of::<[u32; 1*256]>() as wgpu::BufferAddress,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let gpu_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("NN Buffer"),
        contents: bytemuck::cast_slice(&load),
        usage: wgpu::BufferUsages::STORAGE
        | wgpu::BufferUsages::COPY_DST
        | wgpu::BufferUsages::COPY_SRC,
    });

    let bind_group_layout = compute_pipeline.get_bind_group_layout(0);

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: gpu_buffer.as_entire_binding(),
        }],
    });

    let mut command_encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

    {
        let mut compute_pass = command_encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: None });
        compute_pass.set_pipeline(&compute_pipeline);
        compute_pass.set_bind_group(0, &bind_group, &[]);
        compute_pass.insert_debug_marker("compute pass");
        compute_pass.dispatch(1, 1, 1); // Number of cells to run, the (x,y,z) size of item being processed
    }

    command_encoder.copy_buffer_to_buffer(&gpu_buffer, 0, &cpu_buffer, 0, mem::size_of::<[u32; 1*256]>() as wgpu::BufferAddress);

    queue.submit(Some(command_encoder.finish()));

    let cpu_buffer_slice = cpu_buffer.slice(..);
    let cpu_buffer_future = cpu_buffer_slice.map_async(wgpu::MapMode::Read);

    device.poll(wgpu::Maintain::Wait);

    if let Ok(()) = cpu_buffer_future.await {
        let result: Vec<u32> = bytemuck::cast_slice(&cpu_buffer_slice.get_mapped_range()).to_vec();
        cpu_buffer.unmap();
        return result
    } else {
        panic!("failed to run compute on gpu!")
    }
}     

#[test]
fn long_running_shader() {
    for i in 0.. { // how often to run? when it occurs doesnt look deterministic to me
        println!("{i}");
        let actual = pollster::block_on(use_gpu());
        for value in actual {
            assert_eq!(100000, value);
        }
    }   
}