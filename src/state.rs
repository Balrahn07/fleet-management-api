use std::sync::{Arc, Mutex};

use crate::models::Vehicle;

#[derive(Clone)]
pub struct AppState {
    pub vehicles: Arc<Mutex<Vec<Vehicle>>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            vehicles: Arc::new(Mutex::new(Vec::new())),
        }
    }
}
