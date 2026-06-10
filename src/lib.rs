use std::{collections::HashMap, sync::Arc};
use cgmath::{InnerSpace, Matrix3, Point3, Rad, Vector3, Zero};
use wgpu::util::DeviceExt;

mod fps;
use fps::Fps;

use winit::{
  application::ApplicationHandler,
  event::*,
  event_loop::{ActiveEventLoop, EventLoop},
  keyboard::{KeyCode, PhysicalKey},
  window::Window,
};

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::from_cols(
  cgmath::Vector4::new(1.0, 0.0, 0.0, 0.0),
  cgmath::Vector4::new(0.0, 1.0, 0.0, 0.0),
  cgmath::Vector4::new(0.0, 0.0, 0.5, 0.0),
  cgmath::Vector4::new(0.0, 0.0, 0.5, 1.0),
);

#[derive(Debug)]
struct CameraController {
  speed: f64,
  sensivity: f64,

  up:    Vector3<f64>,
  right: Vector3<f64>,
  front: Vector3<f64>,
  
  position: cgmath::Point3 <f64>,
  
  mouse_x: f64,
  mouse_y: f64,
}

impl CameraController {
  fn new(speed: f64, sensivity: f64) -> Self {
    Self {
      speed,
      sensivity,

      up: Vector3::zero(),
      right: Vector3::zero(),
      front: Vector3::unit_z(),
      
      position: Point3::new(0.0, 0.0, 0.0),
      
      mouse_x: 0.0,
      mouse_y: 0.0,
    }
  }

  fn handle_key(&mut self, key_manager: &mut KeyManager, delta_time: f64) {
    let mut move_vector: Vector3<f64> = Vector3::zero();

    if *key_manager.get_key(&KeyCode::KeyW) == KeyState::Held {
      move_vector += self.front;
    }
    if *key_manager.get_key(&KeyCode::KeyS) == KeyState::Held {
      move_vector += -self.front;
    }
    if *key_manager.get_key(&KeyCode::KeyA) == KeyState::Held {
      move_vector -= self.right;
    }
    if *key_manager.get_key(&KeyCode::KeyD) == KeyState::Held {
      move_vector += self.right;
    }
    if *key_manager.get_key(&KeyCode::ShiftLeft) == KeyState::Held {
      move_vector -= self.up;
    }
    if *key_manager.get_key(&KeyCode::Space) == KeyState::Held {
      move_vector += self.up;
    }

    self.position += move_vector * self.speed * delta_time;
  }

  fn handle_mouse_delta(&mut self, delta: (f64, f64), delta_time: f64) {
    (self.mouse_x, self.mouse_y) = (self.mouse_x + delta.0 * (self.sensivity) as f64,
				    self.mouse_y + delta.1 * (self.sensivity) as f64);
  }

  // TODO: Move some vars to Camera
  fn update_camera(&mut self, camera: &mut Camera) {
    self.front =
      Matrix3::from_angle_y(Rad(-self.mouse_x as f64)) *
      Matrix3::from_angle_x(Rad(self.mouse_y as f64)) *
      Vector3::unit_z();

    self.right = self.front.cross(Vector3::unit_y()).normalize();
    self.up = self.right.cross(self.front).normalize();
    camera.up = self.up.cast().unwrap();
    
    camera.position = self.position.cast().unwrap();
    camera.target = (self.position + self.front).cast().unwrap();
  }

  fn resize(&mut self, camera: &mut Camera, width: u32, height: u32) {
    camera.aspect = width as f32 / height as f32;
  }
}

struct Camera {
  position: cgmath::Point3<f32>,
  target:   cgmath::Point3<f32>,
  up:       cgmath::Vector3<f32>,
  aspect: f32,
  fovy: f32,
  znear: f32,
  zfar: f32,
}

impl Camera {
  fn build_view_projection_matrix(&self) -> cgmath::Matrix4<f32> {
    let view = cgmath::Matrix4::look_at_rh(self.position, self.target, self.up);
    let proj = cgmath::perspective(cgmath::Deg(self.fovy), self.aspect, self.znear, self.zfar);
    return OPENGL_TO_WGPU_MATRIX * proj * view;
  }
}

