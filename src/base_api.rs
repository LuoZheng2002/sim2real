use serde::{Deserialize, Serialize};

#[derive(Clone, Debug)]
pub struct ExecutionResult {
    pub status: bool, // true if success, false if error
    pub message: String, // error message if any
}

/// Base API state - shared by MessageApi, ReminderApi, FoodPlatform
/// Python: scenariosen/phone_platform/base_api.py
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BaseApi {
    pub wifi: bool,
    pub logged_in: bool,
}

impl Default for BaseApi {
    fn default() -> Self {
        BaseApi {
            wifi: false,
            logged_in: true,
        }
    }
}

impl BaseApi {
    pub fn turn_on_wifi(&mut self) -> ExecutionResult {
        self.wifi = true;
        ExecutionResult {
            status: true,
            message: "Wi-Fi has been turned on".to_string(),
        }
    }
    pub fn login_device(&mut self) -> ExecutionResult {
        self.logged_in = true;
        ExecutionResult {
            status: true,
            message: "Device has been logged in".to_string(),
        }
    }
}