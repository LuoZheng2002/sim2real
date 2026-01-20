use serde::{Deserialize, Serialize};

// this is the model for the object passed to the LLM as the tool execution result
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExecutionResult {
    status: bool, // true if success, false if error
    pub message: String, // error message if any
}

impl ExecutionResult {
    pub fn success(message: String) -> Self {
        ExecutionResult {
            status: true,
            message,
        }
    }
    pub fn error(message: String) -> Self {
        ExecutionResult {
            status: false,
            message,
        }
    }
}

/// Base API state - shared by MessageApi, ReminderApi, FoodPlatform
/// Python: scenariosen/phone_platform/base_api.py
#[derive(Clone, Debug, Serialize, Deserialize)]
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

    pub fn equals_ground_truth(&self, ground_truth: &BaseApi) -> Result<(), String> {
        if self.wifi != ground_truth.wifi {
            return Err(format!("Wi-Fi status does not match. expected: {}, got: {}", ground_truth.wifi, self.wifi));
        }
        if self.logged_in != ground_truth.logged_in {
            return Err(format!("Logged-in status does not match. expected: {}, got: {}", ground_truth.logged_in, self.logged_in));
        }
        Ok(())
    }
}