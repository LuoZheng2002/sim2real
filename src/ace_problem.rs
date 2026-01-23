use std::{cell::RefCell, sync::Arc};

use atomic_refcell::AtomicRefCell;
use indexmap::IndexMap;

use crate::{
    ace_generator::{AgentResultEntry, NormalResultEntry},
    base_api::BaseApi,
    evaluate_parse::FunctionCallHygienic,
    food_services::FoodPlatform,
    message::MessageApi,
    parse_ast::decode_function_list,
    prompts::{
        base_prompt_en, multi_step_agent_prompt_system_en, multi_step_agent_prompt_user_en,
        multi_turn_agent_prompt_system_en, multi_turn_agent_prompt_user_en,
        system_prompt_for_normal_data_en, system_prompt_for_preference_data_en,
        system_prompt_for_special_data_en, travel_prompt_en, user_prompt_en,
        user_simulation_init_prompt_en, user_simulation_system_prompt_base_en,
        user_simulation_system_prompt_travel_en,
    },
    python_interface::{PythonResponse, PythonTask},
    reminder::ReminderApi,
    travel::Travel,
    world_state::WorldState,
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
    // Whether this problem has transition perturbation
    pub has_transition_perturbation: bool,
    // Whether the transition has been perturbed
    pub perturbed: bool,
    /// Initial configuration used to initialize WorldState (kept for reference/reset)
    pub initial_config: WorldState,
    /// Classes involved in this task (e.g., ["BaseApi", "MessageApi"])
    pub involved_classes: Vec<String>,
    /// The original question/instruction for user simulation (multi-turn only)
    pub question: Option<String>,

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
        has_transition_perturbation: bool,
    ) -> Self {
        let mut world_state = initial_config.clone();
        world_state.populate_with_involved_classes(&involved_classes);
        Self {
            has_transition_perturbation,
            perturbed: false,
            initial_config,
            involved_classes,
            question: None, // multi-step doesn't need user simulation
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
    pub fn new_multi_turn(
        initial_config: WorldState,
        involved_classes: Vec<String>,
        question: &str,
        has_transition_perturbation: bool,
    ) -> Self {
        let mut world_state = initial_config.clone();
        world_state.populate_with_involved_classes(&involved_classes);
        Self {
            has_transition_perturbation,
            perturbed: false,
            initial_config,
            involved_classes,
            question: Some(question.to_string()), // stored for user simulation
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
                DialogueParticipant::Execution => "execution",
            };
            inference_message.push_str(&format!("{}: {}\n", sender_str, entry.message));
        }
        inference_message
    }

    /// Returns true if the state requires an LLM call (user or agent response).
    /// Valid states for LLM call:
    /// - Empty history (initial)
    /// - last_recipient is User (user needs to respond)
    /// - last_recipient is Agent (agent needs to respond)
    ///
    /// Note: last_recipient == Execution means agent just sent a function call request,
    /// which should be executed locally, not by LLM. If sender is Execution, recipient is always Agent.
    pub fn needs_llm_response(&self) -> bool {
        if self.dialogue_history.is_empty() {
            return true;
        }
        let last = self.dialogue_history.last().unwrap();
        matches!(
            last.recipient,
            DialogueParticipant::User | DialogueParticipant::Agent
        )
    }

    /// Returns true if the state is waiting for local execution (not LLM).
    /// This is when agent just sent a message to execution.
    pub fn is_pending_execution(&self) -> bool {
        if self.dialogue_history.is_empty() {
            return false;
        }
        let last = self.dialogue_history.last().unwrap();
        last.recipient == DialogueParticipant::Execution
    }
}

pub struct SingleTurnProblemState {
    pub has_transition_perturbation: bool,
    pub time: Option<String>,    // for normal and special
    pub profile: Option<String>, // for preference
    pub first_turn: bool,
    pub question: String,
    pub prev_llm_response: Option<String>,
}

pub enum AceProblemState {
    SingleTurnNormal(SingleTurnProblemState),
    SingleTurnPreference(SingleTurnProblemState),
    SingleTurnSpecial(SingleTurnProblemState),
    MultiTurn(AgentProblemState),
    MultiStep(AgentProblemState),
}

