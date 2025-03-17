use std::collections::HashMap;

use once_cell::sync::Lazy;
use tokio::sync::Mutex;

///Store temperature for each user
pub static TEMPERATURE: Lazy<Mutex<HashMap<i64, f32>>> = Lazy::new(|| Mutex::new(HashMap::new()));

///Set the temperature for a specific user
///
/// # Arguments
/// * `user_id` - User ID
/// * `temperature` - Temperature value
///
/// # Returns
/// * `()`
pub async fn set_temperature(user_id: i64, temperature: f32) {
    let mut temperatures = TEMPERATURE.lock().await;

    temperatures.insert(
        user_id,
        if temperature < 0.0 || temperature > 1.0 {
            0.7
        } else {
            temperature
        },
    );
}

/// Get the temperature for a specific user
///
/// # Arguments
/// * `user_id` - User ID
///
/// # Returns
/// * `f32` - Temperature value or 0.7 if not found
pub async fn get_temperature(user_id: i64) -> f32 {
    let temperatures = TEMPERATURE.lock().await;
    *temperatures.get(&user_id).unwrap_or(&0.7)
}
