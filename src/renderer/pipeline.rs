use wgpu::{ShaderModuleDescriptor, util::DeviceExt};

pub struct GpuBuffer {
  pub buffer: wgpu::Buffer,
  pub length: u64,
  usages: wgpu::BufferUsages,
}

impl GpuBuffer {
  pub fn new(
    device: &wgpu::Device,
    length: usize,
    usages: wgpu::BufferUsages
  ) -> Self {
    Self {
      buffer: device.create_buffer(&wgpu::wgt::BufferDescriptor {
	label: None,
	size: length as u64,
	usage: usages,
	mapped_at_creation: false
      }),
      length: length as u64,
      usages
    }
  }

  pub fn new_with_data(
    device: &wgpu::Device,
    contents: &[u8],
    usages: wgpu::BufferUsages
  ) -> Self {
    let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
      label: None,
      contents: contents,
      usage: usages,
    });
    
    Self {
      length: buffer.size(),
      buffer,
      usages,
    }
  }

  pub fn recreate(&mut self, device: &wgpu::Device, length: usize) {
    self.buffer.destroy();
    self.buffer = device.create_buffer(&wgpu::wgt::BufferDescriptor {
      label: None,
      size: Self::valid_length(length),
      usage: self.usages,
      mapped_at_creation: false
    });
    self.length = Self::valid_length(length);
  }

  fn valid_length(length: usize) -> u64 {
    let unpadded_size = length as wgpu::BufferAddress;
    let align_mask = wgpu::COPY_BUFFER_ALIGNMENT - 1;
    let padded_size =
      ((unpadded_size + align_mask) & !align_mask).max(wgpu::COPY_BUFFER_ALIGNMENT);

    padded_size
  }

  /// optionally `queue.submit([])` after this 
  pub fn write_data(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, contents: &[u8]) {
    if contents.len() > self.length as usize {
      self.recreate(device, contents.len());
    }
    queue.write_buffer(&self.buffer, 0, contents);
  }
}

pub struct UniformDescriptor {
  pub contents: Vec<u8>,
  pub visibility: wgpu::ShaderStages,
}

pub struct UniformGroupManager {
  buffers: Vec<GpuBuffer>,
  bind_group_layout: wgpu::BindGroupLayout,
  bind_group: wgpu::BindGroup
}

impl UniformGroupManager {
  pub fn new(descriptors: &[UniformDescriptor], device: &wgpu::Device) -> Self {
    let mut buffers = Vec::new();
    for v in descriptors {
      buffers.push(GpuBuffer::new_with_data(
	device,
	&v.contents,
	wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
      ));
    }

    let mut bind_group_layout_entries = Vec::new();

    for (i, v) in descriptors.iter().enumerate() {
      bind_group_layout_entries.push(
	wgpu::BindGroupLayoutEntry {
	  binding: i as u32,
	  visibility: v.visibility,
	  count: None,
	  ty: wgpu::BindingType::Buffer {
	    ty: wgpu::BufferBindingType::Uniform,
	    has_dynamic_offset: false,
	    min_binding_size: None,
	  }
	}
      );
    }

    let bind_group_layout = device.create_bind_group_layout(
      &wgpu::BindGroupLayoutDescriptor {
	label: None,
	entries: bind_group_layout_entries.as_slice(),
      }
    );

    let mut bind_group_entries = Vec::new();
    
    for (i, _v) in descriptors.iter().enumerate() {
      bind_group_entries.push(
	wgpu::BindGroupEntry {
	  binding: i as u32,
	  resource: buffers[i].buffer.as_entire_binding(),
	}
      );
    }

    let bind_group = device.create_bind_group(
      &wgpu::BindGroupDescriptor {
	label: None,
	layout: &bind_group_layout,
	entries: &bind_group_entries,
      }
    );

    Self {
      buffers,
      bind_group,
      bind_group_layout,
    }
  }

  pub fn write(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, buf: &[u8], buf_index: usize) {
    &self.buffers[buf_index].write_data(device, queue, buf);
  }
}

pub struct VertexBufferDescriptor<'a> {
  pub contents: Vec<u8>,
  pub description: wgpu::VertexBufferLayout<'a>,
}

pub struct VertexBuffersDescriptor<'a> {
  pub buffers: Vec<VertexBufferDescriptor<'a>>,
  pub instance_buffer_index: Option<usize>,
  pub instance_buffer_len: Option<usize>,
}

    
pub struct VertexBuffersManager<'a> {
  buffers: Vec<GpuBuffer>,
  descriptions: Vec<wgpu::VertexBufferLayout<'a>>,
  instance_buffer_len: Option<usize>,
  instance_buffer_index: Option<usize>,
}

impl<'a> VertexBuffersManager<'a> {
  pub fn new(descriptors: VertexBuffersDescriptor<'a>, device: &wgpu::Device) -> Self {
    let mut buffers = Vec::new();
    let mut descriptions = Vec::new();
    
    for i in descriptors.buffers {
      buffers.push(GpuBuffer::new_with_data(
	device,
	&i.contents,
	wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST
      ));

      descriptions.push(i.description.clone());
    }

    Self {
      buffers,
      descriptions,
      instance_buffer_index: descriptors.instance_buffer_index,
      instance_buffer_len: descriptors.instance_buffer_len,
    }
  }

