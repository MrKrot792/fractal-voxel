use std::collections::{VecDeque, HashMap};
use std::rc::Rc;
use std::cell::RefCell;
use winit::event::{DeviceEvent, ElementState, KeyEvent, MouseButton, WindowEvent};
use crate::renderer::instance;
use crate::fps;

#[derive(Debug)]
pub enum Event {
  Resized(winit::dpi::PhysicalSize<u32>),
  Key(KeyEvent),
  MouseMotion((f64, f64)),
  Mouse(ElementState, MouseButton),
}

// good enough for now i guess
pub trait Entity {
  fn update(&mut self, entity_index: &usize, render_context: &mut instance::RenderContext, fps: &fps::Fps) -> anyhow::Result<()> {
    _ = entity_index;
    _ = render_context;
    _ = fps;
    Ok(())
  }
  fn event(&mut self, entity_index: &usize,  render_context: &mut instance::RenderContext, event: &Event) -> anyhow::Result<()> {
    _ = entity_index;
    _ = render_context;
    _ = event;
    Ok(())
  }
  fn set_id(&mut self, new_id: usize);
}

#[derive(Default)]
pub struct EntityManager {
  event_queue: VecDeque<Event>,
  entities: HashMap<usize, Rc<RefCell<dyn Entity>>>,
  entity_last_id: usize,
}

impl EntityManager {
  pub fn new() -> Self {
    Self::default()
  }
  
  pub fn entity_create<T: Entity + 'static>(&mut self, entity: Rc<RefCell<T>>) -> usize {
    entity.borrow_mut().set_id(self.entity_last_id);
    self.entities.insert(self.entity_last_id, entity);
    self.entity_last_id += 1;
    self.entity_last_id - 1
  }

  pub fn entity_destroy(&mut self, entity: usize) -> Option<Rc<RefCell<dyn Entity>>> {
    self.entities.remove(&entity)
  }

  pub fn update(&mut self, render_context: &mut instance::RenderContext, fps: &fps::Fps) -> anyhow::Result<()> {
    for (k, v) in &self.entities {
      v.borrow_mut().update(k, render_context, fps)?
    }

    Ok(())
  }
  
  pub fn handle_event_window(&mut self, event: WindowEvent) {
    match event {
      WindowEvent::Resized(size) => self.event_queue.push_back(Event::Resized(size)),
      WindowEvent::KeyboardInput { device_id: _, event, is_synthetic: _ } =>
	self.event_queue.push_back(Event::Key(event)),
      WindowEvent::MouseInput { device_id: _, state, button } =>
	self.event_queue.push_back(Event::Mouse(state, button)),
      _ => (),
    }
  }

  pub fn handle_event_device(&mut self, event: DeviceEvent) {
    match event {
      DeviceEvent::MouseMotion { delta } =>
	self.event_queue.push_back(Event::MouseMotion(delta)),
      _ => (),
    }
  }

  pub fn dispatch_events(&mut self, render_context: &mut instance::RenderContext) -> anyhow::Result<()> {
    for event in self.event_queue.iter() {
      for (k, v) in self.entities.iter_mut() {
	v.borrow_mut().event(k, render_context, event)?;
      }
    }

    self.event_queue.clear();
    Ok(())
  } 
}
