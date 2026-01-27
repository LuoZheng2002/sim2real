use std::collections::HashMap;

use indexmap::IndexMap;
use pyo3::pyfunction;
use regex::Regex;
use rustpython_parser::{Mode, ast, parse};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{
    ace_generator::{AgentEntry, AgentResultEntry, EvaluationType, NormalEntry, NormalResultEntry},
    datasets::DATASETS,
    evaluate_parse::{
        FunctionCallHygienic, PointingOutHygienic, PossibleAnswerAgentHygienic,
        PossibleAnswerIrrelevantHygienic, PossibleAnswerNormalHygienic,
        PossibleAnswerPointingOutHygienic,
    },
    parse_ast::{parse_from_ast_to_structured, parse_from_string_to_ast},
    paths::{BASE_DATASET_PATH, BASE_OUTPUT_PATH, BASE_SCORE_PATH},
    perturbations::PerturbationType,
    utils::{load_json_lines, write_json_lines_to_file},
    world_state::WorldState,
};

// /// Python type mapping equivalent
// fn get_expected_json_type(type_desc: &str) -> &'static str {
//     match type_desc {
//         "string" | "any" => "string",
//         "integer" | "int" | "number" => "number",
//         "float" => "number",
//         "boolean" => "boolean",
//         "array" | "tuple" | "list" | "list(string)" | "list(enum)" | "objectArray" => "array",
//         "dict" | "object" => "object",
//         "enum" => "string",
//         _ => "unknown",
//     }
// }

// /// Types that need nested checking
// fn is_nested_type(type_desc: &str) -> bool {
//     matches!(
//         type_desc,
//         "array" | "tuple" | "list(string)" | "list(enum)" | "object" | "objectArray"
//     )
// }