pub struct AceProblem {
    pub identifier: String,
    pub perturbation_type: String,
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

const MAX_TURNS: usize = 20;

impl AceProblem {
    /// The LLM task is going to be executed by python, and it will produce a response with the same identifier
    /// after receiving the response, the internal state will be updated accordingly
    pub fn build_python_task(&self) -> PythonTask {
        match &self.state {
            AceProblemState::SingleTurnNormal(single_turn_state) => {
                let function_str =
                    serde_json::to_string(&self.function).expect("failed to serialize function");
                let system_prompt = system_prompt_for_normal_data_en(
                    single_turn_state.time.as_ref().unwrap(),
                    &function_str,
                );
                let user_prompt = if single_turn_state.has_transition_perturbation
                    && !single_turn_state.first_turn
                {
                    let mut user_prompt = user_prompt_en(&single_turn_state.question);
                    let Some(prev_response) = &single_turn_state.prev_llm_response else {
                        panic!("Single-turn normal problem missing previous LLM response");
                    };
                    user_prompt.push_str(format!("\nassistant: {}\ntool: The API server is experiencing high latency due to network issues. Please retry your request.\nassistant: ", prev_response).as_str());
                    user_prompt
                } else {
                    assert!(single_turn_state.first_turn);
                    user_prompt_en(&single_turn_state.question)
                };
                PythonTask {
                    identifier: self.identifier.clone(),
                    system_prompt,
                    user_prompt,
                    role: "assistant".to_string(),
                }
            }
            AceProblemState::SingleTurnPreference(single_turn_state) => {
                let function_str =
                    serde_json::to_string(&self.function).expect("failed to serialize function");
                let system_prompt = system_prompt_for_preference_data_en(
                    single_turn_state.profile.as_ref().unwrap(),
                    &function_str,
                );
                let user_prompt = if single_turn_state.has_transition_perturbation
                    && !single_turn_state.first_turn
                {
                    let mut user_prompt = user_prompt_en(&single_turn_state.question);
                    let Some(prev_response) = &single_turn_state.prev_llm_response else {
                        panic!("Single-turn preference problem missing previous LLM response");
                    };
                    user_prompt.push_str(format!("\nassistant: {}\ntool: The API server is experiencing high latency due to network issues. Please retry your request.\nassistant: ", prev_response).as_str());
                    user_prompt
                } else {
                    assert!(single_turn_state.first_turn);
                    user_prompt_en(&single_turn_state.question)
                };
                PythonTask {
                    identifier: self.identifier.clone(),
                    system_prompt,
                    user_prompt,
                    role: "assistant".to_string(),
                }
            }
            AceProblemState::SingleTurnSpecial(_single_turn_state) => {
                // let function_str =
                //     serde_json::to_string(&self.function).expect("failed to serialize function");
                // let system_prompt = system_prompt_for_special_data_en(
                //     single_turn_state.time.as_ref().unwrap(),
                //     &function_str,
                // );
                // let user_prompt = user_prompt_en(&single_turn_state.question);
                // PythonTask {
                //     identifier: self.identifier.clone(),
                //     system_prompt,
                //     user_prompt,
                //     role: "assistant".to_string(),
                // }
                panic!(
                    "Single-turn special problems are not supported in the current implementation"
                );
            }
            AceProblemState::MultiStep(agent_problem_state) => {
                // Assert: state requires LLM response (not pending execution)
                assert!(
                    agent_problem_state.needs_llm_response(),
                    "build_python_task called but state is pending execution"
                );
                assert!(
                    !agent_problem_state.is_pending_execution(),
                    "build_python_task called but state is pending execution"
                );
                let last_sender = agent_problem_state
                    .dialogue_history
                    .last()
                    .expect("In multi-step, dialogue history is initialized with user question")
                    .sender;
                assert!(
                    agent_problem_state.num_steps == 0
                        || matches!(last_sender, DialogueParticipant::Execution)
                );
                // let inference_message: String = agent_problem_state.inference_data.clone();
                let inference_message = agent_problem_state.get_inference_message();
                // Multi-step uses different prompts - agent decides when to finish
                // Note: Multi-step does NOT use travel_prompt or base_prompt (unlike multi-turn)
                let system_prompt = multi_step_agent_prompt_system_en();
                let functions_str =
                    serde_json::to_string(&self.function).expect("failed to serialize function");
                let user_prompt =
                    multi_step_agent_prompt_user_en(&functions_str, &inference_message);
                // User turn
                PythonTask {
                    identifier: self.identifier.clone(),
                    system_prompt, // system prompt is only used in the first turn
                    user_prompt,
                    role: "assistant".to_string(),
                }
            }
            AceProblemState::MultiTurn(agent_problem_state) => {
                // Assert: state requires LLM response (not pending execution)
                assert!(
                    agent_problem_state.needs_llm_response(),
                    "build_python_task called but state is pending execution"
                );
                assert!(
                    !agent_problem_state.is_pending_execution(),
                    "build_python_task called but state is pending execution"
                );

                // Multi-turn has 3 participants: User, Agent, Execution
                // We need to determine who should respond next based on dialogue history

                if agent_problem_state.dialogue_history.is_empty() {
                    // Initial state: need to get user's first message
                    // The user model is initialized with system prompt containing the instruction
                    // and then asked "Is there anything you need help with today?"
                    let system_prompt = if agent_problem_state
                        .involved_classes
                        .contains(&"Travel".to_string())
                    {
                        user_simulation_system_prompt_travel_en(
                            agent_problem_state
                                .question
                                .as_ref()
                                .expect("Multi-turn requires question for user simulation"),
                        )
                    } else {
                        user_simulation_system_prompt_base_en(
                            agent_problem_state
                                .question
                                .as_ref()
                                .expect("Multi-turn requires question for user simulation"),
                        )
                    };
                    let user_prompt = user_simulation_init_prompt_en();
                    PythonTask {
                        identifier: self.identifier.clone(),
                        system_prompt,
                        user_prompt,
                        role: "user".to_string(),
                    }
                } else {
                    let last_recipient = agent_problem_state
                        .dialogue_history
                        .last()
                        .unwrap()
                        .recipient;

                    match last_recipient {
                        DialogueParticipant::User => {
                            // User needs to respond to agent's message
                            // Build user simulation prompt with conversation history
                            let system_prompt = if agent_problem_state
                                .involved_classes
                                .contains(&"Travel".to_string())
                            {
                                user_simulation_system_prompt_travel_en(
                                    agent_problem_state
                                        .question
                                        .as_ref()
                                        .expect("Multi-turn requires question"),
                                )
                            } else {
                                user_simulation_system_prompt_base_en(
                                    agent_problem_state
                                        .question
                                        .as_ref()
                                        .expect("Multi-turn requires question"),
                                )
                            };
                            // Build the user prompt from conversation history
                            // From the user simulator's perspective:
                            // - Agent's messages are what the agent said TO the user
                            // - User's own messages are what the user previously responded
                            let mut user_prompt = user_simulation_init_prompt_en();
                            for entry in &agent_problem_state.dialogue_history {
                                match entry.sender {
                                    DialogueParticipant::User => {
                                        // User's own previous messages - what "you" (the simulator) said
                                        user_prompt.push_str(&format!("\nuser: {}", entry.message));
                                    }
                                    DialogueParticipant::Agent => {
                                        // Agent messages - what the agent said to the user
                                        user_prompt
                                            .push_str(&format!("\nagent: {}", entry.message));
                                    }
                                    DialogueParticipant::Execution => {
                                        // Execution results are shown to agent, not directly to user
                                        // Skip or could include if agent relayed the info
                                    }
                                }
                            }
                            PythonTask {
                                identifier: self.identifier.clone(),
                                system_prompt,
                                user_prompt,
                                role: "user".to_string(),
                            }
                        }
                        DialogueParticipant::Agent => {
                            // Agent needs to respond
                            let inference_message = agent_problem_state.get_inference_message();
                            // Build system prompt with domain-specific rules
                            let mut system_prompt = multi_turn_agent_prompt_system_en();
                            if agent_problem_state
                                .involved_classes
                                .contains(&"Travel".to_string())
                            {
                                system_prompt.push_str(&travel_prompt_en());
                            }
                            if agent_problem_state
                                .involved_classes
                                .contains(&"BaseApi".to_string())
                            {
                                system_prompt.push_str(&base_prompt_en());
                            }
                            let functions_str = serde_json::to_string(&self.function)
                                .expect("failed to serialize function");
                            let user_prompt =
                                multi_turn_agent_prompt_user_en(&functions_str, &inference_message);
                            PythonTask {
                                identifier: self.identifier.clone(),
                                system_prompt,
                                user_prompt,
                                role: "assistant".to_string(),
                            }
                        }
                        DialogueParticipant::Execution => {
                            panic!("Last recipient cannot be Execution when building Python task");
                        }
                    }
                }
            }
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
            AceProblemState::SingleTurnNormal(single_turn_state)
            | AceProblemState::SingleTurnPreference(single_turn_state)
            | AceProblemState::SingleTurnSpecial(single_turn_state) => {
                // Single-turn problems are completed after one response
                // The response contains the LLM's API call output
                if single_turn_state.has_transition_perturbation && single_turn_state.first_turn {
                    single_turn_state.first_turn = false;
                    single_turn_state.prev_llm_response = Some(response.response.clone());
                    return false;
                }
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
                // Assert: we're receiving a response, so state should have been waiting for LLM
                assert!(
                    agent_problem_state.needs_llm_response(),
                    "handle_python_response called but state was not waiting for LLM"
                );
                assert!(
                    !agent_problem_state.is_pending_execution(),
                    "handle_python_response called but state was pending execution"
                );

                agent_problem_state.num_steps += 1;
                if agent_problem_state.num_steps > MAX_TURNS {
                    // to do: finalize and write to file
                    Self::agent_finish_conversation(
                        self.id.clone(),
                        agent_problem_state,
                        &self.output_file,
                    );
                    return true;
                }
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
                    Self::agent_finish_conversation(
                        self.id.clone(),
                        agent_problem_state,
                        &self.output_file,
                    );
                    return true;
                }
                let function_call_list = match decode_function_list(&response.response) {
                    Ok(funcs) => funcs,
                    Err(e) => {
                        if !response.response.starts_with("[") {
                            let new_history_entry = DialogueEntry {
                                sender: DialogueParticipant::Execution,
                                recipient: DialogueParticipant::Agent,
                                message: "Please do not ask me any questions, use the known conditions to solve the problem".to_string(),
                            };
                            println!(
                                "The agent is trying to ask a question: {}",
                                response.response
                            );
                            agent_problem_state.dialogue_history.push(new_history_entry);
                        } else {
                            let new_history_entry = DialogueEntry {
                                sender: DialogueParticipant::Execution,
                                recipient: DialogueParticipant::Agent,
                                message: format!("Failed to parse function calls: {}", e),
                            };
                            agent_problem_state.dialogue_history.push(new_history_entry);
                        }
                        return false;
                    }
                };
                agent_problem_state
                    .mile_stones
                    .push(response.response.clone());
                if agent_problem_state.has_transition_perturbation && !agent_problem_state.perturbed
                {
                    agent_problem_state.perturbed = true;
                    // synthesize a dialogue entry from execution to agent
                    let new_history_entry = DialogueEntry {
                        sender: DialogueParticipant::Execution,
                        recipient: DialogueParticipant::Agent,
                        message: "The API server is experiencing high latency due to network issues. Please retry your request.".to_string(),
                    };
                    agent_problem_state.dialogue_history.push(new_history_entry);
                } else {
                    let execution_results = agent_problem_state
                        .world_state
                        .execute_function_calls(&function_call_list);
                    let execution_message = serde_json::to_string(&execution_results)
                        .expect("failed to serialize execution results");
                    let new_history_entry = DialogueEntry {
                        sender: DialogueParticipant::Execution,
                        recipient: DialogueParticipant::Agent,
                        message: execution_message,
                    };
                    agent_problem_state.dialogue_history.push(new_history_entry);
                }
                // println!("conversation: {}", agent_problem_state.get_inference_message());
                println!(
                    "Problem {} turn {} response: {}",
                    self.id, agent_problem_state.num_steps, response.response
                );
                
                // Post-condition: state should be ready for next LLM call
                assert!(
                    agent_problem_state.needs_llm_response(),
                    "MultiStep: after handle_python_response, state should need LLM response"
                );
                false
            }
            AceProblemState::MultiTurn(agent_problem_state) => {
                // Assert: we're receiving a response, so state should have been waiting for LLM
                assert!(
                    agent_problem_state.needs_llm_response(),
                    "handle_python_response called but state was not waiting for LLM"
                );
                assert!(
                    !agent_problem_state.is_pending_execution(),
                    "handle_python_response called but state was pending execution"
                );

                agent_problem_state.num_steps += 1;

                if agent_problem_state.dialogue_history.is_empty() {
                    // This is the user's initial message
                    let new_history_entry = DialogueEntry {
                        sender: DialogueParticipant::User,
                        recipient: DialogueParticipant::Agent,
                        message: response.response.clone(),
                    };
                    agent_problem_state.dialogue_history.push(new_history_entry);
                    // Post-condition: now agent needs to respond
                    assert!(
                        agent_problem_state.needs_llm_response(),
                        "MultiTurn: after user initial message, state should need LLM response"
                    );
                    return false;
                }

                let last_recipient = agent_problem_state
                    .dialogue_history
                    .last()
                    .unwrap()
                    .recipient;

                match last_recipient {
                    DialogueParticipant::User => {
                        // User responded, add to history
                        let new_history_entry = DialogueEntry {
                            sender: DialogueParticipant::User,
                            recipient: DialogueParticipant::Agent,
                            message: response.response.clone(),
                        };
                        agent_problem_state.dialogue_history.push(new_history_entry);

                        if response.response.contains("finish conversation") {
                            Self::agent_finish_conversation(
                                self.id.clone(),
                                agent_problem_state,
                                &self.output_file,
                            );
                            return true;
                        }
                        // Post-condition: now agent needs to respond
                        assert!(
                            agent_problem_state.needs_llm_response(),
                            "MultiTurn: after user response, state should need LLM response"
                        );
                        false
                    }
                    DialogueParticipant::Agent | DialogueParticipant::Execution => {
                        // Agent responded (after receiving from user or execution)
                        let new_history_entry = DialogueEntry {
                            sender: DialogueParticipant::Agent,
                            recipient: DialogueParticipant::Execution,
                            message: response.response.clone(),
                        };
                        agent_problem_state.dialogue_history.push(new_history_entry);

                        if response.response.contains("finish conversation") {
                            Self::agent_finish_conversation(
                                self.id.clone(),
                                agent_problem_state,
                                &self.output_file,
                            );
                            return true;
                        }

                        let function_call_list = match decode_function_list(&response.response) {
                            Ok(funcs) => funcs,
                            Err(e) => {
                                if !response.response.starts_with("[") {
                                    // Agent is not making a function call, relay message to user
                                    // Change recipient from Execution to User
                                    agent_problem_state
                                        .dialogue_history
                                        .last_mut()
                                        .unwrap()
                                        .recipient = DialogueParticipant::User;
                                    println!("Agent message to user: {}", response.response);
                                    // Post-condition: now user needs to respond
                                    assert!(
                                        agent_problem_state.needs_llm_response(),
                                        "MultiTurn: after agent message to user, state should need LLM response"
                                    );
                                } else {
                                    let new_history_entry = DialogueEntry {
                                        sender: DialogueParticipant::Execution,
                                        recipient: DialogueParticipant::Agent,
                                        message: format!("Failed to parse function calls: {}", e),
                                    };
                                    agent_problem_state.dialogue_history.push(new_history_entry);
                                }
                                return false;
                            }
                        };

                        // Execute function calls
                        agent_problem_state
                            .mile_stones
                            .push(response.response.clone());

                        if agent_problem_state.has_transition_perturbation
                            && !agent_problem_state.perturbed
                        {
                            agent_problem_state.perturbed = true;
                            // synthesize a dialogue entry from execution to agent
                            let new_history_entry = DialogueEntry {
                                sender: DialogueParticipant::Execution,
                                recipient: DialogueParticipant::Agent,
                                message: "The API server is experiencing high latency due to network issues. Please retry your request.".to_string(),
                            };
                            agent_problem_state.dialogue_history.push(new_history_entry);
                        } else {
                            let execution_results = agent_problem_state
                                .world_state
                                .execute_function_calls(&function_call_list);
                            let execution_message = serde_json::to_string(&execution_results)
                                .expect("failed to serialize execution results");

                            let new_history_entry = DialogueEntry {
                                sender: DialogueParticipant::Execution,
                                recipient: DialogueParticipant::Agent,
                                message: execution_message,
                            };
                            agent_problem_state.dialogue_history.push(new_history_entry);
                        }

                        // println!("conversation: {}", agent_problem_state.get_inference_message());
                        println!(
                            "Problem {} turn {} response: {}",
                            self.id, agent_problem_state.num_steps, response.response
                        );
                        if agent_problem_state.num_steps > MAX_TURNS {
                            Self::agent_finish_conversation(
                                self.id.clone(),
                                agent_problem_state,
                                &self.output_file,
                            );
                            return true;
                        }
                        // Post-condition: now agent needs to respond to execution result
                        assert!(
                            agent_problem_state.needs_llm_response(),
                            "MultiTurn: after execution, state should need LLM response"
                        );
                        false
                    }
                }
            }
        }
    }

    fn agent_finish_conversation(
        id: String,
        agent_problem_state: &AgentProblemState,
        output_file: &Arc<AtomicRefCell<std::fs::File>>,
    ) {
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
        let agent_result_entry = AgentResultEntry {
            id,
            final_world_state: agent_problem_state.world_state.clone(),
            output_function_calls: agent_problem_state.mile_stones.clone(),
            conversation: agent_problem_state.get_inference_message(),
        };
        let entry_serialized = serde_json::to_string(&agent_result_entry)
            .expect("failed to serialize agent result entry");
        let mut file_ref = output_file.borrow_mut();
        writeln!(file_ref, "{}", entry_serialized).expect("failed to write agent result entry");
    }

    /// Get the LLM response result for completed problems
    /// This is used for evaluation/output
    pub fn get_result(&self) -> Option<&str> {
        // For single-turn problems, the result is stored after receive_python_response
        // Currently we don't store it, but this method provides the interface
        None
    }
}
