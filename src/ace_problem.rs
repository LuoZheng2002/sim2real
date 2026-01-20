use std::{cell::RefCell, sync::Arc};

use atomic_refcell::AtomicRefCell;
use indexmap::IndexMap;

use crate::{
    ace_generator::{AgentResultEntry, NormalResultEntry}, base_api::BaseApi, evaluate_parse::FunctionCallHygienic, food_services::FoodPlatform, message::MessageApi, parse_ast::decode_function_list, prompts::{
        multi_turn_agent_prompt_system_en, multi_turn_agent_prompt_user_en, system_prompt_for_normal_data_en, system_prompt_for_preference_data_en, system_prompt_for_special_data_en, user_prompt_en
    }, python_interface::{PythonResponse, PythonTask}, reminder::ReminderApi, travel::Travel, world_state::WorldState
};

use std::io::Write;

pub enum ProblemStatus {
    Waiting,
    Executing,
}
/// Sender/recipient in dialogue history
/// Multi-turn has 3 participants: User, Agent, Execution
/// Multi-step has 2 participants: Agent, Execution (User only appears in initial message)
#[derive(Clone, Debug, Copy, PartialEq, Eq)]
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
    pub message: String,
}

/// Unified agent task state for both multi-turn and multi-step scenarios
/// Python: Scene (multi_turn_scene.py) and Mulit_Step_Scene (multi_step_scene.py)
///
/// Multi-turn: 3-party conversation (User ↔ Agent ↔ Execution), inference_data uses "execution:"
/// Multi-step: 2-party interaction (Agent ↔ Execution), inference_data uses "execution result:"
pub struct AgentProblemState {
    // immutable fields
    /// Initial configuration used to initialize WorldState (kept for reference/reset)
    pub initial_config: WorldState,
    /// Classes involved in this task (e.g., ["BaseApi", "MessageApi"])
    pub involved_classes: Vec<String>,

    // mutable fields
    pub num_steps: usize,
    /// Current state of all API instances (mutates during execution)
    pub world_state: WorldState,
    /// Full dialogue history: [{sender, recipient, message}, ...]
    dialogue_history: Vec<DialogueEntry>,
    /// Accumulated string for LLM prompt
    /// Multi-turn: "user:...\nagent:...\nexecution:..."
    /// Multi-step: "user:...\nagent:...\nexecution result:..."
    // pub inference_data: RefCell<String>,
    /// Function calls made during execution (milestones)
    pub mile_stones: Vec<String>,
}

impl AgentProblemState {
    
