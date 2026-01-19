
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::world_state::WorldState;


#[derive(Serialize, Deserialize, Clone)]
pub struct FunctionCallHygienic {
    pub name: String,
    pub parameters: IndexMap<String, serde_json::Value>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PointingOutHygienic {
    pub name: String,
    pub values: Vec<String>,
}

#[derive(Deserialize, Clone)]
pub struct PossibleAnswerNormalHygienic {
    pub id: String,
    pub ground_truth: Vec<FunctionCallHygienic>,
}

#[derive(Deserialize, Clone)]
pub struct PossibleAnswerAgentHygienic {
    pub id: String,
    pub ground_truth: WorldState,
    pub mile_stone: Vec<String>, // a list of function calls
}

#[derive(Deserialize, Clone)]
pub struct PossibleAnswerPointingOutHygienic {
    pub id: String,
    pub ground_truth: Vec<PointingOutHygienic>,
}
#[derive(Deserialize, Clone)]
pub struct PossibleAnswerIrrelevantHygienic {
    pub id: String,
    pub ground_truth: String, // the same across all answers
}

