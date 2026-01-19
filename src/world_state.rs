use indexmap::IndexMap;
use ordered_float::NotNan;
use serde::{Deserialize, Serialize};

use crate::{base_api::BaseApi, evaluate_parse::FunctionCallHygienic, food_services::FoodPlatform, message::MessageApi, reminder::ReminderApi, travel::Travel};

// ============================================================================
// Scenario API State Structs
// These mirror the Python classes in ACEBench/model_inference/multi_turn/scenariosen/
// ============================================================================



/// Unified world state for multi-turn/multi-step scenarios
/// Contains the state of all involved API instances
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorldState {
    #[serde(rename = "BaseApi", default)]
    pub base_api: Option<BaseApi>,
    #[serde(rename = "MessageApi", default)]
    pub message_api: Option<MessageApi>,
    #[serde(rename = "ReminderApi", default)]
    pub reminder_api: Option<ReminderApi>,
    #[serde(rename = "FoodPlatform", default)]
    pub food_platform: Option<FoodPlatform>,
    #[serde(rename = "Travel", default)]
    pub travel: Option<Travel>,
}

impl WorldState {
    pub fn execute_function_calls(&mut self, function_calls: &Vec<FunctionCallHygienic>) {
        let function_call_names: Vec<&str> = function_calls.iter().map(|fc| fc.name.as_str()).collect();
        println!("function calls to execute: {:?}", function_call_names);
        panic!("Function call execution not implemented yet");
    }
}