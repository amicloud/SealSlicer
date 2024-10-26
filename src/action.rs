use std::{cell::RefCell, rc::Rc};

use nalgebra::{Quaternion, Vector3};

use crate::body::Body;

pub trait Action {
    fn execute(&mut self);
    fn undo(&mut self);
}


pub struct SetPositionAction {
    pub body: Rc<RefCell<Body>>,
    pub input: Vector3<f32>,
    pub previous: Vector3<f32>,
}

impl Action for SetPositionAction {
    fn execute(&mut self) {
        self.body.borrow_mut().set_position(self.input);
    }

    fn undo(&mut self) {
        self.body.borrow_mut().set_position(self.previous);
    }
}

pub struct SetRotationAction {
    pub body: Rc<RefCell<Body>>,
    pub input: Vector3<f32>,
    pub previous: Quaternion<f32>,
}

impl Action for SetRotationAction {
    fn execute(&mut self) {
        self.body.borrow_mut().set_rotation(self.input);
    }

    fn undo(&mut self) {
        self.body.borrow_mut().set_rotation_quat(self.previous);
    }
}

pub struct SetScaleAction {
    pub body: Rc<RefCell<Body>>,
    pub input: Vector3<f32>,
    pub previous: Vector3<f32>,
}

impl Action for SetScaleAction {
    fn execute(&mut self) {
        self.body.borrow_mut().set_scale(self.input);
    }

    fn undo(&mut self) {
        self.body.borrow_mut().set_scale(self.previous);
    }
}