  pub fn set_vertex_buffer(&mut self, render_pass: &mut wgpu::RenderPass) {
    for (i, v) in self.buffers.iter().enumerate() {
      render_pass.set_vertex_buffer(i as u32, v.buffer.slice(..));
    }
  }

  pub fn state<'b>(&'b self, module: &'b wgpu::ShaderModule) -> wgpu::VertexState<'b>{
    wgpu::VertexState {
      module,
      entry_point: Some("vs_main"),
      buffers: &self.descriptions[..],
      compilation_options: wgpu::PipelineCompilationOptions::default(),
    }
  }
}

#[derive(Clone)]
pub struct IndexBufferDescriptor {
  pub contents: Vec<u8>,
  pub content_len: usize,
}

pub struct IndexBufferManager {
  buffer: GpuBuffer,
  content_len: usize,
}

impl IndexBufferManager {
  pub fn new(descriptor: IndexBufferDescriptor, device: &wgpu::Device) -> Self {
    let b = GpuBuffer::new_with_data(
      device,
      &descriptor.contents,
      wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::INDEX);
    
    Self {
      buffer: b,
      content_len: descriptor.content_len,
    }
  }
}

pub enum ShaderDataDescriptor<'a> {
  File(&'a str),
  RawData(&'a str),
}

pub struct RenderPipelineManagerDescriptor<'a> {
  pub uniforms: Vec<UniformDescriptor>,
  pub vertex_buffers: VertexBuffersDescriptor<'a>,
  pub instance_buffer_index: Option<usize>,
  pub index_buffer: IndexBufferDescriptor,
  pub shader: ShaderDataDescriptor<'a>,
}

pub struct RenderPipelineManager<'a> {
  pub uniforms: UniformGroupManager,
  pub vertex_buffers: VertexBuffersManager<'a>,
  pub index_buffer: IndexBufferManager,
  pub shader: wgpu::ShaderModule,
  pipeline: wgpu::RenderPipeline,
}

impl<'a> RenderPipelineManager<'a> {
  pub fn new(descriptor: RenderPipelineManagerDescriptor<'a>, device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) -> Self {
    let uniforms = UniformGroupManager::new(&descriptor.uniforms, device);
    let vertex_buffers = VertexBuffersManager::new(descriptor.vertex_buffers, device);
    let index_buffer = IndexBufferManager::new(descriptor.index_buffer, device);
    
    let shader_data = match descriptor.shader {
      ShaderDataDescriptor::File(_) => unimplemented!("Reading shader from the file: Not implemented"),
      ShaderDataDescriptor::RawData(d) => d,
    };
    
    let shader = device.create_shader_module(
      ShaderModuleDescriptor {
	label: Some(shader_data),
	source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(shader_data)),
      }
    );

    let pipeline_layout = device.create_pipeline_layout(
      &wgpu::PipelineLayoutDescriptor {
	label: None,
	bind_group_layouts: &[Some(&uniforms.bind_group_layout)],
	immediate_size: 0,
      }
    );

    let pipeline = device.create_render_pipeline(
      &wgpu::RenderPipelineDescriptor {
	label: None,
	layout: Some(&pipeline_layout),
	vertex: vertex_buffers.state(&shader),
	fragment: Some(wgpu::FragmentState {
	  module: &shader,
	  entry_point: Some("fs_main"),
	  compilation_options: wgpu::PipelineCompilationOptions::default(),
	  targets: &[Some(wgpu::ColorTargetState {
            format: config.format,
            blend: Some(wgpu::BlendState::REPLACE),
            write_mask: wgpu::ColorWrites::ALL,
	  })],
	}),
	primitive: wgpu::PrimitiveState {
	  topology: wgpu::PrimitiveTopology::TriangleList,
          strip_index_format: None,
          front_face: wgpu::FrontFace::Ccw,
          cull_mode: Some(wgpu::Face::Back),
          polygon_mode: wgpu::PolygonMode::Fill,
          unclipped_depth: false,
          conservative: false,
	},
	depth_stencil: Some(wgpu::DepthStencilState {
	  format: wgpu::TextureFormat::Depth32Float,
	  depth_write_enabled: Some(true),
          depth_compare: Some(wgpu::CompareFunction::Less),
          stencil: wgpu::StencilState::default(),
          bias: wgpu::DepthBiasState::default(),
	}),
	multisample: wgpu::MultisampleState {
          count: 1,
          mask: !0,
          alpha_to_coverage_enabled: false,
	},
	multiview_mask: None,
	cache: None,
      }
    );
    
    Self {
      uniforms,
      vertex_buffers,
      index_buffer,
      shader,
      pipeline,
    }
  }

  pub fn draw(&mut self, render_pass: &mut wgpu::RenderPass) {
    render_pass.set_pipeline(&self.pipeline);
    render_pass.set_bind_group(0, &self.uniforms.bind_group, &[]);
    self.vertex_buffers.set_vertex_buffer(render_pass);
    render_pass.set_index_buffer(self.index_buffer.buffer.buffer.slice(..), wgpu::IndexFormat::Uint16);

    let instances_len = self.vertex_buffers.instance_buffer_len.unwrap_or(1);
    render_pass.draw_indexed(0..self.index_buffer.content_len as u32, 0,
			     0..instances_len as u32);
  }
}
