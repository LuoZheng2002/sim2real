use std::{
    cell::RefCell,
    collections::{HashMap, HashSet, VecDeque},
    path::{Path, PathBuf},
    rc::Rc,
    sync::{Arc, LazyLock},
};

use atomic_refcell::AtomicRefCell;
use indexmap::IndexMap;
use pyo3::{pyclass, pymethods};
use serde::{Deserialize, Serialize};

use crate::{
    ace_problem::{
        AceProblem, AceProblemState, AgentProblemState, DialogueEntry, DialogueParticipant,
        ProblemStatus, SingleTurnProblemState,
    },
    datasets::DATASETS,
    paths::{BASE_DATASET_PATH, BASE_OUTPUT_PATH},
    perturbations::PerturbationType,
    python_interface::PythonResponse,
    utils::{load_json_lines, write_json_lines_to_file},
    world_state::WorldState,
};

/// Entry for agent_multi_turn and agent_multi_step datasets
/// Files: data_agent_multi_turn.json, data_agent_multi_step.json
#[derive(Deserialize, Clone)]
pub struct AgentEntry {
    pub id: String,
    pub question: String,
    pub initial_config: IndexMap<String, serde_json::Value>,
    pub path: Vec<serde_json::Value>, // unused in current codebase
    pub function: Vec<serde_json::Value>, // passed as-is to LLM
    pub involved_classes: Vec<String>,
}

/// Entry for normal and special datasets (most common format)
/// Files: data_normal_atom_*.json, data_normal_multi_turn_*.json,
///        data_normal_similar_api.json, data_normal_single_turn_*.json,
///        data_special_*.json
#[derive(Deserialize, Clone)]
pub struct NormalEntry {
    pub id: String,
    pub question: String,
    pub function: Vec<serde_json::Value>,
    #[serde(default)]
    pub time: Option<String>, // exists in non-preference normal datasets, even if it exists, it can be empty string
    #[serde(default)]
    pub profile: Option<String>, // exists in normal preference datasets, does not exist in other datasets
}

// /// Entry for preference dataset
// /// File: data_normal_preference.json
// #[derive(Deserialize, Clone)]
// pub struct PreferenceEntry {
//     pub id: String,
//     pub question: String,
//     pub function: Vec<serde_json::Value>,
//     pub profile: String, // JSON-like string containing user profile
// }

/// Unified entry enum for all dataset types
#[derive(Deserialize, Clone)]
#[serde(untagged)]
pub enum DatasetEntry {
    Agent(AgentEntry),
    Normal(NormalEntry),
}

