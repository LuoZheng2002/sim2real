"""
Error analysis script for ACEBench perturbed evaluation results.

For each failed case, calls gpt-4.1 to analyze the essential reason for failure.

Usage:
    python error_analysis.py --model-name <model_name>
"""

import argparse
import asyncio
import json
import os
from pathlib import Path

from dotenv import load_dotenv
load_dotenv()

from src_py.api_backend import create_api_backend, call_api_model_async

SCORE_DIR = Path("acebench_perturbed_score")
DATA_DIR = Path("acebench_perturbed")
OUTPUT_DIR = Path("acebench_perturbed_analysis")

ANALYZER_MODEL = "gpt-4.1"

# Score folder "no_perturbation" / "transition" map to data folder "original_modified"
PERTURBATION_DATA_MAP = {
    "no_perturbation": "original_modified",
    "transition": "original_modified",
}

SYSTEM_PROMPT = """\
You are an expert error analyst for tool-use / function-calling LLM benchmarks.

You will be given a FAILED test case from the ACEBench benchmark. The case includes:
1. The original question (task the agent was supposed to accomplish)
2. The available tools/functions
3. The agent's conversation trace (what it actually did)
4. The expected vs actual world state or function calls
5. The error message from the evaluator

Your job is to identify the essential root cause of the failure in 20-50 words. Be specific and concise. Focus on what the agent did wrong or failed to do, not on restating the error message.

Respond with ONLY the plain text description, no JSON, no markdown, no extra formatting.
"""


def build_user_prompt(entry: dict, ground_truth: dict | None) -> str:
    parts = []

    # Agent entries have "conversation"; normal entries do not
    if "conversation" in entry:
        parts.append("## Question / Task")
        parts.append(entry["conversation"].split("\n")[0])

        parts.append("\n## Error Message")
        parts.append(entry["error"])

        parts.append("\n## Agent Conversation Trace")
        parts.append(entry["conversation"])

        if "expected_function_calls" in entry:
            parts.append("\n## Expected Function Calls")
            parts.append(json.dumps(entry["expected_function_calls"], indent=2))

        if "output_function_calls" in entry:
            parts.append("\n## Actual Function Calls")
            parts.append(json.dumps(entry["output_function_calls"], indent=2))

        if "expected_world_state" in entry:
            parts.append("\n## Expected World State")
            parts.append(json.dumps(entry["expected_world_state"], indent=2, ensure_ascii=False))

        if "final_world_state" in entry:
            parts.append("\n## Actual World State")
            parts.append(json.dumps(entry["final_world_state"], indent=2, ensure_ascii=False))
    else:
        # Normal entries: atom, single_turn, multi_turn, preference, similar_api
        parts.append("## Error Message")
        parts.append(entry["error"])

        if "model_raw_output" in entry:
            parts.append("\n## Model Raw Output")
            parts.append(entry["model_raw_output"])

        if "possible_answer" in entry:
            parts.append("\n## Expected Answer")
            parts.append(json.dumps(entry["possible_answer"], indent=2, ensure_ascii=False))

        if "turn" in entry:
            parts.append(f"\n## Turn: {entry['turn']}")

    if ground_truth:
        if "function" in ground_truth:
            parts.append("\n## Available Tools (from ground truth)")
            functions = ground_truth["function"]
            tool_summary = [{"name": f["name"], "description": f.get("description", "")} for f in functions]
            parts.append(json.dumps(tool_summary, indent=2, ensure_ascii=False))

        if "initial_config" in ground_truth:
            parts.append("\n## Initial Config")
            parts.append(json.dumps(ground_truth["initial_config"], indent=2, ensure_ascii=False))

        if "question" in ground_truth:
            parts.append("\n## Original Question")
            parts.append(ground_truth["question"])

    return "\n".join(parts)


def load_ground_truth_index(data_file: Path) -> dict[str, dict]:
    """Load ground truth JSONL file into a dict keyed by id."""
    index = {}
    if not data_file.exists():
        return index
    with open(data_file) as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            entry = json.loads(line)
            if "id" in entry:
                index[entry["id"]] = entry
    return index


