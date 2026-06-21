use std::collections::{VecDeque, HashMap};
use std::cell::RefCell;
use winit::event::{DeviceEvent, ElementState, KeyEvent, MouseButton, WindowEvent};
use crate::renderer::instance;
use crate::fps;
use std::any::Any;

/// Different kinds of events.
/// They're self explinatory.
#[derive(Debug)]
pub enum Event {
  Resized(winit::dpi::PhysicalSize<u32>),
  Key(KeyEvent),
  /// (f64, f64) is mouse's delta.
  MouseMotion((f64, f64)),
  Mouse(ElementState, MouseButton),
}

bitflags::bitflags! {
  /// Must be returned in the init function.
  /// This defines what callbacks the entity will need, for example, a
  /// simple fps daemon doesn't care about events. So it can just
  /// return [`UPDATE`].
  pub struct RequestedCallbacks: u32 {
    /// Called on update, usually 60 times per second
    const UPDATE = 1 << 0;
    /// Called on any event
    /// This should be only used for things like raw mouse input,
    /// resizing, etc. For keyboard inputs, [`KeyEvent`] exists.
    const EVENT =  1 << 1;
    /// Called whenever the entity's ID changes. Usually, it never
    /// happens, so this is reserved for future use.
    const SET_ID = 1 << 2;
    /// Called after updating, used for rendering. Gives you access
    /// to the render context, which allows for updating some
    /// [`GpuBuffer`]s, or anything else you need to do.
    const RENDER = 1 << 3;
  }
}

pub type EntityId = usize;

/// A trait for general kind of entity, for something that updates in
/// the game. For example, some kind of manager.
/// The Entity must save it's ID when [`init()`] is called.
pub trait Entity: Any {
  /// The only needed function.
  fn init(&mut self, id: EntityId) -> RequestedCallbacks;
  fn update(&mut self, _render_context: &mut instance::RenderContext, _fps: &fps::Fps) -> anyhow::Result<()> { Ok(()) }
  fn render(&mut self, _render_context: &mut instance::RenderContext) -> anyhow::Result<()> { Ok(()) }
  fn event(&mut self, _render_context: &mut instance::RenderContext, _event: &Event) -> anyhow::Result<()> { Ok(()) }
  fn set_id(&mut self, _new_id: EntityId) -> anyhow::Result<()> { Ok(()) }
}

struct EntityWithCallbacks {
  entity: RefCell<Box<dyn Entity>>,
  callbacks: RequestedCallbacks,
}

// TODO: Maybe add multiple Vec's for storing referemces to
// entities with different callbacks, so it's faster to call
// them.
#[derive(Default)]
pub struct EntityManager {
  event_queue: VecDeque<Event>,
  entities: HashMap<EntityId, EntityWithCallbacks>,
  entity_last_id: EntityId,
}

impl EntityManager {
  pub fn new() -> Self {
    Self::default()
  }
  
  pub fn entity_create<T: Entity + 'static>(&mut self, mut entity: T) -> EntityId {
    let actual_entity = EntityWithCallbacks {
      callbacks: entity.init(self.entity_last_id),
      entity: RefCell::new(Box::new(entity)),
    };
     
    self.entities.insert(self.entity_last_id, actual_entity);
    self.entity_last_id += 1;
    self.entity_last_id - 1
  }

  pub fn entity_destroy(&mut self, entity: EntityId) {
    self.entities.remove(&entity).unwrap();
  }

  pub fn update(&mut self, fps: &fps::Fps) -> anyhow::Result<()> {
    for v in self.entities.values_mut() {
      if !v.callbacks.contains(RequestedCallbacks::UPDATE) { continue; }
      
      v.entity.borrow_mut().update(fps)?
    }

    Ok(())
  }

  pub fn render(&mut self, render_context: &mut instance::RenderContext) -> anyhow::Result<()> {
    for v in self.entities.values_mut() {
      if !v.callbacks.contains(RequestedCallbacks::RENDER) { continue; }
      
      v.entity.borrow_mut().render(render_context)?
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
    for v in self.entities.values_mut() {
      if !v.callbacks.contains(RequestedCallbacks::EVENT) { continue; }
      for event in self.event_queue.iter() {
	v.entity.borrow_mut().event(render_context, event)?;
      }
    }

    self.event_queue.clear();
    Ok(())
  }
}
