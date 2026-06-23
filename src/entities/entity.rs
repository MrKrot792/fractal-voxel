use std::borrow::BorrowMut;
use std::collections::{VecDeque, HashMap};
use std::cell::{BorrowError, BorrowMutError, Ref, RefCell, RefMut};
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

#[derive(Clone, Copy, Hash, Debug, Eq, PartialEq)]
pub struct EntityId {
  id: usize,
}

impl EntityId {
  pub fn empty() -> Self {
    Self { id: 0 }
  }
  
  pub fn new(id: usize) -> Self {
    Self { id }
  }
  
  pub fn get_entity(&self, entity_manager: &EntityManager) -> Result<Ref<'_, Box<dyn Entity>>, BorrowError> {
    entity_manager.entity_get(self.clone())
  }
  
  pub fn get_entity_mut(&self, entity_manager: &EntityManager) -> Result<RefMut<'_, Box<dyn Entity>>, BorrowMutError> {
    entity_manager.entity_get_mut(self.clone())
  }
}

/// A trait for general kind of entity, for something that updates in
/// the game. For example, some kind of manager.
/// The Entity must save it's ID when `init()` is called.
/// `event()` is called first, then `update()` is called, and lastly,
/// `render()` is called.
pub trait Entity: Any {
  /// The only needed function.
  fn init(&mut self, id: EntityId) -> RequestedCallbacks;
  fn event(&mut self, _event: &Event) -> anyhow::Result<()> { Ok(()) }
  fn update(&mut self, entity_manager: &mut EntityManagerInner, _fps: &fps::Fps) -> anyhow::Result<()> { Ok(()) }
  fn render(&mut self, _render_context: &mut instance::RenderContext) -> anyhow::Result<()> { Ok(()) }
  fn set_id(&mut self, _new_id: EntityId) -> anyhow::Result<()> { Ok(()) }
}

struct EntityWithCallbacks {
  entity: RefCell<Box<dyn Entity>>,
  callbacks: RequestedCallbacks,
}

#[derive(Default)]
pub struct EntityManagerInner {
  entities: HashMap<EntityId, EntityWithCallbacks>,
}

impl EntityManagerInner {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn entity_create<T: Entity + 'static>(&mut self, last_id: &mut usize, mut entity: T) -> EntityId {
    let id = EntityId::new(*last_id);
    let actual_entity = EntityWithCallbacks {
      callbacks: entity.init(id),
      entity: RefCell::new(Box::new(entity)),
    };
    
    self.entities.insert(id, actual_entity);
    *last_id += 1;
    id
  }

  /// Destroys the entity using the ID.
  pub fn entity_destroy(&mut self, entity: EntityId) {
    self.entities.remove(&entity).unwrap();
  }

  pub fn entity_get(&mut self, entity: EntityId) -> Result<Ref<'_, Box<dyn Entity>>, BorrowError> {
    self.entities.get(&entity).unwrap().entity.try_borrow()
  }
  
  pub fn entity_get_mut(&mut self, entity: EntityId) -> Result<RefMut<'_, Box<dyn Entity>>, BorrowMutError> {
    self.entities.get(&entity).unwrap().entity.try_borrow_mut()
  }  
}

// TODO: Maybe add multiple Vec's for storing referemces to
// entities with different callbacks, so it's faster to call
// them.
#[derive(Default)]
pub struct EntityManager {
  event_queue: VecDeque<Event>,
  entities: EntityManagerInner,
  entity_last_id: usize,
}

impl EntityManager {
  pub fn new() -> Self {
    let mut s = Self::default();
    s.entity_last_id = 1;
    s
  }

  /// Make the entity manager manage the entity.
  pub fn update(&mut self, fps: &fps::Fps) -> anyhow::Result<()> {
    for v in self.entities.entities.values_mut() {
      if !v.callbacks.contains(RequestedCallbacks::UPDATE) { continue; }
      
      v.entity.borrow_mut().update(fps)?
    }

    Ok(())
  }

  pub fn render(&mut self, render_context: &mut instance::RenderContext) -> anyhow::Result<()> {
    for v in self.entities.entities.values_mut() {
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

  pub fn dispatch_events(&mut self) -> anyhow::Result<()> {
    for v in self.entities.entities.values_mut() {
      if !v.callbacks.contains(RequestedCallbacks::EVENT) { continue; }
      for event in self.event_queue.iter() {
	v.entity.borrow_mut().event(event)?;
      }
    }

    self.event_queue.clear();
    Ok(())
  }
}
