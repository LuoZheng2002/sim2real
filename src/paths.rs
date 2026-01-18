use std::{path::PathBuf, sync::LazyLock};

pub static BASE_DATASET_PATH: LazyLock<PathBuf> =
    LazyLock::new(|| PathBuf::from("ACEBench/data_all/data_en"));

pub static BASE_GROUND_TRUTH_PATH: LazyLock<PathBuf> =
    LazyLock::new(|| PathBuf::from("ACEBench/data_all/data_en/possible_answer"));

pub static BASE_OUTPUT_PATH: LazyLock<PathBuf> =
    LazyLock::new(|| PathBuf::from("result_all/result_en"));

pub static BASE_SCORE_PATH: LazyLock<PathBuf> =
    LazyLock::new(|| PathBuf::from("score_all/score_en"));