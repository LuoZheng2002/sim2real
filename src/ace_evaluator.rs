use std::collections::HashMap;

use regex::Regex;
use rustpython_parser::{Mode, ast, parse};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{
    ace_generator::{EvaluationType, NormalResultEntry},
    datasets::DATASETS,
    paths::{BASE_DATASET_PATH, BASE_GROUND_TRUTH_PATH, BASE_OUTPUT_PATH, BASE_SCORE_PATH},
    utils::{load_json_lines, write_json_lines_to_file},
};

/// Python type mapping equivalent
fn get_expected_json_type(type_desc: &str) -> &'static str {
    match type_desc {
        "string" | "any" => "string",
        "integer" | "int" | "number" => "number",
        "float" => "number",
        "boolean" => "boolean",
        "array" | "tuple" | "list" | "list(string)" | "list(enum)" | "objectArray" => "array",
        "dict" | "object" => "object",
        "enum" => "string",
        _ => "unknown",
    }
}

/// Types that need nested checking
fn is_nested_type(type_desc: &str) -> bool {
    matches!(
        type_desc,
        "array" | "tuple" | "list(string)" | "list(enum)" | "object" | "objectArray"
    )
}

/// Result of evaluation for a single entry
#[derive(Debug, Serialize, Deserialize)]
pub struct EvaluationResult {
    pub id: String,
    pub valid: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub error: Vec<String>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub error_type: String,
    pub model_raw_output: String,
    pub possible_answer: serde_json::Value,
}

/// Summary statistics for evaluation
#[derive(Debug, Serialize, Deserialize)]
pub struct EvaluationSummary {
    pub accuracy: f64,
    pub correct_count: usize,
    pub total_count: usize,
}

/// Extract the outermost bracket content from text
/// Python: extract_outermost_bracket_content
fn extract_outermost_bracket_content(text: &str) -> Option<String> {
    let mut start: Option<usize> = None;
    let mut depth = 0;

    for (i, char) in text.chars().enumerate() {
        if char == '[' {
            if depth == 0 {
                start = Some(i);
            }
            depth += 1;
        } else if char == ']' {
            depth -= 1;
            if depth == 0 {
                if let Some(s) = start {
                    return Some(text[s..=i].to_string());
                }
            }
        }
    }
    None
}

// /// Resolve AST node to serde_json::Value
// /// Python: resolve_ast_by_type
// fn resolve_ast_by_type(expr: &ast::Expr) -> Result<Value, String> {
//     match expr {
//         ast::Expr::Constant(c) => match &c.value {
//             ast::Constant::Str(s) => Ok(Value::String(s.to_string())),
//             ast::Constant::Int(i) => {
//                 // Try to convert to i64, fallback to string for big ints
//                 let val = i.to_string().parse::<i64>().unwrap_or(0);
//                 Ok(Value::Number(serde_json::Number::from(val)))
//             }
//             ast::Constant::Float(f) => Ok(Value::Number(
//                 serde_json::Number::from_f64(*f).unwrap_or(serde_json::Number::from(0)),
//             )),
//             ast::Constant::Bool(b) => Ok(Value::Bool(*b)),
//             ast::Constant::None => Ok(Value::Null),
//             ast::Constant::Ellipsis => Ok(Value::String("...".to_string())),
//             _ => Err(format!("Unsupported constant type: {:?}", c.value)),
//         },
//         ast::Expr::UnaryOp(u) => {
//             if matches!(u.op, ast::UnaryOp::USub) {
//                 let operand = resolve_ast_by_type(&u.operand)?;
//                 match operand {
//                     Value::Number(n) => {
//                         if let Some(i) = n.as_i64() {
//                             Ok(Value::Number(serde_json::Number::from(-i)))
//                         } else if let Some(f) = n.as_f64() {
//                             Ok(Value::Number(
//                                 serde_json::Number::from_f64(-f)
//                                     .unwrap_or(serde_json::Number::from(0)),
//                             ))
//                         } else {
//                             Err("Cannot negate non-numeric value".to_string())
//                         }
//                     }
//                     _ => Err("Cannot negate non-numeric value".to_string()),
//                 }
//             } else {
//                 Err(format!("Unsupported unary operator: {:?}", u.op))
//             }
//         }
//         ast::Expr::List(l) => {
//             let items: Result<Vec<Value>, String> =
//                 l.elts.iter().map(|e| resolve_ast_by_type(e)).collect();
//             Ok(Value::Array(items?))
//         }
//         ast::Expr::Tuple(t) => {
//             let items: Result<Vec<Value>, String> =
//                 t.elts.iter().map(|e| resolve_ast_by_type(e)).collect();
//             Ok(Value::Array(items?))
//         }
//         ast::Expr::Dict(d) => {
//             let mut map = serde_json::Map::new();
//             for (key_opt, value) in d.keys.iter().zip(d.values.iter()) {
//                 if let Some(key) = key_opt {
//                     let key_val = resolve_ast_by_type(key)?;
//                     let key_str = match key_val {
//                         Value::String(s) => s,
//                         _ => key_val.to_string(),
//                     };
//                     let val = resolve_ast_by_type(value)?;
//                     map.insert(key_str, val);
//                 }
//             }
//             Ok(Value::Object(map))
//         }
//         ast::Expr::Name(n) => {
//             // Handle True/False/None as names
//             match n.id.as_str() {
//                 "True" => Ok(Value::Bool(true)),
//                 "False" => Ok(Value::Bool(false)),
//                 "None" => Ok(Value::Null),
//                 other => Ok(Value::String(other.to_string())),
//             }
//         }
//         ast::Expr::Call(c) => {
//             // Handle function calls - extract function name and keyword arguments
//             let func_name = resolve_func_name(&c.func)?;
//             let mut args_map = serde_json::Map::new();

