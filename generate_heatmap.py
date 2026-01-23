import argparse
import json
from pathlib import Path

import pandas as pd
import matplotlib.pyplot as plt
import seaborn as sns


def load_accuracies(base_dir: Path):
    """
    Returns a DataFrame with:
        rows    = perturbation names
        columns = dataset names
        values  = accuracy
    """
    records = []
    perturbations = [
        'no_perturbation',
        'action_a',
        'action_b',
        'action_c',
        'action_d',
        'action_e',
        'action_redundant',
        'obs_param_desc',
        'obs_paraphrase',
        'obs_tool_desc',
        'obs_typos',
        'reward_cd',
        'reward_cd_ab',
        'reward_cd_nt',
        'reward_td',
        'reward_td_ab',
        'reward_td_nt',
        'transition'
    ]
    for perturbation in perturbations:
        perturbation_dir = base_dir / perturbation

        for json_file in perturbation_dir.glob("*.json"):
            dataset = json_file.stem

            try:
                with json_file.open("r", encoding="utf-8") as f:
                    first_line = f.readline()
                    obj = json.loads(first_line)
                    accuracy = obj.get("accuracy")
            except Exception as e:
                print(f"Warning: failed to read {json_file}: {e}")
                accuracy = None

            records.append({
                "perturbation": perturbation,
                "dataset": dataset.removeprefix("data_").removesuffix("_evaluation"),
                "accuracy": accuracy,
            })

    df = pd.DataFrame(records)
    df["perturbation"] = pd.Categorical(
        df["perturbation"],
        categories=perturbations,
        ordered=True,
    )
    return df.pivot(index="perturbation", columns="dataset", values="accuracy")


def plot_heatmap(df, model_name, output_path=None):
    if output_path is None:
        output_path = f"{model_name}_accuracy_heatmap.png"

    plt.figure(figsize=(1.2 * len(df.columns), 0.6 * len(df.index) + 2))

    sns.heatmap(
        df,
        annot=True,
        fmt=".3f",
        cmap="RdYlGn",
        vmin=0,
        vmax=1,
        linewidths=0.5,
        cbar_kws={"label": "Accuracy"},
    )

    plt.title(f"Accuracy Heatmap — {model_name}")
    plt.xlabel("Dataset")
    plt.ylabel("Perturbation")
    plt.tight_layout()
    plt.savefig(output_path, dpi=300, bbox_inches="tight")
    plt.close()

    print(f"Saved heatmap to {output_path}")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--model-name",
        required=True,
        help="Model name under acebench_perturbed_score/",
    )
    parser.add_argument(
    "--output",
    default=None,
    help="Output image path (default: <model-name>_accuracy_heatmap.png)",
)
    args = parser.parse_args()

    base_dir = Path("acebench_perturbed_score") / args.model_name

    if not base_dir.exists():
        raise FileNotFoundError(f"Directory not found: {base_dir}")

    df = load_accuracies(base_dir)
    plot_heatmap(df, args.model_name, args.output)


if __name__ == "__main__":
    main()
