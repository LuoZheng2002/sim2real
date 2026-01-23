use std::{path::PathBuf, sync::LazyLock};

pub static BASE_DATASET_PATH: LazyLock<PathBuf> =
    LazyLock::new(|| PathBuf::from("acebench_perturbed"));

pub static BASE_OUTPUT_PATH: LazyLock<PathBuf> =
    LazyLock::new(|| PathBuf::from("acebench_perturbed_result"));

pub static BASE_SCORE_PATH: LazyLock<PathBuf> =
    LazyLock::new(|| PathBuf::from("acebench_perturbed_score"));