use std::collections::HashMap;
use controllers::*;

pub struct ControllerRegistry{
    pub register : HashMap<&'static str, Box<Controller + 'static>>
}

impl ControllerRegistry{
    pub fn new()-> ControllerRegistry{
        return ControllerRegistry{
            register : HashMap::new()
        }
    }

    pub fn put<T: Controller + 'static>(&mut self, key: &'static str, controller : T){
        self.register.insert(key,Box::new(controller));
    }

    pub fn get<T: Controller + 'static>(&self, key: &'static str) -> Option<&T>{
        return self.register.get(key)
                    .and_then(|c| c.downcast_ref::<T>())
    }
}