    pub fn new_multi_step(
        initial_config: WorldState,
        involved_classes: Vec<String>,
        question: &str,
    ) -> Self {
        let mut world_state = initial_config.clone();
        world_state.populate_with_involved_classes(&involved_classes);
        Self {
            initial_config,
            involved_classes,
            num_steps: 0,
            world_state,
            dialogue_history: vec![DialogueEntry {
                sender: DialogueParticipant::User,
                recipient: DialogueParticipant::Agent,
                message: question.to_string(),
            }],
            // inference_data: RefCell::new(String::new()),
            mile_stones: Vec::new(),
        }
    }
    pub fn new_multi_turn(initial_config: WorldState, involved_classes: Vec<String>) -> Self {
        let mut world_state = initial_config.clone();
        world_state.populate_with_involved_classes(&involved_classes);
        Self {
            initial_config,
            involved_classes,
            num_steps: 0,
            world_state,
            dialogue_history: Vec::new(), // needs to call api user to get started
            // inference_data: RefCell::new(String::new()),
            mile_stones: Vec::new(),
        }
    }
    pub fn get_inference_message(&self) -> String {
        let mut inference_message = String::new();
        for entry in &self.dialogue_history {
            let sender_str = match entry.sender {
                DialogueParticipant::User => "user",
                DialogueParticipant::Agent => "agent",
                DialogueParticipant::Execution => "execution result",
            };
            inference_message.push_str(&format!("{}: {}\n", sender_str, entry.message));
        }
        inference_message
    }
    // pub fn execute_function_calls(&mut self, function_calls: Vec<FunctionCallHygienic>) {
    //     let execution_results = self
    //         .world_state
    //         .execute_function_calls(function_calls.clone());
    //     let execution_message = serde_json::to_string(&execution_results)
    //         .expect("failed to serialize execution results");
    //     let new_history_entry = DialogueEntry {
    //         sender: DialogueParticipant::Execution,
    //         recipient: DialogueParticipant::Agent,
    //         message: execution_message,
    //     };
    //     self.dialogue_history.push(new_history_entry);
    // }
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
                    role: "assistant".to_string(),
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
                    role: "assistant".to_string(),
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
                    role: "assistant".to_string(),
                }
            }
            AceProblemState::MultiStep(agent_problem_state) => {
                let last_sender = agent_problem_state
                    .dialogue_history
                    .last()
                    .expect("In multi-step, dialogue history is initialized with user question")
                    .sender;
                assert!(agent_problem_state.num_steps == 0 || matches!(last_sender, DialogueParticipant::Execution));
                // let inference_message: String = agent_problem_state.inference_data.clone();
                let inference_message = agent_problem_state.get_inference_message();
                // system_prompt = MULTI_TURN_AGENT_PROMPT_SYSTEM_EN.format(time = self.time)
                // user_prompt = MULTI_TURN_AGENT_PROMPT_USER_EN.format(functions = self.functions, history = history)
                let system_prompt = multi_turn_agent_prompt_system_en();
                let functions_str = serde_json::to_string(&self.function)
                    .expect("failed to serialize function");
                let user_prompt = multi_turn_agent_prompt_user_en(&functions_str, &inference_message);
                // User turn
                PythonTask {
                    identifier: self.identifier.clone(),
                    system_prompt, // system prompt is only used in the first turn
                    user_prompt,
                    role: "assistant".to_string(),
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

                writeln!(file_ref, "{}", entry_serialized)
                    .expect("failed to write normal result entry");
                true
            }
            AceProblemState::MultiStep(agent_problem_state) => {
                agent_problem_state.num_steps += 1;
                // when receiving the response, the last recipient must be the agent
                let last_recipient = agent_problem_state
                    .dialogue_history
                    .last()
                    .expect("In multi-step, dialogue history is initialized with user question")
                    .recipient;
                assert!(matches!(last_recipient, DialogueParticipant::Agent));
                let new_history_entry = DialogueEntry {
                    sender: DialogueParticipant::Agent,
                    recipient: DialogueParticipant::Execution,
                    message: response.response.clone(),
                };
                agent_problem_state.dialogue_history.push(new_history_entry);

                if response.response.contains("finish conversation") {
                    // to do: finalize and write to file
                    Self::agent_finish_conversation(self.id.clone(), agent_problem_state, &self.output_file);
                    return true;
                }
                // execute the function call and get the result
                let Ok(function_call_list) = decode_function_list(&response.response) else {
                    let new_history_entry = DialogueEntry {
                        sender: DialogueParticipant::Execution,
                        recipient: DialogueParticipant::Agent,
                        message: "Please do not ask me any questions, use the known conditions to solve the problem".to_string(),
                    };
                    agent_problem_state.dialogue_history.push(new_history_entry);
                    println!("The agent is trying to ask a question: {}", response.response);
                    return false;
                };
                agent_problem_state
                    .mile_stones
                    .push(response.response.clone());
                let execution_results = agent_problem_state.world_state.execute_function_calls(&function_call_list);

                let execution_message = serde_json::to_string(&execution_results)
                    .expect("failed to serialize execution results");
                let new_history_entry = DialogueEntry {
                    sender: DialogueParticipant::Execution,
                    recipient: DialogueParticipant::Agent,
                    message: execution_message,
                };

                agent_problem_state.dialogue_history.push(new_history_entry);

                println!("conversation: {}", agent_problem_state.get_inference_message());

                if agent_problem_state.num_steps > 40 {
                    // to do: finalize and write to file
                    Self::agent_finish_conversation(self.id.clone(), agent_problem_state, &self.output_file);
                    return true;
                }
                false
            }
            _ => todo!(),
        }
    }

    fn agent_finish_conversation(id: String, agent_problem_state: &AgentProblemState, output_file: &Arc<AtomicRefCell<std::fs::File>>) {
        // let normal_result_entry = NormalResultEntry {
        //             id: self.id.clone(),
        //             result: response.response,
        //         };
        //         let entry_serialized = serde_json::to_string(&normal_result_entry)
        //             .expect("failed to serialize normal result entry");
        //         let mut file_ref = self.output_file.borrow_mut();

        //         writeln!(file_ref, "{}", entry_serialized)
        //             .expect("failed to write normal result entry");
        //         true
        let agent_result_entry = AgentResultEntry{
            id,
            final_world_state: agent_problem_state.world_state.clone(),
            output_function_calls: agent_problem_state.mile_stones.clone(),
            conversation: agent_problem_state.get_inference_message(),
        };
        let entry_serialized = serde_json::to_string(&agent_result_entry)
            .expect("failed to serialize agent result entry");
        let mut file_ref = output_file.borrow_mut();
        writeln!(file_ref, "{}", entry_serialized)
            .expect("failed to write agent result entry");
    }

    /// Get the LLM response result for completed problems
    /// This is used for evaluation/output
    pub fn get_result(&self) -> Option<&str> {
        // For single-turn problems, the result is stored after receive_python_response
        // Currently we don't store it, but this method provides the interface
        None
    }
}