//             for keyword in &c.keywords {
//                 if let Some(ref arg_name) = keyword.arg {
//                     let val = resolve_ast_by_type(&keyword.value)?;
//                     args_map.insert(arg_name.to_string(), val);
//                 }
//             }

//             let mut result = serde_json::Map::new();
//             result.insert(func_name, Value::Object(args_map));
//             Ok(Value::Object(result))
//         }
//         _ => Err(format!("Unsupported AST type: {:?}", expr)),
//     }
// }

// /// Resolve function name from AST expression (handles nested attributes)
// fn resolve_func_name(expr: &ast::Expr) -> Result<String, String> {
//     match expr {
//         ast::Expr::Name(n) => Ok(n.id.to_string()),
//         ast::Expr::Attribute(a) => {
//             let value_name = resolve_func_name(&a.value)?;
//             Ok(format!("{}.{}", value_name, a.attr))
//         }
//         _ => Err(format!("Cannot resolve function name from: {:?}", expr)),
//     }
// }

// /// Parse Python-style function call list and return decoded output
// /// Python: ast_parse + decode_ast
// fn decode_ast(input_str: &str) -> Result<Vec<Value>, String> {
//     // Parse as Python expression
//     let parsed = parse(input_str, Mode::Expression, "<input>")
//         .map_err(|e| format!("Failed to parse AST: {}", e))?;

//     let ast::Mod::Expression(expr) = parsed else {
//         return Err("Expected expression".to_string());
//     };

//     // The expression should be a list of function calls
//     match expr.body.as_ref() {
//         ast::Expr::List(l) => {
//             let mut extracted = Vec::new();
//             for elem in &l.elts {
//                 if let ast::Expr::Call(c) = elem {
//                     let func_name = resolve_func_name(&c.func)?;
//                     let mut args_dict = serde_json::Map::new();

//                     for keyword in &c.keywords {
//                         if let Some(ref arg_name) = keyword.arg {
//                             let output = resolve_ast_by_type(&keyword.value)?;
//                             args_dict.insert(arg_name.to_string(), output);
//                         }
//                     }

//                     let mut func_obj = serde_json::Map::new();
//                     func_obj.insert(func_name, Value::Object(args_dict));
//                     extracted.push(Value::Object(func_obj));
//                 } else {
//                     return Err(format!("Expected Call, got: {:?}", elem));
//                 }
//             }
//             Ok(extracted)
//         }
//         _ => Err("Expected list expression".to_string()),
//     }
// }

// /// Check if decoded output is valid function call format
// /// Python: is_function_call_format_valid
// fn is_function_call_format_valid(decoded_output: &[Value]) -> bool {
//     for item in decoded_output {
//         if !item.is_object() {
//             return false;
//         }
//     }
//     true
// }