// We need this for Rust to store our data correctly for the shaders
#[repr(C)]
// This is so we can store this in a buffer
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
  // We can't use cgmath with bytemuck directly, so we'll have
  // to convert the Matrix4 into a 4x4 f32 array
  view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
  fn new() -> Self {
    use cgmath::SquareMatrix;
    Self {
      view_proj: cgmath::Matrix4::identity().into(),
    }
  }

  fn update_view_proj(&mut self, camera: &Camera) {
    self.view_proj = camera.build_view_projection_matrix().into();
  }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
  position: [f32; 3],
  color: [f32; 3],
}

const VERTICES: &[Vertex] = &[
  Vertex { position: [ 1.0,  1.0,  0.0], color: [0.0, 0.0, 0.0] },
  Vertex { position: [ 1.0,  0.0,  0.0], color: [0.0, 0.0, 0.0] },
  Vertex { position: [ 1.0,  1.0,  1.0], color: [0.0, 0.0, 0.0] },
  Vertex { position: [ 1.0,  0.0,  1.0], color: [0.0, 0.0, 0.0] },
  Vertex { position: [ 0.0,  1.0,  0.0], color: [0.0, 0.0, 0.0] },
  Vertex { position: [ 0.0,  0.0,  0.0], color: [0.0, 0.0, 0.0] },
  Vertex { position: [ 0.0,  1.0,  1.0], color: [0.0, 0.0, 0.0] },
  Vertex { position: [ 0.0,  0.0,  1.0], color: [0.0, 0.0, 0.0] },
];

const INDICES: &[u16] = &[
  4, 2, 0,
  2, 7, 3,
  6, 5, 7,
  1, 7, 5,
  0, 3, 1,
  4, 1, 5,
  4, 6, 2,
  2, 6, 7,
  6, 4, 5,
  1, 3, 7,
  0, 2, 3,
  4, 0, 1,
];

impl Vertex {
  const ATTRIBS: [wgpu::VertexAttribute; 2] =
    wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];

  fn desc() -> wgpu::VertexBufferLayout<'static> {
    use std::mem;

    wgpu::VertexBufferLayout {
      array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
      step_mode: wgpu::VertexStepMode::Vertex,
      attributes: &Self::ATTRIBS,
    }
  }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct VertexInstance { 
  position: [f32; 3],
}

impl VertexInstance {
  const ATTRIBS: [wgpu::VertexAttribute; 1] =
    wgpu::vertex_attr_array![2 => Float32x3];

  fn desc() -> wgpu::VertexBufferLayout<'static> {
    use std::mem;

    wgpu::VertexBufferLayout {
      array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
      step_mode: wgpu::VertexStepMode::Instance,
      attributes: &Self::ATTRIBS,
    }
  }
}

// This will store the state of our game
pub struct State {
  surface: wgpu::Surface<'static>,
  device: wgpu::Device,
  queue: wgpu::Queue,
  config: wgpu::SurfaceConfiguration,
  is_surface_configured: bool,
  window: Arc<Window>,
  render_pipeline: wgpu::RenderPipeline,
  fps: fps::Fps,
  vertex_buffer: wgpu::Buffer,
  instance_buffer: wgpu::Buffer,
//num_vertices: u32,
  index_buffer: wgpu::Buffer, 
  num_indices: u32,

  camera: Camera,
  camera_uniform: CameraUniform,
  camera_buffer: wgpu::Buffer,
  camera_bind_group: wgpu::BindGroup,
  camera_controller: CameraController,

  key_manager: KeyManager,

  instances: Vec<VertexInstance>,
}

