use std::collections::HashMap;

use once_cell::sync::Lazy;
use tokio::sync::Mutex;

///Store system fingerprint for each user
static SYSTEM_FINGERPRINT: Lazy<Mutex<HashMap<i64, String>>> =
    Lazy::new(|| Mutex::new(HashMap::with_capacity(0)));

/// Sets the system fingerprint for a specific user
///
/// # Arguments
/// * `user_id` - User ID
/// * `fingerprint` - System fingerprint value
///
/// # Returns
/// * `()`
pub async fn set_system_fingerprint(user_id: i64, fingerprint: String) {
    let mut fingerprints = SYSTEM_FINGERPRINT.lock().await;
    fingerprints.insert(user_id, fingerprint);
}

/// Retrieves the system fingerprint for a specific user
///
/// # Arguments
/// * `user_id` - User ID
///
/// # Returns
/// * `String` - System fingerprint value or empty string if not found
pub async fn get_system_fingerprint(user_id: i64) -> String {
    let fingerprints = SYSTEM_FINGERPRINT.lock().await;
    fingerprints
        .get(&user_id)
        .unwrap_or(&"".to_string())
        .to_string()
}