/// Standardize string for comparison (remove punctuation, lowercase)
/// Python: standardize_string
fn standardize_string(input: &str) -> String {
    let re = Regex::new(r"[ ,./\-_*^]").unwrap();
    re.replace_all(input, "").to_lowercase().replace('\'', "\"")
}

/// Count function names in a list of function call dicts
/// Python: sum_key_list
fn sum_key_list(data: &[Value]) -> HashMap<String, usize> {
    let mut counts: HashMap<String, usize> = HashMap::new();
    for item in data {
        if let Some(obj) = item.as_object() {
            for key in obj.keys() {
                *counts.entry(key.clone()).or_insert(0) += 1;
            }
        }
    }
    counts
}

/// Find function description by name
/// Python: find_description
fn find_description<'a>(func_descriptions: &'a [Value], name: &str) -> Option<&'a Value> {
    for desc in func_descriptions {
        if let Some(func_name) = desc.get("name").and_then(|n| n.as_str()) {
            if name.contains(func_name) {
                return Some(desc);
            }
        }
    }
    None
}

/// String checker - compare strings with standardization
/// Python: string_checker
fn string_checker(
    param: &str,
    model_output: &str,
    possible_answer: &str,
    func_name: &str,
    test_category: &str,
) -> (bool, Vec<String>, String) {
    let std_model = standardize_string(model_output);
    let std_answer = standardize_string(possible_answer);

    if test_category.contains("agent") {
        if std_model != std_answer {
            return (
                false,
                vec![format!(
                    "wrong value for parameter ({}) of api ({}): [expected: {}, real: {}]",
                    param, func_name, possible_answer, model_output
                )],
                "value_error:string".to_string(),
            );
        }
    } else if !std_model.contains(&std_answer) {
        return (
            false,
            vec![format!(
                "wrong value for parameter ({}) of api ({}): [expected: {}, real: {}]",
                param, func_name, possible_answer, model_output
            )],
            "value_error:string".to_string(),
        );
    }

    (true, vec![], String::new())
}

/// List checker - compare lists with standardization
/// Python: list_checker
fn list_checker(
    param: &str,
    model_output: &Value,
    possible_answer: &Value,
    func_name: &str,
) -> (bool, Vec<String>, String) {
    let model_arr = match model_output.as_array() {
        Some(a) => a,
        None => {
            return (
                false,
                vec![format!(
                    "wrong type for parameter ({}) of api ({}): expected array",
                    param, func_name
                )],
                "type_error".to_string(),
            );
        }
    };

    let answer_arr = match possible_answer.as_array() {
        Some(a) => a,
        None => {
            return (
                false,
                vec![format!("Invalid answer format for parameter ({})", param)],
                "internal_error".to_string(),
            );
        }
    };

    // Standardize and compare
    let std_model: Vec<String> = model_arr
        .iter()
        .map(|v| {
            if let Some(s) = v.as_str() {
                standardize_string(s)
            } else {
                v.to_string()
            }
        })
        .collect();

    let std_answer: Vec<String> = answer_arr
        .iter()
        .map(|v| {
            if let Some(s) = v.as_str() {
                standardize_string(s)
            } else {
                v.to_string()
            }
        })
        .collect();

    if std_model != std_answer {
        return (
            false,
            vec![format!(
                "wrong value for parameter ({}) of api ({}): [expected {:?}, real: {:?}]",
                param, func_name, possible_answer, model_output
            )],
            "value_error:list/tuple".to_string(),
        );
    }

    (true, vec![], String::new())
}