impl State {
  pub async fn new(window: Arc<Window>) -> anyhow::Result<Self> {
    let size = window.inner_size();

    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
      backends: wgpu::Backends::PRIMARY,
      flags: Default::default(),
      memory_budget_thresholds: Default::default(),
      backend_options: Default::default(),
      display: None,
    });

    let surface = instance.create_surface(window.clone()).unwrap();

    let adapter = instance
      .request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::default(),
        compatible_surface: Some(&surface),
        force_fallback_adapter: false,
      })
      .await?;

    let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor {
      label: None,
      required_features: wgpu::Features::empty(),
      experimental_features: wgpu::ExperimentalFeatures::disabled(),
      required_limits: wgpu::Limits::default(),
      memory_hints: Default::default(),
      trace: wgpu::Trace::Off,
    }).await?;

    let surface_caps = surface.get_capabilities(&adapter);
    let surface_format = surface_caps.formats.iter()
      .find(|f| f.is_srgb())
      .copied()
      .unwrap_or(surface_caps.formats[0]);
    
    let config = wgpu::SurfaceConfiguration {
      usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
      format: surface_format,
      width: size.width,
      height: size.height,
      present_mode: surface_caps.present_modes[0],
      alpha_mode: surface_caps.alpha_modes[0],
      view_formats: vec![],
      desired_maximum_frame_latency: 2,
    };
    
    let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));

    let vertex_buffer = device.create_buffer_init(
      &wgpu::util::BufferInitDescriptor {
        label: Some("Vertex Buffer"),
        contents: bytemuck::cast_slice(VERTICES),
        usage: wgpu::BufferUsages::VERTEX,
      }
    );

    let mut instances: Vec<VertexInstance> = Vec::new();
    for i in 0..8 {
      for j in 0..8 {
	for k in 0..8 {
	  instances.push(VertexInstance { position: [i as f32, j as f32, k as f32] });
	}
      }
    }
    
    let instance_buffer = device.create_buffer_init(
      &wgpu::util::BufferInitDescriptor {
	label: Some("Instance buffer"),
	contents: bytemuck::cast_slice(instances.as_slice()),
	usage: wgpu::BufferUsages::VERTEX,
      }
    );

    let index_buffer = device.create_buffer_init(
      &wgpu::util::BufferInitDescriptor {
        label: Some("Index Buffer"),
        contents: bytemuck::cast_slice(INDICES),
        usage: wgpu::BufferUsages::INDEX,
      }
    );