#[derive(Serialize, Deserialize, Clone)]
pub struct AgentResultEntry {
    pub id: String,
    pub conversation: String,
    pub final_world_state: WorldState,
    pub output_function_calls: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct NormalResultEntry {
    pub id: String,
    pub result: String, // to do: there might be a better representation
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum ResultEntry {
    Agent(AgentResultEntry),
    Normal(NormalResultEntry),
}

pub enum ProblemType {
    SingleTurnNormal,
    SingleTurnPreference,
    SingleTurnSpecial,
    AgentMultiTurn,
    AgentMultiStep,
}
pub enum EvaluationType {
    NormalSingleTurn,
    NormalMultiTurn,
    SpecialErrorParam,
    SpecialIncomplete,
    SpecialIrrelevant,
    Agent,
}
pub struct DatasetTrait {
    pub problem_type: ProblemType,
    pub evaluation_type: EvaluationType,
}

fn parse_entries_to_problems(
    entries: Vec<serde_json::Value>,
    perturbation_type: PerturbationType,
    dataset_name: String,
    output_file_path: impl AsRef<Path>,
    problem_type: &ProblemType,
) -> Vec<AceProblem> {
    let existing_entries: Vec<serde_json::Value> =
        load_json_lines(output_file_path.as_ref()).unwrap_or_default();
    let existing_ids = existing_entries
        .iter()
        .map(|entry_value| {
            let entry: ResultEntry =
                serde_json::from_value(entry_value.clone()).expect("failed to parse NormalEntry");
            match entry {
                ResultEntry::Agent(agent_entry) => agent_entry.id,
                ResultEntry::Normal(normal_entry) => normal_entry.id,
            }
        })
        .collect::<HashSet<String>>();
    let output_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(output_file_path.as_ref())
        .expect(&format!(
            "Failed to create/open output file at {}",
            output_file_path.as_ref().display()
        ));
    let output_file = Arc::new(AtomicRefCell::new(output_file));
    let has_transition_perturbation = matches!(
        perturbation_type,
        PerturbationType::Transition
    );
    match problem_type {
        ProblemType::SingleTurnNormal => {
            let mut problems: Vec<AceProblem> = Vec::new();
            for entry_value in entries {
                let entry: NormalEntry = serde_json::from_value(entry_value.clone())
                    .expect("failed to parse NormalEntry");
                if existing_ids.contains(&entry.id) {
                    continue;
                }
                let identifier = format!(
                    "{}_{}_{}",
                    perturbation_type.to_folder_name(),
                    dataset_name,
                    entry.id
                );
                let time = entry
                    .time
                    .clone()
                    .expect("Non-preference normal dataset should have time field");
                let single_turn_state = SingleTurnProblemState {
                    has_transition_perturbation,
                    time: Some(time),
                    profile: None,
                    question: entry.question.clone(),
                    first_turn: true,
                    prev_llm_response: None,
                };
                let problem = AceProblem {
                    identifier,
                    perturbation_type: perturbation_type.to_folder_name(),
                    dataset_name: dataset_name.clone(),
                    id: entry.id,
                    status: ProblemStatus::Waiting,
                    question: entry.question,
                    function: entry.function,
                    state: AceProblemState::SingleTurnNormal(single_turn_state),
                    output_file: output_file.clone(),
                };
                problems.push(problem);
            }
            problems
        }
        ProblemType::SingleTurnPreference => {
            let mut problems: Vec<AceProblem> = Vec::new();
            for entry_value in entries {
                let entry: NormalEntry = serde_json::from_value(entry_value.clone())
                    .expect("failed to parse PreferenceEntry");
                if existing_ids.contains(&entry.id) {
                    continue;
                }
                let identifier = format!(
                    "{}_{}_{}",
                    perturbation_type.to_folder_name(),
                    dataset_name,
                    entry.id
                );
                let profile = entry
                    .profile
                    .clone()
                    .expect("Preference normal dataset should have profile field");
                let single_turn_state = SingleTurnProblemState {
                    has_transition_perturbation,
                    time: None,
                    profile: Some(profile),
                    question: entry.question.clone(),
                    first_turn: true,
                    prev_llm_response: None,
                };
                let problem = AceProblem {
                    identifier,
                    perturbation_type: perturbation_type.to_folder_name(),
                    dataset_name: dataset_name.clone(),
                    id: entry.id,
                    status: ProblemStatus::Waiting,
                    question: entry.question,
                    function: entry.function,
                    state: AceProblemState::SingleTurnPreference(single_turn_state),
                    output_file: output_file.clone(),
                };
                problems.push(problem);
            }
            problems
        }
        ProblemType::SingleTurnSpecial => {
            // let mut problems: Vec<AceProblem> = Vec::new();
            // for entry_value in entries {
            //     let entry: NormalEntry = serde_json::from_value(entry_value.clone())
            //         .expect("failed to parse NormalEntry for special");
            //     if existing_ids.contains(&entry.id) {
            //         continue;
            //     }
            //     let identifier = format!(
            //         "{}_{}_{}",
            //         perturbation_type.to_folder_name(),
            //         dataset_name,
            //         entry.id
            //     );
            //     let time = entry
            //         .time
            //         .clone()
            //         .expect("Non-preference normal dataset should have time field");
            //     let single_turn_state = SingleTurnProblemState {
            //         time: Some(time),
            //         profile: None,
            //         question: entry.question.clone(),
            //         first_turn: true,
            //         prev_llm_response: None,
            //     };
            //     let problem = AceProblem {
            //         identifier,
            //         perturbation_type: perturbation_type.to_folder_name(),
            //         dataset_name: dataset_name.clone(),
            //         id: entry.id,
            //         status: ProblemStatus::Waiting,
            //         question: entry.question,
            //         function: entry.function,
            //         state: AceProblemState::SingleTurnSpecial(single_turn_state),
            //         output_file: output_file.clone(),
            //     };
            //     problems.push(problem);
            // }
            // problems
            panic!("Special single-turn datasets are not supported in this project.");
        }
        ProblemType::AgentMultiTurn => {
            let mut problems: Vec<AceProblem> = Vec::new();
            for entry_value in entries {
                let entry: AgentEntry = serde_json::from_value(entry_value)
                    .expect("failed to parse AgentEntry for multi-turn");
                if existing_ids.contains(&entry.id) {
                    continue;
                }
                let world_state: WorldState =
                    serde_json::from_value(serde_json::to_value(&entry.initial_config).unwrap())
                        .unwrap_or_default();
                let identifier = format!(
                    "{}_{}_{}",
                    perturbation_type.to_folder_name(),
                    dataset_name,
                    entry.id
                );
                let problem = AceProblem {
                    identifier,
                    perturbation_type: perturbation_type.to_folder_name(),
                    dataset_name: dataset_name.clone(),
                    id: entry.id,
                    status: ProblemStatus::Waiting,
                    question: entry.question.clone(),
                    function: entry.function,
                    state: AceProblemState::MultiTurn(AgentProblemState::new_multi_turn(
                        world_state.clone(),
                        entry.involved_classes.clone(),
                        &entry.question,
                        has_transition_perturbation,
                    )),
                    output_file: output_file.clone(),
                };
                problems.push(problem);
            }
            problems
        }
        ProblemType::AgentMultiStep => {
            let mut problems: Vec<AceProblem> = Vec::new();
            for entry_value in entries {
                let entry: AgentEntry = serde_json::from_value(entry_value)
                    .expect("failed to parse AgentEntry for multi-step");
                if existing_ids.contains(&entry.id) {
                    continue;
                }
                let world_state: WorldState =
                    serde_json::from_value(serde_json::to_value(&entry.initial_config).unwrap())
                        .unwrap_or_default();
                let identifier = format!(
                    "{}_{}_{}",
                    perturbation_type.to_folder_name(),
                    dataset_name,
                    entry.id
                );
                let problem = AceProblem {
                    identifier,
                    perturbation_type: perturbation_type.to_folder_name(),
                    dataset_name: dataset_name.clone(),
                    id: entry.id,
                    status: ProblemStatus::Waiting,
                    question: entry.question.clone(),
                    function: entry.function,
                    state: AceProblemState::MultiStep(AgentProblemState::new_multi_step(
                        world_state.clone(),
                        entry.involved_classes.clone(),
                        &entry.question,
                        has_transition_perturbation,
                    )),
                    output_file: output_file.clone(),
                };
                problems.push(problem);
            }
            problems
        }
    }
}

#[pyclass]
pub struct AceGenerator {
    // exposes an interface for getting the next task, assigning an id
    // and retrieving the result and matching it with the id
    // needs to store all the tasks and results
    pub model_safe_name: String,
    pub waiting_queue: VecDeque<AceProblem>,
    pub executing_pool: HashMap<String, AceProblem>,
    pub num_completed: usize,
    pub total_num: usize,
}

#[pymethods]
impl AceGenerator {
    #[new]
    pub fn new(model_name: String) -> Self {
        Self::new_helper(model_name)
    }
    /// Returns a json string with the format {"identifier": str, "system_prompt": str, "user_prompt": str}
    pub fn next_task(&mut self) -> Option<String> {
        self.next_task_helper()
    }

    pub fn receive_response(&mut self, response: String) {
        self.receive_response_helper(response);
    }

    pub fn sort_all_files_after_generation(&mut self) {
        self.sort_all_files_after_generation_helper();
    }
}
impl AceGenerator {
    pub fn new_helper(model_name: String) -> Self {
        let mut waiting_queue = VecDeque::new();
        let executing_pool = HashMap::new();
        let model_safe_name = model_name.replace("/", "-");
        for perturbation_type in PerturbationType::all_perturbations() {
            let perturbation_folder_name = perturbation_type.to_folder_name();
            for (dataset_name, dataset_trait) in DATASETS.iter() {
                // let dataset_path = BASE_DATASET_PATH
                //     .join(perturbation_folder_name.clone())
                //     .join(dataset_name.to_string() + ".json");
                let dataset_path = match perturbation_type {
                    PerturbationType::NoPerturbation | PerturbationType::Transition => {
                        BASE_DATASET_PATH
                            .join(model_safe_name.clone())
                            .join("original_modified") // original dataset
                            .join(dataset_name.to_string() + ".json")
                    }
                    _ => BASE_DATASET_PATH
                        .join(model_safe_name.clone())
                        .join(perturbation_folder_name.clone())
                        .join(dataset_name.to_string() + ".json"),
                };
                let output_path = BASE_OUTPUT_PATH
                    .join(model_safe_name.clone())
                    .join(perturbation_folder_name.clone())
                    .join(dataset_name.to_string() + "_result.json");
                std::fs::create_dir_all(output_path.parent().unwrap())
                    .expect("failed to create output directory");

                // exit the program if any dataset fails to load
                let dataset_entries = load_json_lines(&dataset_path).expect(&format!(
                    "Failed to load dataset from {}",
                    dataset_path.display()
                ));
                let problems = parse_entries_to_problems(
                    dataset_entries,
                    perturbation_type,
                    dataset_name.to_string(),
                    &output_path,
                    &dataset_trait.problem_type,
                );
                waiting_queue.extend(problems);
            }
        }
        let total_num = waiting_queue.len();
        println!("Initialized ACEBenchRunner with {} problems.", total_num);
        AceGenerator {
            model_safe_name,
            waiting_queue,
            executing_pool,
            num_completed: 0,
            total_num,
        }
    }
    pub fn next_task_helper(&mut self) -> Option<String> {
        let mut problem = self.waiting_queue.pop_front()?;
        problem.status = ProblemStatus::Executing;
        let python_task = problem.build_python_task();
        self.executing_pool
            .insert(problem.identifier.clone(), problem);
        let python_task_serialized =
            serde_json::to_string(&python_task).expect("failed to serialize task");
        Some(python_task_serialized)
    }
    pub fn receive_response_helper(&mut self, response: String) {
        // first deserialize the json string to a result struct
        let response: PythonResponse =
            serde_json::from_str(&response).expect("failed to deserialize response");
        let mut problem = self
            .executing_pool
            .remove(&response.identifier)
            .expect("The problem is not in the executing pool");
        let completed = problem.handle_python_response(response);
        if !completed {
            problem.status = ProblemStatus::Waiting;
            println!(
                "Problem {} not completed, re-added to waiting queue.",
                problem.identifier
            );
            self.waiting_queue.push_front(problem); // insert to the front of the queue to make problems finish early            
        } else {
            self.num_completed += 1;
            println!(
                "Problem {} completed. {}/{} completed.",
                problem.identifier, self.num_completed, self.total_num
            );
        }
    }

    pub fn sort_all_files_after_generation_helper(&mut self) {
        for (dataset_name, _) in DATASETS.iter() {
            let output_path = BASE_OUTPUT_PATH
                .join(self.model_safe_name.clone())
                .join(dataset_name.to_string() + "_result.json");

            let entries = match load_json_lines(&output_path) {
                Ok(entries) => entries,
                Err(e) => {
                    println!("Skipping {}: {}", output_path.display(), e);
                    continue;
                }
            };

            let is_multi_turn = dataset_name.contains("normal_multi_turn");

            let mut entries = entries;
            entries.sort_by(|a, b| {
                let id_a = a.get("id").and_then(|v| v.as_str()).unwrap_or("");
                let id_b = b.get("id").and_then(|v| v.as_str()).unwrap_or("");

                if is_multi_turn {
                    // For multi_turn datasets, IDs are like "123_456"
                    // Compare by first number (major), then second number (minor)
                    let parse_multi_turn_id = |id: &str| -> (i64, i64) {
                        let parts: Vec<&str> = id.split('_').collect();
                        let major = parts.first().and_then(|s| s.parse().ok()).unwrap_or(0);
                        let minor = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
                        (major, minor)
                    };
                    let (major_a, minor_a) = parse_multi_turn_id(id_a);
                    let (major_b, minor_b) = parse_multi_turn_id(id_b);
                    (major_a, minor_a).cmp(&(major_b, minor_b))
                } else {
                    // For other datasets, extract trailing number from ID
                    let extract_trailing_number = |id: &str| -> i64 {
                        id.chars()
                            .rev()
                            .take_while(|c| c.is_ascii_digit())
                            .collect::<String>()
                            .chars()
                            .rev()
                            .collect::<String>()
                            .parse()
                            .unwrap_or(0)
                    };
                    let num_a = extract_trailing_number(id_a);
                    let num_b = extract_trailing_number(id_b);
                    num_a.cmp(&num_b)
                }
            });

            if let Err(e) = write_json_lines_to_file(&output_path, &entries) {
                println!(
                    "Failed to write sorted file {}: {}",
                    output_path.display(),
                    e
                );
            } else {
                println!("Sorted {}", output_path.display());
            }
        }
    }
}