/// Dict checker - recursively compare dictionaries
/// Python: dict_checker
fn dict_checker(
    param: &str,
    model_output: &Value,
    possible_answer: &Value,
    func_name: &str,
) -> (bool, Vec<String>, String) {
    let model_obj = match model_output.as_object() {
        Some(o) => o,
        None => {
            return (
                false,
                vec![format!(
                    "wrong type for parameter ({}) of api ({}): [expected: object, real: {}]",
                    param, func_name, model_output
                )],
                "type_error".to_string(),
            );
        }
    };

    let answer_obj = match possible_answer.as_object() {
        Some(o) => o,
        None => {
            return (
                false,
                vec![format!("Invalid answer format for parameter ({})", param)],
                "internal_error".to_string(),
            );
        }
    };

    if model_obj.len() != answer_obj.len() {
        return (
            false,
            vec![format!(
                "wrong value for parameter ({}) of api ({}): [expected: {:?}, real: {:?}]",
                param, func_name, possible_answer, model_output
            )],
            "value_error".to_string(),
        );
    }

    for (key, model_val) in model_obj {
        // Normalize true/false strings
        let model_val = if model_val == "true" {
            &Value::Bool(true)
        } else if model_val == "false" {
            &Value::Bool(false)
        } else {
            model_val
        };

        if !answer_obj.contains_key(key) {
            return (
                false,
                vec![format!(
                    "wrong value for parameter ({}) of api ({}): [expected: {:?}, real: {:?}]",
                    param, func_name, possible_answer, model_output
                )],
                "value_error".to_string(),
            );
        }

        let expected_val = &answer_obj[key];

        if expected_val.is_object() {
            let (valid, error, error_type) =
                dict_checker(param, model_val, expected_val, func_name);
            if !valid {
                return (valid, error, error_type);
            }
        } else {
            // Compare values with standardization for strings
            let std_model = if let Some(s) = model_val.as_str() {
                standardize_string(s)
            } else {
                model_val.to_string()
            };

            let std_expected = if let Some(s) = expected_val.as_str() {
                standardize_string(s)
            } else {
                expected_val.to_string()
            };

            if !std_model.contains(&std_expected) {
                return (
                    false,
                    vec![format!(
                        "wrong value for parameter ({}) of api ({}): [expected: {:?}, real: {:?}]",
                        param, func_name, possible_answer, model_output
                    )],
                    "value_error".to_string(),
                );
            }
        }
    }

    (true, vec![], String::new())
}

/// List of dicts checker
/// Python: list_dict_checker
fn list_dict_checker(
    param: &str,
    model_output: &Value,
    possible_answer: &Value,
    func_name: &str,
) -> (bool, Vec<String>, String) {
    let model_arr = match model_output.as_array() {
        Some(a) => a,
        None => {
            return (
                false,
                vec![format!(
                    "wrong type for parameter ({}) of api ({})",
                    param, func_name
                )],
                "type_error".to_string(),
            );
        }
    };

    let answer_arr = match possible_answer.as_array() {
        Some(a) => a,
        None => {
            return (
                false,
                vec![format!("Invalid answer format for parameter ({})", param)],
                "internal_error".to_string(),
            );
        }
    };

    if model_arr.len() != answer_arr.len() {
        return (
            false,
            vec![format!(
                "wrong value for parameter ({}) of api ({}): [expected: {:?}, real: {:?}]",
                param, func_name, possible_answer, model_output
            )],
            "value_error:list_dict_count".to_string(),
        );
    }

    for (model_item, answer_item) in model_arr.iter().zip(answer_arr.iter()) {
        let (valid, error, error_type) = dict_checker(param, model_item, answer_item, func_name);
        if !valid {
            return (valid, error, error_type);
        }
    }

    (true, vec![], String::new())
}

