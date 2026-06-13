use std::{cell::RefCell, rc::Rc, collections::HashMap};
use winit::{
  event::{ElementState, KeyEvent},
  keyboard::{KeyCode, PhysicalKey}
};

use crate::{entity::{self, Entity, EntityManager}, instance::RenderContext};

pub trait Inputable {
  fn handle_key(&mut self, keys: &mut KeyManager);
}

#[derive(Debug)]
pub struct KeyManager {
  keys: HashMap<KeyCode, KeyState>,  
}

impl KeyManager {  
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
  
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum KeyState {
  Pressed,
  Held,
  NotPressed,
  Released,
}

pub struct KeyInputManager {
  keys: KeyManager,
  entity_id: usize,
  inputables: HashMap<usize, Rc<RefCell<dyn Inputable>>>,
  inputable_last_id: usize,
}

impl Entity for KeyInputManager {
  fn set_id(&mut self, new_id: usize) {
    self.entity_id = new_id;
  }

  fn update(&mut self, _entity_index: &usize, _instance: &mut RenderContext, _delta_time: &crate::fps::Fps) -> anyhow::Result<()> {
    self.update();
    println!("{:#?}", self.keys);
    Ok(())
  }

  fn event(&mut self, _entity_index: &usize, _instance: &mut RenderContext, event: &crate::entity::Event) -> anyhow::Result<()> {
    match event {
      entity::Event::Key(key_event) => self.keys.update_key(&key_event),
      _ => (),
    }
  
    Ok(())
  }
}

impl KeyInputManager {
  pub fn new() -> Self {
    Self {
      keys: KeyManager { keys: HashMap::with_capacity(128) },
      entity_id: 0,
      inputables: HashMap::new(),
      inputable_last_id: 0,
    }
  }

  pub fn register<T: Inputable + 'static>(&mut self, inputable: Rc<RefCell<T>>) {
    self.inputables.insert(self.inputable_last_id, inputable);
  }

  pub fn update(&mut self) {
    self.keys.update();
    for (_k, v) in &self.inputables {
      v.borrow_mut().handle_key(&mut self.keys);
    }
  }
  
  // TODO: maybe move this to the entity manager
  pub fn manage(self, entity_manager: &mut EntityManager) -> usize {
    entity_manager.entity_create(Rc::new(RefCell::new(self)))
  }
}