//  let num_vertices = VERTICES.len() as u32;
    let num_indices = INDICES.len() as u32;

    let camera = Camera {
      position: (0.0, 0.0, 0.0).into(),
      target: (0.0, 0.0, 0.0).into(),
      up: cgmath::Vector3::unit_y(),
      aspect: config.width as f32 / config.height as f32,
      fovy: 90.0,
      znear: 0.1,
      zfar: 1000.0,
    };

    let mut camera_uniform = CameraUniform::new();
    camera_uniform.update_view_proj(&camera);

    let camera_buffer = device.create_buffer_init(
      &wgpu::util::BufferInitDescriptor {
        label: Some("Camera Buffer"),
        contents: bytemuck::cast_slice(&[camera_uniform]),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
      }
    );

    let camera_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
      entries: &[
        wgpu::BindGroupLayoutEntry {
          binding: 0,
          visibility: wgpu::ShaderStages::VERTEX,
          ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
          },
          count: None,
        }
      ],
      label: Some("camera_bind_group_layout"),
    });

    let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
      layout: &camera_bind_group_layout,
      entries: &[
        wgpu::BindGroupEntry {
          binding: 0,
          resource: camera_buffer.as_entire_binding(),
        }
      ],
      label: Some("camera_bind_group"),
    });

    let render_pipeline_layout =
      device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Render Pipeline Layout"),
        bind_group_layouts: &[
	  Some(&camera_bind_group_layout),
	],
        immediate_size: 0,
      });

    let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
      label: Some("Render Pipeline"),
      layout: Some(&render_pipeline_layout),
      vertex: wgpu::VertexState {
        module: &shader,
        entry_point: Some("vs_main"),
        buffers: &[Vertex::desc(), VertexInstance::desc()],
        compilation_options: wgpu::PipelineCompilationOptions::default(),
      },
      fragment: Some(wgpu::FragmentState {
        module: &shader,
        entry_point: Some("fs_main"),
        targets: &[Some(wgpu::ColorTargetState {
          format: config.format,
          blend: Some(wgpu::BlendState::REPLACE),
          write_mask: wgpu::ColorWrites::ALL,
        })],
        compilation_options: wgpu::PipelineCompilationOptions::default(),
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
      depth_stencil: None,
      multisample: wgpu::MultisampleState {
        count: 1,
        mask: !0,
        alpha_to_coverage_enabled: false, // 4.
      },
      multiview_mask: None, // 5.
      cache: None, // 6.
    });

    let camera_controller = CameraController::new(5.0, 0.001);
    let key_manager = KeyManager::new();
    
    Ok(Self {
      surface,
      device,
      queue,
      config,
      is_surface_configured: false,
      window,
      render_pipeline,
      fps: Fps::new(fps::TargetFps::Unlimited),
      vertex_buffer,
      instance_buffer,
//    num_vertices,
      index_buffer,
      num_indices,
      camera,
      camera_uniform,
      camera_buffer,
      camera_bind_group,
      camera_controller,
      key_manager,
      instances,
    })
  }
  
  pub fn resize(&mut self, width: u32, height: u32) {
    if width > 0 && height > 0 {
      self.config.width = width;
      self.config.height = height;
      self.surface.configure(&self.device, &self.config);
      self.is_surface_configured = true;
    }
  }

  fn render(&mut self) -> anyhow::Result<()> {
    self.window.request_redraw();

    if !self.is_surface_configured {
      return Ok(());
    }

    let mut is_suboptimal = false;
    let output = match self.surface.get_current_texture() {
      wgpu::CurrentSurfaceTexture::Success(surface_texture) => surface_texture,
      wgpu::CurrentSurfaceTexture::Suboptimal(t) => {
	is_suboptimal = true;
	t
      },
      wgpu::CurrentSurfaceTexture::Timeout
	| wgpu::CurrentSurfaceTexture::Occluded
	| wgpu::CurrentSurfaceTexture::Validation => {
          // Skip this frame
          return Ok(());
	}
      wgpu::CurrentSurfaceTexture::Outdated => {
	self.surface.configure(&self.device, &self.config);
	return Ok(());
      }
      wgpu::CurrentSurfaceTexture::Lost => {
	// You could recreate the devices and all resources
	// created with it here, but we'll just bail
	anyhow::bail!("Lost device");
      }
    };

    let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
    let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
      label: Some("Render Encoder"),
    });

    {
      let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("Render Pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
          view: &view,
          resolve_target: None,
          depth_slice: None,
          ops: wgpu::Operations {
            load: wgpu::LoadOp::Clear(wgpu::Color {
              r: 0.1,
              g: 0.2,
              b: 0.3,
              a: 1.0,
            }),
            store: wgpu::StoreOp::Store,
          },
        })],
        depth_stencil_attachment: None,
        occlusion_query_set: None,
        timestamp_writes: None,
        multiview_mask: None,
      });

      render_pass.set_pipeline(&self.render_pipeline);
      render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
      render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
      render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
      render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
      render_pass.draw_indexed(0..self.num_indices, 0, 0..self.instances.len() as u32);
    }

    // submit will accept anything that implements IntoIter
    self.queue.submit(std::iter::once(encoder.finish()));
    output.present();
    
    if is_suboptimal {
      self.surface.configure(&self.device, &self.config);
    }

    Ok(())
  }

  fn update(&mut self) {
    self.camera_controller.handle_key(&mut self.key_manager, self.fps.delta);
    self.camera_controller.update_camera(&mut self.camera);
    self.camera_uniform.update_view_proj(&self.camera);
    self.queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[self.camera_uniform]));

    self.key_manager.update();
    dbg!(self.fps);
    println!("{:#?}", self.key_manager);
  }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum KeyState {
  Pressed,
  Held,
  NotPressed,
  Released,
}

#[derive(Debug)]
pub struct KeyManager {
  keys: HashMap<KeyCode, KeyState>,
}

impl KeyManager {
  fn new() -> Self {
    Self {
      keys: HashMap::with_capacity(128),
    }
  }
  
  fn insert_if_doesnt_contain(&mut self, key: &KeyCode) {
    if !self.keys.contains_key(key) {
      self.keys.insert(key.clone(), KeyState::NotPressed);
    }
  }
  
