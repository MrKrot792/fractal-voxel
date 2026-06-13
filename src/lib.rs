use std::{collections::HashMap, sync::Arc};
use cgmath::{InnerSpace, Matrix3, Point3, Rad, Vector3, Zero};
use instance::InstanceManager;
use pipeline::{IndexBufferDescriptor, UniformDescriptor, VertexBufferDescriptor};

mod fps;
use fps::Fps;

mod instance;
mod pipeline;

mod entity;

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
pub struct State<'a> {
  pub instance: InstanceManager<'a>,
  fps: fps::Fps,
  camera: Camera,
  camera_uniform: CameraUniform,
  camera_controller: CameraController,
  key_manager: KeyManager,
  instances: Vec<VertexInstance>,
}

impl<'a> State<'a> {
  pub async fn new(window: Arc<Window>) -> anyhow::Result<Self> {
    let mut instances: Vec<VertexInstance> = Vec::new();
    for i in 0..8 {
      for j in 0..8 {
	for k in 0..8 {
	  instances.push(VertexInstance { position: [i as f32, j as f32, k as f32] });
	}
      }
    }

    let size = window.inner_size();
    let camera = Camera {
      position: (0.0, 0.0, 0.0).into(),
      target: (0.0, 0.0, 0.0).into(),
      up: cgmath::Vector3::unit_y(),
      aspect: size.width as f32 / size.height as f32,
      fovy: 90.0,
      znear: 0.1,
      zfar: 1000.0,
    };

    let mut camera_uniform = CameraUniform::new();
    camera_uniform.update_view_proj(&camera);
    
    let uniforms = vec![UniformDescriptor {
      contents: bytemuck::cast_slice(&[camera_uniform]).into(),
      visibility: wgpu::ShaderStages::VERTEX,
    }];

    let vertex_buffers = pipeline::VertexBuffersDescriptor {
      buffers: vec![
	VertexBufferDescriptor {
          contents: bytemuck::cast_slice(VERTICES).into(),
	  description: Vertex::desc(),
	},
	VertexBufferDescriptor {
          contents: bytemuck::cast_slice(instances.clone().as_slice()).into(),
	  description: VertexInstance::desc(),
	},
      ],
      instance_buffer_index: Some(1),
      instance_buffer_len: Some(instances.len()),
    };
    
    let render_pipeline_desc = pipeline::RenderPipelineManagerDescriptor {
      uniforms,
      vertex_buffers: vertex_buffers,
      instance_buffer_index: Some(1),
      index_buffer: IndexBufferDescriptor {
	contents: bytemuck::cast_slice(INDICES).into(),
	content_len: INDICES.len(),
      },
      shader: pipeline::ShaderDataDescriptor::RawData(include_str!("shader.wgsl")),
    };

    let instance_manager = InstanceManager::new(window, render_pipeline_desc).await?;
    
    let camera_controller = CameraController::new(5.0, 0.001);
    let key_manager = KeyManager::new();
    
    Ok(Self {
      instance: instance_manager,
      fps: Fps::new(fps::TargetFps::Unlimited),
      camera,
      camera_uniform,
      camera_controller,
      key_manager,
      instances,
    })
  }
  
  pub fn resize(&mut self, width: u32, height: u32) {
    if width > 0 && height > 0 {
      let config = self.instance.get_config_mut();
      config.width = width;
      config.height = height;

      self.instance.configure_surface();
    }
  }

  fn render(&mut self) -> anyhow::Result<()> {
    self.instance.render()
  }

  fn update(&mut self) {
    self.camera_controller.handle_key(&mut self.key_manager, self.fps.delta);
    self.camera_controller.update_camera(&mut self.camera);
    self.camera_uniform.update_view_proj(&self.camera);
    self.instance.uniform_write(bytemuck::cast_slice(&[self.camera_uniform]), 0);
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
  state: Option<State<'static>>,
}

impl App {
  pub fn new() -> Self {
    Self {
      state: None,
    }
  }
}

impl ApplicationHandler<State<'static>> for App {
  fn resumed(&mut self, event_loop: &ActiveEventLoop) {
    let window_attributes = Window::default_attributes()
      .with_title("VoxelGame");

    let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
    window.set_cursor_grab(winit::window::CursorGrabMode::Locked).unwrap();
    window.set_cursor_visible(false);
    self.state = Some(pollster::block_on(State::new(window)).unwrap());
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
      WindowEvent::MouseInput { device_id, state, button } => {
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
      DeviceEvent::Button { button, state } => {},
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
