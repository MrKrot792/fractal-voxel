use std::cell::RefCell;
use std::rc::Rc;

use cgmath::InnerSpace;
use cgmath::Vector3;
use cgmath::Zero;
use cgmath::Matrix3;
use cgmath::Point3;
use cgmath::Rad;

use crate::entities::{
  entity::{self, *},
  key_manager::*,
};
use crate::renderer::instance;
use crate::fps;
use winit::keyboard::KeyCode;

// TODO: this can be optimized.
#[derive(Debug)]
pub struct Camera {
  speed: f32,
  sensitivity: f32,
  fps: fps::Fps,
  
  fovy: f32,
  aspect: f32,
  znear: f32,
  zfar: f32,
  view_proj: [[f32; 4]; 4],

  up:    Vector3<f32>,
  right: Vector3<f32>,
  front: Vector3<f32>,
  
  position: cgmath::Point3 <f32>,
  
  mouse_x: f64,
  mouse_y: f64,

  entity_id: usize,
}

impl Entity for Camera {
  fn update(&mut self, _entity_index: &usize, render_context: &mut instance::RenderContext, fps: &fps::Fps) -> anyhow::Result<()> {
    self.front =
      Matrix3::from_angle_y(Rad(-self.mouse_x as f32)) *
      Matrix3::from_angle_x(Rad(self.mouse_y as f32)) *
      Vector3::unit_z();

    self.right = self.front.cross(Vector3::unit_y()).normalize();
    self.up = self.right.cross(self.front).normalize();

    self.update_view_proj();
    render_context.uniform_write(
      bytemuck::cast_slice(&[self.get_view_proj()]),
      0
    );

    self.fps = *fps;

    Ok(())
  }

  fn event(&mut self, _entity_index: &usize, _instance: &mut instance::RenderContext, event: &crate::entities::entity::Event) -> anyhow::Result<()> {
    match *event {
      entity::Event::MouseMotion(d) => self.handle_mouse_delta(d),
      entity::Event::Resized(r) => self.resize(r.width, r.height),
      _ => (),
    }
    
    Ok(())
  }

  fn set_id(&mut self, new_id: usize) {
    self.entity_id = new_id;
  }
}

impl Inputable for Camera {
  fn handle_key(&mut self, key_manager: &mut KeyManager) {
    let mut move_vector: Vector3<f32> = Vector3::zero();

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

    self.position += move_vector * self.speed * self.fps.delta as f32;
  }
}

pub struct CameraDescriptor {
  pub speed: f32,
  pub sensitivity: f32,
  pub fovy: f32,
  pub aspect: f32,
  pub fps: fps::Fps
}

impl Camera {
  pub fn new(descriptor: CameraDescriptor) -> Self {
    use cgmath::SquareMatrix;
    Self {
      speed: descriptor.speed,
      sensitivity: descriptor.sensitivity,
      fps: descriptor.fps,

      fovy: descriptor.fovy,
      aspect: descriptor.aspect,
      znear: 0.1,
      zfar: 1000.0,
      view_proj: cgmath::Matrix4::identity().into(),
      
      up: Vector3::zero(),
      right: Vector3::zero(),
      front: Vector3::unit_z(),
      
      position: Point3::new(0.0, 0.0, 0.0),
      
      mouse_x: 0.0,
      mouse_y: 0.0,

      entity_id: 0,
    }
  }

  pub fn get_view_proj(&self) -> [[f32; 4]; 4] { self.view_proj }
  pub fn manage(s: Rc<RefCell<Self>>, entity_manager: &mut EntityManager) -> usize {
    entity_manager.entity_create(s)
  }

  fn handle_mouse_delta(&mut self, delta: (f64, f64)) {
    (self.mouse_x, self.mouse_y) =
      (self.mouse_x + delta.0 * (self.sensitivity) as f64,
       self.mouse_y + delta.1 * (self.sensitivity) as f64);
  }

  fn resize(&mut self, width: u32, height: u32) {
    self.aspect = width as f32 / height as f32;
  }

  fn build_view_projection_matrix(&self) -> cgmath::Matrix4<f32> {
    let view = cgmath::Matrix4::look_at_rh(self.position, self.position + self.front, self.up);
    let proj = cgmath::perspective(cgmath::Deg(self.fovy), self.aspect, self.znear, self.zfar);
    OPENGL_TO_WGPU_MATRIX * proj * view
  }

  pub fn update_view_proj(&mut self) {
    self.view_proj = self.build_view_projection_matrix().into();
  }
}

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::from_cols(
  cgmath::Vector4::new(1.0, 0.0, 0.0, 0.0),
  cgmath::Vector4::new(0.0, 1.0, 0.0, 0.0),
  cgmath::Vector4::new(0.0, 0.0, 0.5, 0.0),
  cgmath::Vector4::new(0.0, 0.0, 0.5, 1.0),
);

// We need this for Rust to store our data correctly for the shaders
#[repr(C)]
// This is so we can store this in a buffer
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
  // We can't use cgmath with bytemuck directly, so we'll have
  // to convert the Matrix4 into a 4x4 f32 array
  view_proj: [[f32; 4]; 4],
}
