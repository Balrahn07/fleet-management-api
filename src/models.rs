use serde::{Deserialize, Serialize};

#[derive(Serialize, Clone)]
pub struct Vehicle {
    pub id: u32,
    pub vin: String,
    pub model: String,
    pub status: String,
}

#[derive(Deserialize)]
pub struct CreateVehicleRequest {
    pub vin: String,
    pub model: String,
}

// pub fn sample_vehicles() -> Vec<Vehicle> {
//     vec![
//         Vehicle {
//             id: 1,
//             vin: "VF123456789".to_string(),
//             model: "Tesla Model Y".to_string(),
//             status: "online".to_string(),
//         },
//         Vehicle {
//             id: 2,
//             vin: "VF987654321".to_string(),
//             model: "Renault Megane E-Tech".to_string(),
//             status: "offline".to_string(),
//         },
//     ]
// }