use std::any::Any;
use std::{cell::RefCell, rc::Rc, collections::HashMap};
use std::borrow::BorrowMut;
use winit::{
  event::{ElementState, KeyEvent},
  keyboard::{KeyCode, PhysicalKey}
};

use crate::entities::entity::{self, RequestedCallbacks};

pub trait Inputable: Any {
  fn handle_key(&mut self, keys: &mut KeyManager);
}

#[derive(Debug)]
pub struct KeyManager {
  keys: HashMap<KeyCode, KeyState>,  
}

impl KeyManager {  
  fn insert_if_doesnt_contain(&mut self, key: &KeyCode) {
    if !self.keys.contains_key(key) {
      self.keys.insert(*key, KeyState::NotPressed);
    }
  }

  pub fn get_key(&mut self, key: &KeyCode) -> &KeyState {
    self.insert_if_doesnt_contain(key);
    self.keys.get(key).unwrap()
  }

  // pub fn update_key_from_state(&mut self, key: &KeyCode, state: &KeyState) {
  //   self.insert_if_doesnt_contain(key);
  //   self.keys.insert(*key, *state);
  // }

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
  
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum KeyState {
  Pressed,
  Held,
  NotPressed,
  Released,
}

pub struct KeyInputManager {
  keys: KeyManager,
  entity_id: entity::EntityId,
  inputables: HashMap<usize, entity::EntityId>,
  inputable_last_id: usize,
}

impl entity::Entity for KeyInputManager {
  fn init(&mut self, id: entity::EntityId) -> RequestedCallbacks {
    self.entity_id = id;
    RequestedCallbacks::UPDATE | RequestedCallbacks::EVENT
  }
  
  fn update(&mut self, _delta_time: &crate::fps::Fps) -> anyhow::Result<()> {
    self.update();
    Ok(())
  }

  fn event(&mut self, event: &crate::entities::entity::Event) -> anyhow::Result<()> {
    if let entity::Event::Key(key_event) = event {
      self.keys.update_key(key_event)
    }
    Ok(())
  }
}

impl KeyInputManager {
  pub fn new() -> Self {
    Self {
      keys: KeyManager { keys: HashMap::with_capacity(128) },
      entity_id: entity::EntityId::empty(),
      inputables: HashMap::new(),
      inputable_last_id: 0,
    }
  }

  pub fn register(&mut self, inputable: entity::EntityId) {
    self.inputables.insert(self.inputable_last_id, inputable);
  }

  pub fn update(&mut self, manager: &mut entity::EntityManagerInner) {
for v in self.inputables.values() {
  let e = manager.entity_get_mut(*v).unwrap();
  let a = e.as_any_mut().downcast_ref::<dyn Inputable>().unwrap();
  a.handle_key(&mut self.keys);
}
    self.keys.update();
  }
  
  // TODO: maybe move this to the entity manager
  pub fn manage(self, entity_manager: &mut entity::EntityManager) -> entity::EntityId {
    entity_manager.entity_create(self)
  }
}