/// Result of evaluation for a single entry
#[derive(Serialize, Deserialize, Clone)]
pub struct NormalEvaluationResult {
    pub id: String,
    pub valid: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub model_raw_output: String,
    pub possible_answer: Vec<FunctionCallHygienic>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SpecialEvaluationResult {
    pub id: String,
    pub valid: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub model_raw_output: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct AgentEvaluationResult {
    pub id: String,
    pub valid: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub conversation: String,
    pub final_world_state: WorldState,
    pub expected_world_state: WorldState,
    pub output_function_calls: Vec<String>,
    pub expected_function_calls: serde_json::Value,
}

/// Summary statistics for evaluation
#[derive(Debug, Serialize, Deserialize)]
pub struct EvaluationSummary {
    pub accuracy: f64,
    pub correct_count: usize,
    pub total_count: usize,
}

#[pyfunction]
pub fn evaluate_all_results(model_name: String, enable_fc: bool) {
    let model_safe_name = if enable_fc {
        format!("{}-FC", model_name.replace("/", "-"))
    } else {
        model_name.replace("/", "-")
    };
    for perturbation_type in PerturbationType::all_perturbations() {
        let perturbation_folder_name = perturbation_type.to_folder_name();
        for (dataset_name, dataset_trait) in DATASETS.iter() {
            
            let problem_folder_path = match perturbation_type {
                PerturbationType::NoPerturbation | PerturbationType::Transition => {
                    BASE_DATASET_PATH
                        // .join(model_safe_name.clone())
                        .join("original_modified") // original dataset
                        // .join(dataset_name.to_string() + "_result.json")
                }
                _ => BASE_DATASET_PATH
                    // .join(model_safe_name.clone())
                    .join(perturbation_folder_name.clone())
                    // .join(dataset_name.to_string() + "_result.json"),
            };
            let problem_path = problem_folder_path
                .join(dataset_name.to_string() + ".json");
            let possible_answer_path = problem_folder_path
                .join("possible_answer_hygienic")
                .join(dataset_name.to_string() + ".json");

            let result_path = BASE_OUTPUT_PATH
                .join(model_safe_name.clone())
                .join(perturbation_folder_name.clone())
                .join(dataset_name.to_string() + "_result.json");

            

            // Skip if result file doesn't exist
            if !result_path.exists() {
                eprintln!("Result file not found: {:?}, skipping...", result_path);
                continue;
            }

            let problem_entries =
                load_json_lines(&problem_path).expect("Failed to read problem file");
            let result_entries = load_json_lines(&result_path).expect("Failed to read result file");
            let possible_answer_entries = load_json_lines(&possible_answer_path)
                .expect("Failed to read possible answer file");

            let evaluation_results: Vec<serde_json::Value> = match dataset_trait.evaluation_type {
                EvaluationType::NormalSingleTurn => evaluate_normal_single_turn(
                    &result_entries,
                    &problem_entries,
                    &possible_answer_entries,
                ),
                EvaluationType::NormalMultiTurn => evaluate_normal_multi_turn(
                    &result_entries,
                    &problem_entries,
                    &possible_answer_entries,
                ),
                EvaluationType::SpecialIncomplete
                | EvaluationType::SpecialErrorParam
                | EvaluationType::SpecialIrrelevant => evaluate_special(
                    &result_entries,
                    &problem_entries,
                    &possible_answer_entries,
                    &dataset_trait.evaluation_type,
                ),
                EvaluationType::Agent => {
                    evaluate_agent(&result_entries, &problem_entries, &possible_answer_entries)
                }
            };

            let output_evaluation_path = BASE_SCORE_PATH
                .join(model_safe_name.clone())
                .join(perturbation_folder_name.clone())
                .join(dataset_name.clone() + "_evaluation.json");

            // Create directories if not exist
            std::fs::create_dir_all(output_evaluation_path.parent().unwrap())
                .expect("Failed to create directories for evaluation output");
            write_json_lines_to_file(output_evaluation_path, &evaluation_results)
                .expect("Failed to write evaluation results");

            // Print summary
            if let Some(first) = evaluation_results.first() {
                if let Some(accuracy) = first.get("accuracy") {
                    println!("Dataset: {} | Accuracy: {}", dataset_name, accuracy);
                }
            }
        }
    }
}

/// Result of evaluation for a multi-turn entry
#[derive(Serialize, Deserialize, Clone)]
pub struct NormalMultiTurnEvaluationResult {
    pub id: String,
    pub turn: usize,
    pub valid: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub model_raw_output: String,
    pub possible_answer: Vec<FunctionCallHygienic>,
}

/// Calculate end-to-end and process accuracy for multi-turn evaluation
/// score_map: turn -> (item_index -> is_valid)
fn multi_turn_accuracy(score_map: &IndexMap<usize, IndexMap<usize, bool>>) -> (f64, f64) {
    let mut end_score_list = Vec::new();
    let mut process_score_list = Vec::new();

    for (_turn, items) in score_map.iter() {
        let valid_count = items.values().filter(|&&v| v).count();
        let total_items = items.len();

        // End score: 1 if all items in turn are valid, 0 otherwise
        let end_score = if valid_count == total_items { 1.0 } else { 0.0 };

        // Process score: proportion of valid items in turn
        let process_score = if total_items > 0 {
            valid_count as f64 / total_items as f64
        } else {
            0.0
        };

        end_score_list.push(end_score);
        process_score_list.push(process_score);
    }

    let end_score_total = if end_score_list.is_empty() {
        0.0
    } else {
        end_score_list.iter().sum::<f64>() / end_score_list.len() as f64
    };
    let process_score_total = if process_score_list.is_empty() {
        0.0
    } else {
        process_score_list.iter().sum::<f64>() / process_score_list.len() as f64
    };

    (end_score_total, process_score_total)
}

pub fn evaluate_normal_multi_turn(
    result_entries: &Vec<serde_json::Value>,
    problem_entries: &Vec<serde_json::Value>,
    possible_answer_entries: &Vec<serde_json::Value>,
) -> Vec<serde_json::Value> {
    let result_len = result_entries.len();
    let problem_len = problem_entries.len();
    let possible_answer_len = possible_answer_entries.len();
    assert_eq!(problem_len, possible_answer_len);
    assert_eq!(
        result_len, problem_len,
        "The length of the model result ({}) does not match the length of the prompt ({}) or possible answer ({}). Please check the input files for completeness.",
        result_len, problem_len, possible_answer_len
    );

    // Parse entries into IndexMap by id
    let result_entries_parsed: IndexMap<String, NormalResultEntry> = result_entries
        .iter()
        .map(|entry| {
            let parsed: NormalResultEntry = serde_json::from_value(entry.clone())
                .expect("Failed to parse result entry into NormalResultEntry");
            (parsed.id.clone(), parsed)
        })
        .collect();
    let problem_entries_parsed: IndexMap<String, NormalEntry> = problem_entries
        .iter()
        .map(|entry| {
            let parsed: NormalEntry = serde_json::from_value(entry.clone())
                .expect("Failed to parse problem entry into NormalEntry");
            (parsed.id.clone(), parsed)
        })
        .collect();
    let possible_answer_entries_parsed: IndexMap<String, PossibleAnswerNormalHygienic> =
        possible_answer_entries
            .iter()
            .map(|entry| {
                let parsed: PossibleAnswerNormalHygienic = serde_json::from_value(entry.clone())
                    .expect(
                        "Failed to parse possible answer entry into PossibleAnswerNormalHygienic",
                    );
                (parsed.id.clone(), parsed)
            })
            .collect();

    let mut results: Vec<NormalMultiTurnEvaluationResult> = Vec::new();
    let total_count = result_len;
    let mut correct_count = 0;
    // score_map: turn -> (item_index -> is_valid)
    let mut score_map: IndexMap<usize, IndexMap<usize, bool>> = IndexMap::new();

    for id in result_entries_parsed.keys() {
        let result_entry = result_entries_parsed.get(id).expect("Missing result entry");
        let problem_entry = problem_entries_parsed
            .get(id)
            .expect("Missing problem entry");
        let possible_answer_entry = possible_answer_entries_parsed
            .get(id)
            .expect("Missing possible answer entry");

        // Extract turn and item from problem id (format: xxx_turn_item)
        let id_parts: Vec<&str> = problem_entry.id.split('_').collect();
        assert!(
            id_parts.len() >= 2,
            "Problem ID format incorrect, expected at least two parts separated by '_', got: {}",
            problem_entry.id
        );
        let turn: usize = id_parts[id_parts.len() - 2]
            .parse::<usize>()
            .expect("Failed to parse turn index");
        let item: usize = id_parts[id_parts.len() - 1]
            .parse::<usize>()
            .expect("Failed to parse item index");

        let evaluation_result =
            evaluate_one_normal(&result_entry.result, &possible_answer_entry.ground_truth);
        match evaluation_result {
            Ok(_) => {
                correct_count += 1;
                score_map
                    .entry(turn)
                    .or_insert_with(IndexMap::new)
                    .insert(item, true);
                results.push(NormalMultiTurnEvaluationResult {
                    id: result_entry.id.clone(),
                    turn,
                    valid: true,
                    error: None,
                    model_raw_output: result_entry.result.clone(),
                    possible_answer: possible_answer_entry.ground_truth.clone(),
                });
            }
            Err(e) => {
                score_map
                    .entry(turn)
                    .or_insert_with(IndexMap::new)
                    .insert(item, false);
                results.push(NormalMultiTurnEvaluationResult {
                    id: result_entry.id.clone(),
                    turn,
                    valid: false,
                    error: Some(e),
                    model_raw_output: result_entry.result.clone(),
                    possible_answer: possible_answer_entry.ground_truth.clone(),
                });
            }
        }
    }

    // Calculate accuracy
    let (end_accuracy, process_accuracy) = if score_map.is_empty() {
        (0.0, 0.0)
    } else {
        multi_turn_accuracy(&score_map)
    };

    // Insert summary at the beginning
    let summary = json!({
        "accuracy": end_accuracy,
        "correct_count": correct_count,
        "total_count": total_count,
        "process_accuracy": process_accuracy,
    });
    let results_serialized: Vec<serde_json::Value> = results
        .into_iter()
        .map(|res| serde_json::to_value(res).expect("Failed to serialize evaluation result"))
        .collect();
    let mut final_results = vec![summary];
    final_results.extend(results_serialized);
    final_results
}

pub fn evaluate_normal_single_turn(
    result_entries: &Vec<serde_json::Value>,
    problem_entries: &Vec<serde_json::Value>,
    possible_answer_entries: &Vec<serde_json::Value>,
) -> Vec<serde_json::Value> {
    let result_len = result_entries.len();
    let problem_len = problem_entries.len();
    let possible_answer_len = possible_answer_entries.len();
    assert_eq!(problem_len, possible_answer_len);
    assert_eq!(
        result_len, problem_len,
        "The length of the model result ({}) does not match the length of the prompt ({}) or possible answer ({}). Please check the input files for completeness.",
        result_len, problem_len, possible_answer_len
    );

    // normal single turn should use normal result model
    let result_entries_parsed: IndexMap<String, NormalResultEntry> = result_entries
        .iter()
        .map(|entry| {
            let parsed: NormalResultEntry = serde_json::from_value(entry.clone())
                .expect("Failed to parse result entry into NormalResultEntry");
            (parsed.id.clone(), parsed)
        })
        .collect();
    let problem_entries_parsed: IndexMap<String, NormalEntry> = problem_entries
        .iter()
        .map(|entry| {
            let parsed: NormalEntry = serde_json::from_value(entry.clone())
                .expect("Failed to parse problem entry into NormalEntry");
            (parsed.id.clone(), parsed)
        })
        .collect();
    let possible_answer_entries_parsed: IndexMap<String, PossibleAnswerNormalHygienic> =
        possible_answer_entries
            .iter()
            .map(|entry| {
                let parsed: PossibleAnswerNormalHygienic = serde_json::from_value(entry.clone())
                    .expect(
                        "Failed to parse possible answer entry into PossibleAnswerNormalHygienic",
                    );
                (parsed.id.clone(), parsed)
            })
            .collect();

    let mut results: Vec<NormalEvaluationResult> = Vec::new();
    let total_count = result_len;
    let mut correct_count = 0;

    for id in result_entries_parsed.keys() {
        let result_entry = result_entries_parsed.get(id).expect("Missing result entry");
        let _problem_entry = problem_entries_parsed
            .get(id)
            .expect("Missing problem entry");
        let possible_answer_entry = possible_answer_entries_parsed
            .get(id)
            .expect("Missing possible answer entry");

        match evaluate_one_normal(&result_entry.result, &possible_answer_entry.ground_truth) {
            Ok(_) => {
                correct_count += 1;
                results.push(NormalEvaluationResult {
                    id: id.clone(),
                    valid: true,
                    error: None,
                    model_raw_output: result_entry.result.clone(),
                    possible_answer: possible_answer_entry.ground_truth.clone(),
                });
            }
            Err(e) => {
                results.push(NormalEvaluationResult {
                    id: id.clone(),
                    valid: false,
                    error: Some(e),
                    model_raw_output: result_entry.result.clone(),
                    possible_answer: possible_answer_entry.ground_truth.clone(),
                });
            }
        }
    }

    // Calculate accuracy
    let accuracy = if total_count == 0 {
        0.0
    } else {
        correct_count as f64 / total_count as f64
    };

    // Insert summary at the beginning
    let summary = json!({
        "accuracy": accuracy,
        "correct_count": correct_count,
        "total_count": total_count,
    });
    let results_serialized: Vec<serde_json::Value> = results
        .into_iter()
        .map(|res| serde_json::to_value(res).expect("Failed to serialize evaluation result"))
        .collect();

    let mut final_results = vec![summary];
    final_results.extend(results_serialized);
    final_results
}

pub fn check_functions_all_match(
    model_output_calls: &Vec<FunctionCallHygienic>,
    ground_truth_calls: &Vec<FunctionCallHygienic>,
) -> Result<(), String> {
    if model_output_calls.len() != ground_truth_calls.len() {
        return Err("The number of function calls does not match the possible answer.".to_string());
    }
    for ground_truth_call in ground_truth_calls.iter() {
        let Some(_) = model_output_calls
            .iter()
            .find(|&model_output_call| functions_equivalent(ground_truth_call, model_output_call))
        else {
            return Err(format!(
                "No matching function call for {} found in model's output function calls.",
                ground_truth_call.name
            ));
        };
    }
    Ok(())
}

pub fn evaluate_one_normal(
    model_result_raw: &str,
    possible_answer_function_calls: &Vec<FunctionCallHygienic>,
) -> Result<(), String> {
    let decoded_ast = parse_from_string_to_ast(model_result_raw)?;
    let mut decodeded_function_calls =
        parse_from_ast_to_structured(&decoded_ast, model_result_raw)?;
    // check function equivalence
    if decodeded_function_calls.len() != possible_answer_function_calls.len() {
        return Err("The number of function calls does not match the possible answer.".to_string());
    }

    for possible_answer_function_call in possible_answer_function_calls.iter() {
        let Some(pos) = decodeded_function_calls
            .iter()
            .position(|fa| functions_equivalent(&possible_answer_function_call, fa))
        else {
            return Err(format!(
                "No matching function call for {} found in model's output function calls.",
                possible_answer_function_call.name
            ));
        };
        // remove the matched one
        decodeded_function_calls.swap_remove(pos);
    }
    Ok(())
}

pub fn functions_equivalent(func1: &FunctionCallHygienic, func2: &FunctionCallHygienic) -> bool {
    if func1.name != func2.name {
        return false;
    }
    if func1.parameters.len() != func2.parameters.len() {
        return false;
    }
    for (param_name, param_value1) in func1.parameters.iter() {
        let Some(param_value2) = func2.parameters.get(param_name) else {
            return false;
        };
        if !values_equivalent(param_value1, param_value2) {
            return false;
        }
    }
    true
}

pub fn values_equivalent(value1: &serde_json::Value, value2: &serde_json::Value) -> bool {
    // todo: special handling for list and dict
    value1 == value2
}

pub fn evaluate_special(
    result_entries: &Vec<serde_json::Value>,
    problem_entries: &Vec<serde_json::Value>,
    possible_answer_entries: &Vec<serde_json::Value>,
    evaluation_type: &EvaluationType,
) -> Vec<serde_json::Value> {
    let result_len = result_entries.len();
    let problem_len = problem_entries.len();
    let possible_answer_len = possible_answer_entries.len();
    assert_eq!(problem_len, possible_answer_len);
    assert_eq!(
        result_len, problem_len,
        "The length of the model result ({}) does not match the length of the prompt ({}) or possible answer ({}). Please check the input files for completeness.",
        result_len, problem_len, possible_answer_len
    );

    // Parse entries into IndexMap by id
    let result_entries_parsed: IndexMap<String, NormalResultEntry> = result_entries
        .iter()
        .map(|entry| {
            let parsed: NormalResultEntry = serde_json::from_value(entry.clone())
                .expect("Failed to parse result entry into NormalResultEntry");
            (parsed.id.clone(), parsed)
        })
        .collect();
    let problem_entries_parsed: IndexMap<String, NormalEntry> = problem_entries
        .iter()
        .map(|entry| {
            let parsed: NormalEntry = serde_json::from_value(entry.clone())
                .expect("Failed to parse problem entry into NormalEntry");
            (parsed.id.clone(), parsed)
        })
        .collect();
    let possible_answer_entries: IndexMap<String, serde_json::Value> = possible_answer_entries
        .iter()
        .map(|entry| {
            let id = entry
                .get("id")
                .expect("Possible answer entry must have an id")
                .as_str()
                .expect("Possible answer entry id is not a string")
                .to_string();
            (id, entry.clone())
        })
        .collect();

    let mut results: Vec<SpecialEvaluationResult> = Vec::new();
    let total_count = result_len;
    let mut correct_count = 0;

    for id in result_entries_parsed.keys() {
        let result_entry = result_entries_parsed.get(id).expect("Missing result entry");
        let possible_answer = possible_answer_entries
            .get(id)
            .expect("Missing possible answer entry");
        let evaluation_result: Result<(), String> = match evaluation_type {
            EvaluationType::SpecialIncomplete | EvaluationType::SpecialErrorParam => {
                evaluate_one_pointing_out(&result_entry.result, possible_answer, evaluation_type)
            }
            EvaluationType::SpecialIrrelevant => {
                evaluate_one_irrelevant(&result_entry.result, possible_answer)
            }
            _ => panic!("Unsupported evaluation type for special evaluation"),
        };
        match evaluation_result {
            Ok(_) => {
                correct_count += 1;
                results.push(SpecialEvaluationResult {
                    id: id.clone(),
                    valid: true,
                    error: None,
                    model_raw_output: result_entry.result.clone(),
                });
            }
            Err(e) => {
                results.push(SpecialEvaluationResult {
                    id: id.clone(),
                    valid: false,
                    error: Some(e),
                    model_raw_output: result_entry.result.clone(),
                });
            }
        };
    }
    // Calculate accuracy
    let accuracy = if total_count == 0 {
        0.0
    } else {
        correct_count as f64 / total_count as f64
    };
    // Insert summary at the beginning
    let summary = json!({
        "accuracy": accuracy,
        "correct_count": correct_count,
        "total_count": total_count,
    });
    let results_serialized: Vec<serde_json::Value> = results
        .into_iter()
        .map(|res| serde_json::to_value(res).expect("Failed to serialize evaluation result"))
        .collect();
    let mut final_results = vec![summary];
    final_results.extend(results_serialized);
    final_results
}

pub fn evaluate_one_pointing_out(
    model_result_raw: &str,
    possible_answer: &serde_json::Value,
    evaluation_type: &EvaluationType,
) -> Result<(), String> {
    let phrase_required = match evaluation_type {
        EvaluationType::SpecialIncomplete => "Missing necessary parameters",
        EvaluationType::SpecialErrorParam => "There is incorrect value",
        _ => panic!("Unsupported evaluation type for pointing out evaluation"),
    };
    if !model_result_raw.contains(phrase_required) {
        return Err(format!(
            "No '{}' found in model output while answering an incomplete question.",
            phrase_required
        ));
    }
    let possible_answer_parsed: PossibleAnswerPointingOutHygienic =
        serde_json::from_value(possible_answer.clone())
            .expect("Failed to parse possible answer into PossibleAnswerPointingOutHygienic");
    for PointingOutHygienic { name, values } in possible_answer_parsed.ground_truth.iter() {
        if !model_result_raw.contains(name) || !values.iter().all(|v| model_result_raw.contains(v))
        {
            return Err(format!(
                "The user's instruction is missing necessary parameters / contains incorrect values ({:?}) for the ({}), but the model failed to correctly point it out",
                values, name
            ));
        }
    }
    Ok(())
}
pub fn evaluate_one_irrelevant(
    model_result_raw: &str,
    possible_answer: &serde_json::Value,
) -> Result<(), String> {
    let _possible_answer_parsed: PossibleAnswerIrrelevantHygienic =
        serde_json::from_value(possible_answer.clone())
            .expect("Failed to parse possible answer into PossibleAnswerIrrelevantHygienic");
    if !model_result_raw.contains("the limitations of the function") {
        return Err("The model failed to identify that the question is irrelevant to the available functions.".to_string());
    }
    Ok(())
}

pub fn evaluate_agent(
    result_entries: &Vec<serde_json::Value>,
    problem_entries: &Vec<serde_json::Value>,
    possible_answer_entries: &Vec<serde_json::Value>,
) -> Vec<serde_json::Value> {
    let result_len = result_entries.len();
    let problem_len = problem_entries.len();
    let possible_answer_len = possible_answer_entries.len();
    assert_eq!(problem_len, possible_answer_len);
    assert_eq!(
        result_len, problem_len,
        "The length of the model result ({}) does not match the length of the prompt ({}) or possible answer ({}). Please check the input files for completeness.",
        result_len, problem_len, possible_answer_len
    );

    // normal single turn should use normal result model
    let result_entries_parsed: IndexMap<String, AgentResultEntry> = result_entries
        .iter()
        .map(|entry| {
            let parsed: AgentResultEntry = serde_json::from_value(entry.clone())
                .expect("Failed to parse result entry into NormalResultEntry");
            (parsed.id.clone(), parsed)
        })
        .collect();
    let problem_entries_parsed: IndexMap<String, AgentEntry> = problem_entries
        .iter()
        .map(|entry| {
            let parsed: AgentEntry = serde_json::from_value(entry.clone())
                .expect("Failed to parse problem entry into NormalEntry");
            (parsed.id.clone(), parsed)
        })
        .collect();
    let possible_answer_entries_parsed: IndexMap<String, PossibleAnswerAgentHygienic> =
        possible_answer_entries
            .iter()
            .map(|entry| {
                let parsed: PossibleAnswerAgentHygienic = serde_json::from_value(entry.clone())
                    .expect(
                        &format!("Failed to parse possible answer entry into PossibleAnswerNormalHygienic: {}", serde_json::to_string(entry).unwrap()),
                    );
                (parsed.id.clone(), parsed)
            })
            .collect();

    let mut results: Vec<AgentEvaluationResult> = Vec::new();
    let total_count = result_len;
    let mut correct_count = 0;

    for id in result_entries_parsed.keys() {
        let result_entry = result_entries_parsed.get(id).expect("Missing result entry");
        let _problem_entry = problem_entries_parsed
            .get(id)
            .expect("Missing problem entry");
        let possible_answer_entry = possible_answer_entries_parsed
            .get(id)
            .expect("Missing possible answer entry");

        match result_entry
            .final_world_state
            .equals_ground_truth(&possible_answer_entry.ground_truth)
        {
            Ok(_) => {
                correct_count += 1;
                results.push(AgentEvaluationResult {
                    id: id.clone(),
                    valid: true,
                    error: None,
                    conversation: result_entry.conversation.clone(),
                    final_world_state: result_entry.final_world_state.clone(),
                    expected_world_state: possible_answer_entry.ground_truth.clone(),
                    output_function_calls: result_entry.output_function_calls.clone(),
                    expected_function_calls: possible_answer_entry.mile_stone.clone(),
                });
            }
            Err(err) => {
                results.push(AgentEvaluationResult {
                    id: id.clone(),
                    valid: false,
                    error: Some(format!(
                        "Model output does not match the ground truth world state: {}",
                        err
                    )),
                    conversation: result_entry.conversation.clone(),
                    final_world_state: result_entry.final_world_state.clone(),
                    expected_world_state: possible_answer_entry.ground_truth.clone(),
                    output_function_calls: result_entry.output_function_calls.clone(),
                    expected_function_calls: possible_answer_entry.mile_stone.clone(),
                });
            }
        }
    }

    // Calculate accuracy
    let accuracy = if total_count == 0 {
        0.0
    } else {
        correct_count as f64 / total_count as f64
    };

    // Insert summary at the beginning
    let summary = json!({
        "accuracy": accuracy,
        "correct_count": correct_count,
        "total_count": total_count,
    });
    let results_serialized: Vec<serde_json::Value> = results
        .into_iter()
        .map(|res| serde_json::to_value(res).expect("Failed to serialize evaluation result"))
        .collect();
    let mut final_results = vec![summary];
    final_results.extend(results_serialized);
    final_results
}
