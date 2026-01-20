use std::sync::LazyLock;

use indexmap::IndexMap;

use crate::ace_generator::{DatasetTrait, EvaluationType, ProblemType};

pub static DATASETS: LazyLock<IndexMap<String, DatasetTrait>> = LazyLock::new(|| {
    let mut m = IndexMap::new();
    // // agent datasets
    m.insert(
        "data_agent_multi_step".to_string(),
        DatasetTrait {
            problem_type: ProblemType::AgentMultiStep,
            evaluation_type: EvaluationType::Agent,
        },
    );
    m.insert(
        "data_agent_multi_turn".to_string(),
        DatasetTrait {
            problem_type: ProblemType::AgentMultiTurn,
            evaluation_type: EvaluationType::Agent,
        },
    );
    // normal datasets
    m.insert(
        "data_normal_atom_bool".to_string(),
        DatasetTrait {
            problem_type: ProblemType::SingleTurnNormal,
            evaluation_type: EvaluationType::NormalSingleTurn,
        },
    );
    m.insert(
        "data_normal_atom_enum".to_string(),
        DatasetTrait {
            problem_type: ProblemType::SingleTurnNormal,
            evaluation_type: EvaluationType::NormalSingleTurn,
        },
    );
    m.insert(
        "data_normal_atom_list".to_string(),
        DatasetTrait {
            problem_type: ProblemType::SingleTurnNormal,
            evaluation_type: EvaluationType::NormalSingleTurn,
        },
    );
    m.insert(
        "data_normal_atom_number".to_string(),
        DatasetTrait {
            problem_type: ProblemType::SingleTurnNormal,
            evaluation_type: EvaluationType::NormalSingleTurn,
        },
    );
    m.insert(
        "data_normal_atom_object_deep".to_string(),
        DatasetTrait {
            problem_type: ProblemType::SingleTurnNormal,
            evaluation_type: EvaluationType::NormalSingleTurn,
        },
    );
    m.insert(
        "data_normal_atom_object_short".to_string(),
        DatasetTrait {
            problem_type: ProblemType::SingleTurnNormal,
            evaluation_type: EvaluationType::NormalSingleTurn,
        },
    );
    m.insert(
        "data_normal_multi_turn_user_adjust".to_string(),
        DatasetTrait {
            problem_type: ProblemType::SingleTurnNormal,
            evaluation_type: EvaluationType::NormalMultiTurn,
        },
    );
    m.insert(
        "data_normal_multi_turn_user_switch".to_string(),
        DatasetTrait {
            problem_type: ProblemType::SingleTurnNormal,
            evaluation_type: EvaluationType::NormalMultiTurn,
        },
    );
    m.insert(
        "data_normal_preference".to_string(),
        DatasetTrait {
            problem_type: ProblemType::SingleTurnPreference,
            evaluation_type: EvaluationType::NormalSingleTurn,
        },
    );
    m.insert(
        "data_normal_similar_api".to_string(),
        DatasetTrait {
            problem_type: ProblemType::SingleTurnNormal,
            evaluation_type: EvaluationType::NormalSingleTurn,
        },
    );
    m.insert(
        "data_normal_single_turn_parallel_function".to_string(),
        DatasetTrait {
            problem_type: ProblemType::SingleTurnNormal,
            evaluation_type: EvaluationType::NormalSingleTurn,
        },
    );
    m.insert(
        "data_normal_single_turn_single_function".to_string(),
        DatasetTrait {
            problem_type: ProblemType::SingleTurnNormal,
            evaluation_type: EvaluationType::NormalSingleTurn,
        },
    );
    // special datasets
    m.insert(
        "data_special_error_param".to_string(),
        DatasetTrait {
            problem_type: ProblemType::SingleTurnSpecial,
            evaluation_type: EvaluationType::SpecialErrorParam,
        },
    );
    m.insert(
        "data_special_incomplete".to_string(),
        DatasetTrait {
            problem_type: ProblemType::SingleTurnSpecial,
            evaluation_type: EvaluationType::SpecialIncomplete,
        },
    );
    m.insert(
        "data_special_irrelevant".to_string(),
        DatasetTrait {
            problem_type: ProblemType::SingleTurnSpecial,
            evaluation_type: EvaluationType::SpecialIrrelevant,
        },
    );
    m
});
