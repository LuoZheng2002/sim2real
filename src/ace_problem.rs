use std::sync::Arc;

use atomic_refcell::AtomicRefCell;
use indexmap::IndexMap;

use crate::{ace_generator::NormalResultEntry, prompts::{system_prompt_for_normal_data_en, system_prompt_for_preference_data_en, system_prompt_for_special_data_en, user_prompt_en}, python_interface::{PythonResponse, PythonTask}, world_state::WorldState};

use std::io::Write;



pub enum ProblemStatus {
    Waiting,
    Executing,
}
/// Sender/recipient in dialogue history
/// Multi-turn has 3 participants: User, Agent, Execution
/// Multi-step has 2 participants: Agent, Execution (User only appears in initial message)
#[derive(Clone, Debug)]
pub enum DialogueParticipant {
    User,
    Agent,
    Execution,
}

/// A single dialogue entry in the conversation history
/// Python: {"sender": str, "recipient": str, "message": str | list}
#[derive(Clone, Debug)]
pub struct DialogueEntry {
    pub sender: DialogueParticipant,
    pub recipient: DialogueParticipant,
    /// Message content - can be string or list (execution results)
    pub message: serde_json::Value,
}


/// Unified agent task state for both multi-turn and multi-step scenarios
/// Python: Scene (multi_turn_scene.py) and Mulit_Step_Scene (multi_step_scene.py)
///
/// Multi-turn: 3-party conversation (User ↔ Agent ↔ Execution), inference_data uses "execution:"
/// Multi-step: 2-party interaction (Agent ↔ Execution), inference_data uses "execution result:"
pub struct AgentProblemState {
    // immutable fields
    /// Initial configuration used to initialize WorldState (kept for reference/reset)
    pub initial_config: IndexMap<String, serde_json::Value>,
    /// Classes involved in this task (e.g., ["BaseApi", "MessageApi"])
    pub involved_classes: Vec<String>,

    // mutable fields
    /// Current state of all API instances (mutates during execution)
    pub world_state: WorldState,
    /// Full dialogue history: [{sender, recipient, message}, ...]
    pub dialogue_history: Vec<DialogueEntry>,
    /// Accumulated string for LLM prompt
    /// Multi-turn: "user:...\nagent:...\nexecution:..."
    /// Multi-step: "user:...\nagent:...\nexecution result:..."
    pub inference_data: String,
    /// Function calls made during execution (milestones)
    pub mile_stones: Vec<String>,
}

pub enum AceProblemState {
    SingleTurnNormal { time: String },
    SingleTurnPreference { profile: String },
    SingleTurnSpecial { time: String },
    MultiTurn(AgentProblemState),
    MultiStep(AgentProblemState),
}

pub struct AceProblem {
    pub identifier: String,
    pub dataset_name: String,
    pub id: String,
    pub status: ProblemStatus,
    /// The original question from the dataset entry
    pub question: String,
    /// Function definitions available for this task (passed to LLM)
    pub function: Vec<serde_json::Value>,
    pub state: AceProblemState,
    pub output_file: Arc<AtomicRefCell<std::fs::File>>,
}

impl AceProblem {
    /// The LLM task is going to be executed by python, and it will produce a response with the same identifier
    /// after receiving the response, the internal state will be updated accordingly
    pub fn build_python_task(&self) -> PythonTask {
        match &self.state {
            AceProblemState::SingleTurnNormal { time } => {
                let function_str =
                    serde_json::to_string(&self.function).expect("failed to serialize function");
                let system_prompt = system_prompt_for_normal_data_en(time, &function_str);
                let user_prompt = user_prompt_en(&self.question);
                PythonTask {
                    identifier: self.identifier.clone(),
                    system_prompt,
                    user_prompt,
                }
            }
            AceProblemState::SingleTurnPreference { profile } => {
                let function_str =
                    serde_json::to_string(&self.function).expect("failed to serialize function");
                let system_prompt = system_prompt_for_preference_data_en(profile, &function_str);
                let user_prompt = user_prompt_en(&self.question);
                PythonTask {
                    identifier: self.identifier.clone(),
                    system_prompt,
                    user_prompt,
                }
            }
            AceProblemState::SingleTurnSpecial { time } => {
                let function_str =
                    serde_json::to_string(&self.function).expect("failed to serialize function");
                let system_prompt = system_prompt_for_special_data_en(time, &function_str);
                let user_prompt = user_prompt_en(&self.question);
                PythonTask {
                    identifier: self.identifier.clone(),
                    system_prompt,
                    user_prompt,
                }
            }
            _ => todo!(),
        }
    }

    /// If returns true, the problem is completed and can be removed
    /// The file writing happens inside the function
    /// later we need to add a shared file object to ACEProblem that share the same output file, and use a lock to avoid race condition
    /// to avoid complexity, for the agent tasks, we will write the output after the entire task is done
    pub fn handle_python_response(&mut self, response: PythonResponse) -> bool {
        assert!(self.identifier == response.identifier);
        // the status will be updated outside the function
        // this function is to update the internal state based on the response
        match &mut self.state {
            AceProblemState::SingleTurnNormal { .. }
            | AceProblemState::SingleTurnPreference { .. }
            | AceProblemState::SingleTurnSpecial { .. } => {
                // Single-turn problems are completed after one response
                // The response contains the LLM's API call output
                let normal_result_entry = NormalResultEntry {
                    id: self.id.clone(),
                    result: response.response,
                };
                let entry_serialized = serde_json::to_string(&normal_result_entry)
                    .expect("failed to serialize normal result entry");
                let mut file_ref = self.output_file.borrow_mut();

                writeln!(file_ref, "{}", entry_serialized).expect("failed to write normal result entry");
                true
            }
            _ => todo!(),
        }
    }

    /// Get the LLM response result for completed problems
    /// This is used for evaluation/output
    pub fn get_result(&self) -> Option<&str> {
        // For single-turn problems, the result is stored after receive_python_response
        // Currently we don't store it, but this method provides the interface
        None
    }
}