  pub fn get_key(&mut self, key: &KeyCode) -> &KeyState {
    self.insert_if_doesnt_contain(key);
    self.keys.get(&key).unwrap()
  }

  pub fn update_key_from_state(&mut self, key: &KeyCode, state: &KeyState) {
    self.insert_if_doesnt_contain(key);
    self.keys.insert(*key, *state);
  }

  pub fn update(&mut self) {
    self.keys.iter_mut()
      .for_each(|(_, v)| match *v {
	KeyState::Released => *v = KeyState::NotPressed,
	KeyState::Pressed =>  *v = KeyState::Held,
	_ => (),
      });
  }
  
  pub fn update_key(&mut self, new_key: &KeyEvent) {
    if new_key.repeat { return; }
    // we reject weird key-codes
    if let PhysicalKey::Code(code) = new_key.physical_key {
      self.insert_if_doesnt_contain(&code);
      let key_state = self.keys.get_mut(&code).unwrap();
      match new_key.state {
	ElementState::Pressed => {
	  match *key_state {
	    KeyState::NotPressed => *key_state = KeyState::Pressed,
	    KeyState::Released =>   *key_state = KeyState::Pressed,
	    KeyState::Pressed =>    *key_state = KeyState::Held,
	    KeyState::Held =>       *key_state = KeyState::Held,
	  }
	}
	
	ElementState::Released => {
	  match *key_state {
	    KeyState::NotPressed => *key_state = KeyState::NotPressed,
	    KeyState::Released =>   *key_state = KeyState::NotPressed,
	    KeyState::Pressed =>    *key_state = KeyState::Released,
	    KeyState::Held =>       *key_state = KeyState::Released,
	  }
	}
      }
    }
  }
}

pub struct App {
  state: Option<State>,
}

impl App {
  pub fn new() -> Self {
    Self {
      state: None,
    }
  }
}

impl ApplicationHandler<State> for App {
  fn resumed(&mut self, event_loop: &ActiveEventLoop) {
    let window_attributes = Window::default_attributes()
      .with_title("VoxelGame");

    let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
    window.set_cursor_grab(winit::window::CursorGrabMode::Locked).unwrap();
    window.set_cursor_visible(false);
    self.state = Some(pollster::block_on(State::new(window)).unwrap());
  }

  fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: State) {
    self.state = Some(event);
  }

  fn window_event(
    &mut self,
    event_loop: &ActiveEventLoop,
    _window_id: winit::window::WindowId,
    event: WindowEvent,
  ) {
    let state = match &mut self.state {
      Some(canvas) => canvas,
      None => return,
    };

    match event {
      WindowEvent::CloseRequested => event_loop.exit(),
      WindowEvent::Resized(size) => {
	state.camera_controller.resize(&mut state.camera, size.width, size.height);
	state.resize(size.width, size.height);
      }
      WindowEvent::RedrawRequested => {
	state.fps.frame_start();
	state.update();
	match state.render() {
	  Ok(_) => {}
	  Err(e) => {
	    log::error!("{e}");
	    event_loop.exit();
	  }
	}
	state.fps.sleep_till_end();
	state.fps.frame_end();
      },
      WindowEvent::KeyboardInput { device_id: _, event, is_synthetic: _ } => {
	state.key_manager.update_key(&event);
      }
      _ => {}
    }
  }

  fn device_event(
    &mut self,
    _event_loop: &ActiveEventLoop,
    _device_id: DeviceId,
    event: DeviceEvent,
  ) {
    let state = match &mut self.state {
      Some(canvas) => canvas,
      None => return,
    };
    
    match event {
      DeviceEvent::MouseMotion { delta } => {
	state.camera_controller.handle_mouse_delta(delta, state.fps.delta);
      }
      _ => (),
    }
  }
}

pub fn run() -> anyhow::Result<()> {
  env_logger::init();

  let event_loop = EventLoop::with_user_event().build()?;
  let mut app = App::new();
  event_loop.run_app(&mut app)?;
  Ok(())
}