/// Simple function checker - check a single function call against expected answer
/// Python: simple_function_checker
fn simple_function_checker(
    func_description: &Value,
    model_output: &Value,
    possible_answers: &Value,
    question: &str,
    test_category: &str,
) -> (bool, Vec<String>, String) {
    let func_name = func_description
        .get("name")
        .and_then(|n| n.as_str())
        .unwrap_or("");

    // Get model output object
    let model_obj = match model_output.as_object() {
        Some(o) => o,
        None => {
            return (
                false,
                vec!["Invalid model output format".to_string()],
                "wrong_output_format".to_string(),
            );
        }
    };

    // Get possible answer
    let possible_answer = match possible_answers.as_object() {
        Some(o) => o
            .values()
            .next()
            .cloned()
            .unwrap_or(Value::Object(serde_json::Map::new())),
        None => {
            return (
                false,
                vec!["Invalid answer format".to_string()],
                "internal_error".to_string(),
            );
        }
    };

    let possible_answer_obj = match possible_answer.as_object() {
        Some(o) => o,
        None => {
            return (
                false,
                vec!["Invalid answer format".to_string()],
                "internal_error".to_string(),
            );
        }
    };

    // Handle empty parameters case
    let model_params = model_obj
        .values()
        .next()
        .cloned()
        .unwrap_or(Value::Object(serde_json::Map::new()));
    let model_params_obj = model_params.as_object();

    let func_params = func_description.get("parameters");
    let properties = func_params
        .and_then(|p| p.get("properties"))
        .and_then(|p| p.as_object());

    // Empty parameter case
    if model_params_obj.map(|o| o.is_empty()).unwrap_or(true)
        && (func_params.is_none() || properties.map(|p| p.is_empty()).unwrap_or(true))
    {
        return (true, vec![], String::new());
    }

    // Check function name
    if !model_obj.contains_key(func_name) {
        let actual_name = model_obj.keys().next().unwrap_or(&String::new()).clone();
        return (
            false,
            vec![format!(
                "wrong_function: expected {}, real {}",
                func_name, actual_name
            )],
            "wrong_function_name".to_string(),
        );
    }

    let model_params = model_obj.get(func_name).unwrap();
    let model_params_obj = match model_params.as_object() {
        Some(o) => o,
        None => {
            return (
                false,
                vec!["Invalid model params format".to_string()],
                "wrong_output_format".to_string(),
            );
        }
    };

    let param_details = match properties {
        Some(p) => p,
        None => return (true, vec![], String::new()), // No parameters to check
    };

    let required_params = func_params
        .and_then(|p| p.get("required"))
        .and_then(|r| r.as_array())
        .map(|a| a.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
        .unwrap_or_default();

    // Check required parameters
    for param in &required_params {
        if !model_params_obj.contains_key(*param) {
            return (
                false,
                vec![format!("lack required_params: {}", param)],
                "lack_args".to_string(),
            );
        }
    }

    // Check each parameter
    for (param, value) in model_params_obj {
        if !param_details.contains_key(param) || !possible_answer_obj.contains_key(param) {
            return (
                false,
                vec![format!("addition params: {}", param)],
                "addition_args".to_string(),
            );
        }

        let full_param_details = &param_details[param];
        let expected_type = full_param_details
            .get("type")
            .and_then(|t| t.as_str())
            .unwrap_or("string");

        let expected_value = &possible_answer_obj[param];

        // Normalize true/false strings
        let value = if value == "true" {
            &Value::Bool(true)
        } else if value == "false" {
            &Value::Bool(false)
        } else {
            value
        };

        // Type and value checking based on expected type
        match expected_type {
            "object" | "dict" => {
                let (valid, error, error_type) =
                    dict_checker(param, value, expected_value, func_name);
                if !valid {
                    return (false, error, error_type);
                }
            }
            "array" | "list" | "tuple" | "list(string)" | "list(enum)" => {
                // Check if it's array of objects
                let nested_type = full_param_details
                    .get("items")
                    .and_then(|i| i.get("type"))
                    .and_then(|t| t.as_str());

                if nested_type == Some("object") {
                    let (valid, error, error_type) =
                        list_dict_checker(param, value, expected_value, func_name);
                    if !valid {
                        return (false, error, error_type);
                    }
                } else {
                    let (valid, error, error_type) =
                        list_checker(param, value, expected_value, func_name);
                    if !valid {
                        return (false, error, error_type);
                    }
                }
            }
            "string" | "enum" => {
                if let (Some(model_str), Some(answer_str)) =
                    (value.as_str(), expected_value.as_str())
                {
                    let (valid, error, error_type) =
                        string_checker(param, model_str, answer_str, func_name, test_category);
                    if !valid {
                        return (false, error, error_type);
                    }
                }
            }
            _ => {
                // For other types (number, boolean), do direct comparison
                // Allow int to float conversion
                let model_num = value.as_f64();
                let answer_num = expected_value.as_f64();

                if let (Some(m), Some(a)) = (model_num, answer_num) {
                    if (m - a).abs() > 1e-9 {
                        return (
                            false,
                            vec![format!(
                                "wrong value for parameter ({}) of api ({}): [expected: {}, real: {}]",
                                param, func_name, expected_value, value
                            )],
                            "value_error".to_string(),
                        );
                    }
                } else if value != expected_value {
                    return (
                        false,
                        vec![format!(
                            "wrong value for parameter ({}) of api ({}): [expected: {}, real: {}]",
                            param, func_name, expected_value, value
                        )],
                        "value_error".to_string(),
                    );
                }
            }
        }
    }

    (true, vec![], String::new())
}

/// Normal checker - check model output against possible answers
/// Python: normal_checker
fn normal_checker(
    func_descriptions: &[Value],
    model_output: &[Value],
    possible_answers: &Value,
    question: &str,
    test_category: &str,
) -> (bool, Vec<String>, String) {
    let possible_answers_obj = match possible_answers.as_object() {
        Some(o) => o,
        None => {
            return (
                false,
                vec!["Invalid answer format".to_string()],
                "internal_error".to_string(),
            );
        }
    };

    // Check function count
    if model_output.len() != possible_answers_obj.len() {
        return (
            false,
            vec!["The number of functions does not match the answer.".to_string()],
            "wrong functions number".to_string(),
        );
    }

    // Convert possible_answers to list format
    let possible_answers_list: Vec<Value> = possible_answers_obj
        .iter()
        .map(|(k, v)| {
            // Remove trailing _N suffix from key
            let re = Regex::new(r"_\d+$").unwrap();
            let clean_key = re.replace(k, "").to_string();
            json!({ clean_key: v })
        })
        .collect();

    let func_name_list: Vec<String> = possible_answers_obj.keys().cloned().collect();

    // Count function names
    let output_counts = sum_key_list(model_output);
    let answer_counts = sum_key_list(&possible_answers_list);

    // Check for extra functions
    for (name, _) in &output_counts {
        if !answer_counts.contains_key(name) {
            return (
                false,
                vec![format!(
                    "extra function detected: {} is not in the ground truth",
                    name
                )],
                "function_mismatch".to_string(),
            );
        }
    }

    for (name, _) in &answer_counts {
        if !output_counts.contains_key(name) {
            return (
                false,
                vec![format!(
                    "missing function: {} is not in the model output",
                    name
                )],
                "function_mismatch".to_string(),
            );
        }
    }

    // Check function counts match
    for (name, count) in &output_counts {
        let expected = answer_counts.get(name).unwrap_or(&0);
        if count != expected {
            return (
                false,
                vec![format!(
                    "incorrect count for function {}: [expected: {}, actual: {}]",
                    name, expected, count
                )],
                "function_mismatch".to_string(),
            );
        }
    }

    // Check each function
    for (i, possible_answer) in possible_answers_list.iter().enumerate() {
        let func_name = &func_name_list[i];
        let re = Regex::new(r"_\d+$").unwrap();
        let clean_func_name = re.replace(func_name, "").to_string();

        let func_description = find_description(func_descriptions, func_name);

        let func_description = match func_description {
            Some(d) => d,
            None => {
                return (
                    false,
                    vec![format!("Function description not found for: {}", func_name)],
                    "internal_error".to_string(),
                );
            }
        };

        let mut valid_found = false;

        for model_item in model_output {
            let model_keys: Vec<String> = model_item
                .as_object()
                .map(|o| o.keys().cloned().collect())
                .unwrap_or_default();

            if model_keys
                .first()
                .map(|k| k == &clean_func_name)
                .unwrap_or(false)
            {
                let (valid, error, error_type) = simple_function_checker(
                    func_description,
                    model_item,
                    possible_answer,
                    question,
                    test_category,
                );

                if valid {
                    valid_found = true;
                    break;
                } else if !error.is_empty() {
                    // Store the last error
                    if !valid_found {
                        return (false, error, error_type);
                    }
                }
            }
        }

        if !valid_found {
            return (
                false,
                vec![format!(
                    "Parallel function call failed; expected {:?}, real: {:?}",
                    possible_answers_list, model_output
                )],
                "function_mismatch".to_string(),
            );
        }
    }

    (true, vec![], String::new())
}

pub fn evaluate_all_results(model_name: String) {
    let model_safe_name = model_name.replace("/", "-");
    for (dataset_name, dataset_trait) in DATASETS.iter() {
        let problem_path = BASE_DATASET_PATH.join(dataset_name.clone() + ".json");
        let result_path = BASE_OUTPUT_PATH
            .join(model_safe_name.clone())
            .join(dataset_name.clone() + "_result.json");
        let possible_answer_path = BASE_GROUND_TRUTH_PATH.join(dataset_name.clone() + ".json");

        // Skip if result file doesn't exist
        if !result_path.exists() {
            eprintln!("Result file not found: {:?}, skipping...", result_path);
            continue;
        }

        let problem_entries = load_json_lines(&problem_path).expect("Failed to read problem file");
        let result_entries = load_json_lines(&result_path).expect("Failed to read result file");
        let possible_answer_entries =
            load_json_lines(&possible_answer_path).expect("Failed to read possible answer file");

        let evaluation_results: Vec<serde_json::Value> = match dataset_trait.evaluation_type {
            EvaluationType::NormalSingleTurn => evaluate_normal_single_turn(
                &result_entries,
                &problem_entries,
                &possible_answer_entries,
                dataset_name,
            ),
            _ => {
                eprintln!("Evaluation type not implemented for: {}", dataset_name);
                continue;
            }
        };

        let output_evaluation_path = BASE_SCORE_PATH
            .join(model_safe_name.clone())
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

pub fn evaluate_normal_single_turn(
    result_entries: &Vec<serde_json::Value>,
    problem_entries: &Vec<serde_json::Value>,
    possible_answer_entries: &Vec<serde_json::Value>,
    test_category: &str,
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
    let result_entries_parsed: Vec<NormalResultEntry> = result_entries
        .iter()
        .map(|entry| {
            serde_json::from_value(entry.clone())
                .expect("Failed to parse result entry into NormalResultEntry")
        })
        .collect();

    let mut results: Vec<Value> = Vec::new();
    let mut correct_count = 0;

    for i in 0..result_entries.len() {
        let id = problem_entries[i]
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let question = problem_entries[i]
            .get("question")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let model_result_item = result_entries[i]
            .get("result")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let prompt_item = problem_entries[i]
            .get("function")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        let possible_answer_item = possible_answer_entries[i]
            .get("ground_truth")
            .cloned()
            .unwrap_or(Value::Null);

        // Try to extract outermost bracket content and decode AST
        let model_result_raw = match extract_outermost_bracket_content(model_result_item) {
            Some(s) => s,
            None => {
                results.push(json!({
                    "id": id,
                    "valid": false,
                    "error": ["Invalid syntax. No bracket content found."],
                    "error_type": "wrong_output_format",
                    "model_result": model_result_item,
                    "possible_answer": possible_answer_item,
                }));
                continue;
            }
        };

        let decoded_output = match decode_ast(&model_result_raw) {
            Ok(d) => d,
            Err(e) => {
                results.push(json!({
                    "id": id,
                    "valid": false,
                    "error": [format!("Invalid syntax. Failed to decode AST. {}", e)],
                    "error_type": "wrong_output_format",
                    "model_result_raw": model_result_raw,
                    "possible_answer": possible_answer_item,
                }));
                continue;
            }
        };

        // Check if output format is valid
        if !is_function_call_format_valid(&decoded_output) {
            results.push(json!({
                "id": id,
                "valid": false,
                "error": ["The output format does not meet the specified requirements."],
                "error_type": "wrong_output_format",
                "model_result": model_result_raw,
                "possible_answer": possible_answer_item,
            }));
            continue;
        }

        // Handle multiple possible answers
        let possible_answers: Vec<Value> = if possible_answer_item.is_array() {
            possible_answer_item.as_array().cloned().unwrap_or_default()
        } else {
            vec![possible_answer_item.clone()]
        };

        let mut all_errors: Vec<(Vec<String>, String)> = Vec::new();
        let mut found_valid = false;

        for possible_answer in &possible_answers {
            let (valid, error, error_type) = normal_checker(
                &prompt_item,
                &decoded_output,
                possible_answer,
                question,
                test_category,
            );

            if valid {
                correct_count += 1;
                found_valid = true;
                break;
            } else {
                all_errors.push((error, error_type));
            }
        }

        if !found_valid && !all_errors.is_empty() {
            let (error, error_type) = &all_errors[0];
            results.push(json!({
                "id": id,
                "valid": false,
                "error": error,
                "error_type": error_type,
                "model_result": model_result_raw,
                "possible_answer": possible_answers.last().unwrap_or(&Value::Null),
            }));
        }
    }

    // Calculate accuracy
    let accuracy = if result_entries.is_empty() {
        0.0
    } else {
        (correct_count as f64 / result_entries.len() as f64 * 1000.0).round() / 1000.0
    };

    // Insert summary at the beginning
    let summary = json!({
        "accuracy": accuracy,
        "correct_count": correct_count,
        "total_count": result_entries.len(),
    });

    let mut final_results = vec![summary];
    final_results.extend(results);

    final_results
}