def load_score_entries(score_file: Path) -> list[dict]:
    """Load score JSONL file, skip the first summary line, return individual entries."""
    entries = []
    with open(score_file) as f:
        for i, line in enumerate(f):
            line = line.strip()
            if not line:
                continue
            entry = json.loads(line)
            if i == 0 and "accuracy" in entry:
                continue  # skip summary line
            entries.append(entry)
    return entries


async def analyze_one(client, entry: dict, ground_truth: dict | None, semaphore: asyncio.Semaphore) -> dict:
    user_prompt = build_user_prompt(entry, ground_truth)
    async with semaphore:
        response = await call_api_model_async(client, ANALYZER_MODEL, SYSTEM_PROMPT, user_prompt)

    return {
        "id": entry["id"],
        "analysis": response.strip(),
    }


def load_existing_analysis(out_file: Path) -> dict[str, dict]:
    """Load existing analysis JSONL file into a dict keyed by id."""
    existing = {}
    if not out_file.exists():
        return existing
    with open(out_file) as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            entry = json.loads(line)
            if "id" in entry:
                existing[entry["id"]] = entry
    return existing


async def process_file(client, score_file: Path, gt_index: dict[str, dict], semaphore: asyncio.Semaphore, out_file: Path) -> list[dict]:
    entries = load_score_entries(score_file)
    failed = [e for e in entries if not e.get("valid", True)]
    if not failed:
        return []

    existing = load_existing_analysis(out_file)

    tasks = []
    reused = []
    for entry in failed:
        entry_id = entry.get("id")
        if entry_id in existing:
            reused.append(existing[entry_id])
        else:
            gt = gt_index.get(entry_id)
            tasks.append(analyze_one(client, entry, gt, semaphore))

    new_results = list(await asyncio.gather(*tasks)) if tasks else []

    # Merge: maintain original order of failed entries
    all_by_id = {r["id"]: r for r in reused + new_results}
    results = [all_by_id[e["id"]] for e in failed if e["id"] in all_by_id]

    if reused:
        print(f"    (reused {len(reused)} existing, analyzed {len(new_results)} new)")

    return results


async def main():
    parser = argparse.ArgumentParser(description="Error analysis for ACEBench perturbed results")
    parser.add_argument("--model-name", required=True, help="Name of the model to analyze (folder name in acebench_perturbed_score)")
    args = parser.parse_args()

    model_name = args.model_name
    score_model_dir = SCORE_DIR / model_name
    if not score_model_dir.exists():
        print(f"Score directory not found: {score_model_dir}")
        print(f"Available models: {[d.name for d in SCORE_DIR.iterdir() if d.is_dir()]}")
        return

    output_dir = OUTPUT_DIR / model_name
    output_dir.mkdir(parents=True, exist_ok=True)

    client = create_api_backend(ANALYZER_MODEL)
    semaphore = asyncio.Semaphore(20)  # limit concurrency

    perturbation_dirs = sorted([d for d in score_model_dir.iterdir() if d.is_dir()])
    total_failed = 0
    total_analyzed = 0

    for perturb_dir in perturbation_dirs:
        perturb_name = perturb_dir.name
        data_perturb_name = PERTURBATION_DATA_MAP.get(perturb_name, perturb_name)
        data_perturb_dir = DATA_DIR / data_perturb_name

        print(f"\n=== Processing perturbation: {perturb_name} ===")

        perturb_output_dir = output_dir / perturb_name
        perturb_output_dir.mkdir(parents=True, exist_ok=True)

        score_files = sorted(perturb_dir.glob("*_evaluation.json"))
        for score_file in score_files:
            # Derive the ground truth file name: data_xxx_evaluation.json -> data_xxx.json
            base_name = score_file.stem.replace("_evaluation", "") + ".json"
            gt_file = data_perturb_dir / base_name

            gt_index = load_ground_truth_index(gt_file)

            out_file = perturb_output_dir / score_file.name
            results = await process_file(client, score_file, gt_index, semaphore, out_file)
            if not results:
                print(f"  {base_name}: no failures")
                continue

            total_failed += len(results)
            total_analyzed += len(results)

            with open(out_file, "w") as f:
                for r in results:
                    f.write(json.dumps(r, ensure_ascii=False) + "\n")

            print(f"  {base_name}: {len(results)} failures analyzed")

    print(f"\nDone. Total failed cases analyzed: {total_analyzed}")
    print(f"Results saved to: {output_dir}")


if __name__ == "__main__":
    asyncio.run(main())
