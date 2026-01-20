use std::collections::HashMap;

use indexmap::IndexMap;
use regex::Regex;
use rustpython_parser::{Mode, ast, parse};
use serde::{Deserialize, Serialize};
use serde_json::{json};

use crate::{
    ace_generator::{EvaluationType, NormalResultEntry},
    datasets::DATASETS,
    evaluate_parse::FunctionCallHygienic,
    paths::{BASE_DATASET_PATH, BASE_GROUND_TRUTH_PATH, BASE_OUTPUT_PATH, BASE_SCORE_PATH},
    utils::{load_json_lines, write_json_lines_to_file},
};

pub fn parse_from_string_to_ast(function_calls: &str) -> Result<Vec<ast::Expr>, String> {
    // println!("function calls: {}", function_calls);
    let parsed = rustpython_parser::parse(function_calls, Mode::Expression, "my_source_path");
    let parsed = parsed.map_err(|e| {
        format!(
            "Python function calls parsing failed: invalid syntax: {}",
            e
        )
    })?;
    let ast::Mod::Expression(expr) = parsed else {
        return Err("Python function calls parsing failed: expected an expression".to_string());
    };
    let ast::Expr::List(list_expr) = *expr.body else {
        return Err("Python function calls parsing failed: expected a list expression".to_string());
    };
    Ok(list_expr.elts)
}

pub fn parse_from_ast_to_structured(
    function_calls_ast: &[ast::Expr],
    raw_function_calls: &str,
) -> Result<Vec<FunctionCallHygienic>, String> {
    let mut function_calls = Vec::new();
    for expr in function_calls_ast {
        let ast::Expr::Call(call_expr) = expr else {
            return Err("Expected a function call expression".to_string());
        };
        let func_name = match &*call_expr.func {
            ast::Expr::Name(name_expr) => name_expr.id.clone(),
            _ => {
                return Err(format!(
                    "Unsupported function expression type: {:?}",
                    call_expr.func
                ));
            }
        };
        let mut parameters = IndexMap::new();
        for keyword in &call_expr.keywords {
            if let Some(arg_name) = &keyword.arg {
                let arg_value = ast_expr_to_structured(&keyword.value, raw_function_calls)?;
                parameters.insert(arg_name.to_string(), arg_value);
            }
        }
        // let parameters = serde_json::Value::Object(parameters.into_iter().collect());
        function_calls.push(FunctionCallHygienic {
            name: func_name.to_string(),
            parameters,
        });
    }
    Ok(function_calls)
}

pub fn decode_function_list(function_calls: &str) -> Result<Vec<FunctionCallHygienic>, String> {
    let function_calls_ast = parse_from_string_to_ast(function_calls)?;
    let function_calls_structured = parse_from_ast_to_structured(&function_calls_ast, function_calls)?;
    Ok(function_calls_structured)
}



pub fn ast_expr_to_structured(expr: &ast::Expr, raw_function_calls: &str) -> Result<serde_json::Value, String> {
    match expr {
        ast::Expr::Constant(c) => match &c.value {
            ast::Constant::Str(s) => Ok(serde_json::Value::String(s.to_string())),
            ast::Constant::Int(i) => {
                // Try to convert to i64, fallback to string for big ints
                let val = i.to_string().parse::<i64>().expect(&format!("Failed to parse integer: {}", i));
                Ok(serde_json::Value::Number(serde_json::Number::from(val)))
            }
            ast::Constant::Float(f) => Ok(serde_json::Value::Number(
                serde_json::Number::from_f64(*f).expect(&format!("failed to parse float: {}", f))
            )),
            ast::Constant::Bool(b) => Ok(serde_json::Value::Bool(*b)),
            ast::Constant::None => Ok(serde_json::Value::Null),
            // ast::Constant::Ellipsis => Ok(serde_json::Value::String("...".to_string())),
            // _ => Err(format!("Unsupported constant type: {:?}", c.value)),
            _ => panic!("Unsupported constant type: {:?}", c.value),
        },
        ast::Expr::UnaryOp(u) => {
            match u.op {
                ast::UnaryOp::USub => {
                    let operand = ast_expr_to_structured(&u.operand, raw_function_calls)?;
                    let negated = negate_json_value(&operand).expect("Cannot negate a json value");
                    Ok(negated)
                }
                // _ => Err(format!("Unsupported unary operator: {:?}", u.op)),
                _ => panic!("Unsupported unary operator: {:?}", u.op),
            }
        }
        ast::Expr::List(l) => {
            let items: Result<Vec<serde_json::Value>, String> =
                l.elts.iter().map(|e| ast_expr_to_structured(e, raw_function_calls)).collect();
            Ok(serde_json::Value::Array(items?))
        }
        ast::Expr::Tuple(t) => {
            let items: Result<Vec<serde_json::Value>, String> =
                t.elts.iter().map(|e| ast_expr_to_structured(e, raw_function_calls)).collect();
            Ok(serde_json::Value::Array(items?))
        }
        ast::Expr::Dict(d) => {
            let mut map = serde_json::Map::new();
            for (key_opt, value) in d.keys.iter().zip(d.values.iter()) {
                if let Some(key) = key_opt {
                    let key_val = ast_expr_to_structured(key, raw_function_calls)?;
                    let key_str = match key_val {
                        serde_json::Value::String(s) => s,
                        // _ => key_val.to_string(),
                        _ => panic!("Unsupported dict key type: {:?}", key_val),
                    };
                    let val = ast_expr_to_structured(value, raw_function_calls)?;
                    map.insert(key_str, val);
                }
            }
            Ok(serde_json::Value::Object(map))
        }
        ast::Expr::Name(n) => {
            // Handle True/False/None as names
            match n.id.as_str() {
                "True" | "true" => Ok(serde_json::Value::Bool(true)),
                "False" | "false" => Ok(serde_json::Value::Bool(false)),
                "None" | "null" => Ok(serde_json::Value::Null),
                // other => Ok(serde_json::Value::String(other.to_string())),
                // other => panic!("Unsupported name expression: {}", other),
                _ => return Err(format!("Failed to parse python expression: unsupported name expression: {}", n.id)),
            }
        }
        ast::Expr::Call(c) => {
            // // Handle function calls - extract function name and keyword arguments
            // let func_name = resolve_func_name(&c.func)?;
            // let mut args_map = serde_json::Map::new();

            // for keyword in &c.keywords {
            //     if let Some(ref arg_name) = keyword.arg {
            //         let val = ast_to_structured(&keyword.value)?;
            //         args_map.insert(arg_name.to_string(), val);
            //     }
            // }

            // let mut result = serde_json::Map::new();
            // result.insert(func_name, Value::Object(args_map));
            // Ok(Value::Object(result))
            panic!("Function call expressions are not supported in parameter values: {:?}", c)
        }
        // _ => Err(format!("Unsupported AST type: {:?}", expr)),
        // _ => panic!("Unknown AST type: {:?}, raw function calls: {}", expr, raw_function_calls),
        _ => return Err(format!("Unsupported AST type: {:?}, raw function calls: {}", expr, raw_function_calls)),
    }
}

pub fn negate_json_value(value: &serde_json::Value) -> Result<serde_json::Value, String> {
    match value {
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(serde_json::Value::Number(serde_json::Number::from(-i)))
            } else if let Some(f) = n.as_f64() {
                Ok(serde_json::Value::Number(
                    serde_json::Number::from_f64(-f).unwrap_or(serde_json::Number::from(0)),
                ))
            } else {
                Err("Cannot negate non-numeric value".to_string())
            }
        }
        _ => Err("Cannot negate non-numeric value".to_string()),
    }
}
