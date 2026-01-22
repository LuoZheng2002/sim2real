#!/usr/bin/env python3
"""
Generate LLM-aided perturbations for ACEBench dataset.

This script generates perturbations that require LLM assistance:
- action_redundant: Add semantically similar but different tools as distractors
- obs_typos: Add realistic typos to user queries
- obs_paraphrase: Paraphrase user queries
- obs_tool_desc: Paraphrase tool descriptions
- obs_param_desc: Paraphrase parameter descriptions

Usage:
    python generate_llm_aided_perturbations.py --dataset-folder-path ./acebench_perturbed --model-name gpt-4o-mini

Author: Generated for ACEBench Robustness Testing
"""

import argparse
import asyncio
import copy
from enum import Enum
import json
from pathlib import Path
import random

from dotenv import load_dotenv

from src_py.api_backend import create_api_backend, call_api_model_async


class LlmAidedPerturbationType(Enum):
    ACTION_REDUNDANT = "action_redundant"
    OBS_TYPOS = "obs_typos"
    OBS_PARAPHRASE = "obs_paraphrase"
    OBS_TOOL_DESC = "obs_tool_desc"
    OBS_PARAM_DESC = "obs_param_desc"


NORMAL_DATASET_FILE_NAMES = [
    "data_normal_atom_bool.json",
    "data_normal_atom_enum.json",
    "data_normal_atom_list.json",
    "data_normal_atom_number.json",
    "data_normal_atom_object_deep.json",
    "data_normal_atom_object_short.json",
    "data_normal_multi_turn_user_adjust.json",
    "data_normal_multi_turn_user_switch.json",
    "data_normal_preference.json",
    "data_normal_similar_api.json",
    "data_normal_single_turn_parallel_function.json",
    "data_normal_single_turn_single_function.json",
]

AGENT_DATASET_FILE_NAMES = [
    "data_agent_multi_step.json",
    "data_agent_multi_turn.json",
]

PERTURBATION_TYPE_TO_FOLDER_NAME = {
    LlmAidedPerturbationType.ACTION_REDUNDANT: "action_redundant",
    LlmAidedPerturbationType.OBS_TYPOS: "obs_typos",
    LlmAidedPerturbationType.OBS_PARAPHRASE: "obs_paraphrase",
    LlmAidedPerturbationType.OBS_TOOL_DESC: "obs_tool_desc",
    LlmAidedPerturbationType.OBS_PARAM_DESC: "obs_param_desc",
}

# Global variables for API client, model name, and semaphore
_client = None
_model_name = None
_semaphore = None
_file_locks: dict[Path, asyncio.Lock] = {}
MAX_WORKERS = 200

# ==============================================================================
# Prompts for LLM-aided perturbations
# ==============================================================================

# Prompt for generating similar tools (action_redundant)
GENERATE_SIMILAR_TOOLS_PROMPT = """You are an API designer. Given the following existing tool, generate {num_tools} NEW tools that are semantically related but serve DIFFERENT purposes.

Existing tool:
{existing_tool}

Requirements:
1. The new tools should be plausible extensions that could exist alongside the existing tool
2. They should NOT duplicate existing functionality
3. Each tool should have:
   - A descriptive name following the same naming convention
   - A clear description explaining what it does
   - Appropriate parameters with types and descriptions

Output format (JSON array):
[
  {{
    "name": "tool_name",
    "description": "What this tool does",
    "parameters": {{
      "type": "object",
      "properties": {{
        "param1": {{"type": "string", "description": "Description of param1"}},
        "param2": {{"type": "integer", "description": "Description of param2"}}
      }},
      "required": ["param1"]
    }}
  }}
]

Generate exactly {num_tools} new tools as a JSON array:"""

# Prompt for realistic typos (obs_typos)
REALISTIC_TYPOS_PROMPT = """Add realistic typing errors to the following query, simulating natural human typos.

Original query: {query}

Requirements:
- Add 2-4 realistic typos that humans commonly make when typing quickly
- Include common typo types: adjacent key hits (e→r), character swaps (teh→the), missing letters, doubled letters, common misspellings (definitely→definately)
- DO NOT change any numbers, dates, proper nouns, or technical terms
- DO NOT change the meaning or intent of the query
- The query should still be understandable despite the typos
- Output ONLY the query with typos (no explanation)

Query with typos:"""

# Prompt for query paraphrase (obs_paraphrase)
QUERY_PARAPHRASE_PROMPT = """Paraphrase the following user query while preserving its exact meaning and intent.

Original query: {query}

Requirements:
- Use different wording but keep the same semantic meaning
- DO NOT change any locations, person names, numbers, dates, or specific entities
- Maintain all technical terms and important details
- Output ONLY the paraphrased query (no explanation)

Paraphrased query:"""

# Prompt for tool description paraphrase (obs_tool_desc)
TOOL_DESC_PARAPHRASE_PROMPT = """Paraphrase the following tool/function description while preserving its exact meaning.

Tool name: {tool_name}
Original description: {description}

Requirements:
- Use different wording but keep the same semantic meaning
- Maintain all technical details and constraints
- Keep similar length (±20%)
- Output ONLY the paraphrased description (no explanation)

Paraphrased description:"""

# Prompt for parameter description paraphrase (obs_param_desc)
PARAM_DESC_PARAPHRASE_PROMPT = """Paraphrase the following API parameter description while preserving its exact meaning.

Parameter name: {param_name}
Parameter type: {param_type}
Original description: {description}

Requirements:
- Use different wording but keep the same semantic meaning
- Maintain type constraints and valid values
- Keep similar length
- Output ONLY the paraphrased description (no explanation)

Paraphrased description:"""


# ==============================================================================
# Helper functions
# ==============================================================================

def read_json_lines_from_file(file_path: Path) -> list[dict]:
    """Read a JSON lines file and return a list of dictionaries."""
    with open(file_path, "r") as f:
        return [json.loads(line) for line in f.readlines()]


def load_gt_info_map(gt_file_path: Path) -> dict[str, dict]:
    """Load ground truth info and return a map from id to gt_info."""
    gt_entries = read_json_lines_from_file(gt_file_path)
    return {entry["id"]: entry for entry in gt_entries}


def separate_gt_and_non_gt_tools(
    functions: list[dict], gt_tool_names: set[str]
) -> tuple[list[dict], list[dict]]:
    """Separate functions into GT tools and non-GT tools."""
    gt_tools = []
    non_gt_tools = []
    for func in functions:
        if func["name"] in gt_tool_names:
            gt_tools.append(func)
        else:
            non_gt_tools.append(func)
    return gt_tools, non_gt_tools


def extract_json_from_response(response: str) -> list[dict]:
    """Extract JSON array from LLM response."""
    content = response.strip()

    # Try to extract JSON from markdown code blocks
    if "```json" in content:
        content = content.split("```json")[1].split("```")[0].strip()
    elif "```" in content:
        content = content.split("```")[1].split("```")[0].strip()

    result = json.loads(content)
    if not isinstance(result, list):
        print(f"Error: Expected JSON array but got {type(result)}")
        exit(1)
    return result


def get_file_lock(file_path: Path) -> asyncio.Lock:
    """Get or create an async lock for a file path."""
    if file_path not in _file_locks:
        _file_locks[file_path] = asyncio.Lock()
    return _file_locks[file_path]


def load_existing_entry_ids(output_file_path: Path) -> set[str]:
    """Load entry IDs that have already been processed from output file."""
    if not output_file_path.exists():
        return set()

    existing_ids = set()
    with open(output_file_path, "r") as f:
        for line in f:
            line = line.strip()
            if line:
                entry = json.loads(line)
                existing_ids.add(entry["id"])
    return existing_ids


async def write_result_to_file(result: dict, output_file_path: Path) -> None:
    """Write a single result to file with lock for thread-safe appending."""
    lock = get_file_lock(output_file_path)
    async with lock:
        with open(output_file_path, "a") as f:
            f.write(json.dumps(result, ensure_ascii=False) + "\n")


def get_entry_sort_key(entry_id: str, is_multi_turn: bool) -> tuple:
    """Get sort key for an entry ID based on dataset type.

    For multi_turn datasets: IDs are like "prefix_22_0", sort by (major, minor) at the end
    For other datasets: extract trailing number after the last '_'
    """
    if is_multi_turn:
        # For multi_turn datasets, IDs are like "normal_multi_turn_user_adjust_22_0"
        # Extract the last two underscore-separated numbers
        parts = entry_id.split("_")
        major = int(parts[-2]) if len(parts) >= 2 and parts[-2].isdigit() else 0
        minor = int(parts[-1]) if len(parts) >= 1 and parts[-1].isdigit() else 0
        return (major, minor)
    else:
        # For other datasets, extract trailing number after the last '_'
        parts = entry_id.rsplit("_", 1)
        if len(parts) > 1 and parts[1].isdigit():
            return (int(parts[1]),)
        return (0,)


def sort_output_file(output_file_path: Path, is_multi_turn: bool) -> None:
    """Sort entries in output file by ID and rewrite."""
    if not output_file_path.exists():
        return

    # Read all entries
    entries = []
    with open(output_file_path, "r") as f:
        for line in f:
            line = line.strip()
            if line:
                entries.append(json.loads(line))

    # Sort by ID
    entries.sort(key=lambda e: get_entry_sort_key(e["id"], is_multi_turn))

    # Rewrite file
    with open(output_file_path, "w") as f:
        for entry in entries:
            f.write(json.dumps(entry, ensure_ascii=False) + "\n")


# ==============================================================================
# Perturbation generators
# ==============================================================================

async def generate_similar_tools(
    gt_tool: dict,
    num_tools: int = 2,
) -> list[dict]:
    """Generate similar tools using LLM for action_redundant perturbation."""
    # Format the existing tool for the prompt
    existing_tool_str = json.dumps(gt_tool, ensure_ascii=False, indent=2)

    prompt = GENERATE_SIMILAR_TOOLS_PROMPT.format(
        existing_tool=existing_tool_str,
        num_tools=num_tools,
    )

    system_prompt = "You are an expert API designer. Output only valid JSON. Create tools with UNIQUE names that don't duplicate any existing tool names."

    response = await call_api_model_async(
        client=_client,
        model_name=_model_name,
        system_prompt=system_prompt,
        user_prompt=prompt,
    )

    similar_tools = extract_json_from_response(response)
    return similar_tools


async def generate_typos(query: str) -> str:
    """Generate realistic typos for obs_typos perturbation."""
    prompt = REALISTIC_TYPOS_PROMPT.format(query=query)
    system_prompt = "You are a helpful assistant for generating high-quality data perturbations."

    response = await call_api_model_async(
        client=_client,
        model_name=_model_name,
        system_prompt=system_prompt,
        user_prompt=prompt,
    )

    typo_query = response.strip()
    return typo_query


async def generate_paraphrase(query: str) -> str:
    """Generate paraphrased query for obs_paraphrase perturbation."""
    prompt = QUERY_PARAPHRASE_PROMPT.format(query=query)
    system_prompt = "You are a helpful assistant for generating high-quality data perturbations."

    response = await call_api_model_async(
        client=_client,
        model_name=_model_name,
        system_prompt=system_prompt,
        user_prompt=prompt,
    )

    paraphrased = response.strip()
    return paraphrased


async def generate_tool_desc_paraphrase(tool_name: str, description: str) -> str:
    """Generate paraphrased tool description for obs_tool_desc perturbation."""
    prompt = TOOL_DESC_PARAPHRASE_PROMPT.format(
        tool_name=tool_name,
        description=description,
    )
    system_prompt = "You are a helpful assistant for generating high-quality data perturbations."

    response = await call_api_model_async(
        client=_client,
        model_name=_model_name,
        system_prompt=system_prompt,
        user_prompt=prompt,
    )

    paraphrased = response.strip()
    return paraphrased


async def generate_param_desc_paraphrase(
    param_name: str,
    param_type: str,
    description: str,
) -> str:
    """Generate paraphrased parameter description for obs_param_desc perturbation."""
    prompt = PARAM_DESC_PARAPHRASE_PROMPT.format(
        param_name=param_name,
        param_type=param_type,
        description=description,
    )
    system_prompt = "You are a helpful assistant for generating high-quality data perturbations."

    response = await call_api_model_async(
        client=_client,
        model_name=_model_name,
        system_prompt=system_prompt,
        user_prompt=prompt,
    )

    paraphrased = response.strip()
    return paraphrased


# ==============================================================================
# Entry-level perturbation functions
# ==============================================================================

async def perturb_entry_action_redundant(
    entry: dict,
    gt_info: dict,
    is_agent: bool = False,
) -> dict:
    """Apply action_redundant perturbation to an entry."""
    result = copy.deepcopy(entry)
    functions = result["function"]

    if is_agent:
        # For agent datasets, use mile_stone to identify GT tools
        mile_stone = gt_info["mile_stone"]
        mile_stone_str = json.dumps(mile_stone, ensure_ascii=False)
        gt_tool_names_in_milestone = []
        for func in functions:
            if func["name"] in mile_stone_str:
                gt_tool_names_in_milestone.append(func["name"])

        # Randomly choose one tool to perturb
        chosen_gt_name = random.choice(gt_tool_names_in_milestone)
        gt_tool_names = {chosen_gt_name}
    else:
        # For normal datasets, use ground_truth
        gt_tool_list = gt_info["ground_truth"]
        gt_tool_names = set(tool["name"] for tool in gt_tool_list)

    # Separate GT tools from non-GT tools
    gt_tools, non_gt_tools = separate_gt_and_non_gt_tools(functions, gt_tool_names)

    # Generate similar tools for each GT tool
    all_similar_tools = []
    for gt_tool in gt_tools:
        num_to_generate = random.randint(1, 2)
        similar_tools = await generate_similar_tools(
            gt_tool=gt_tool,
            num_tools=num_to_generate,
        )
        all_similar_tools.extend(similar_tools)

    # Add similar tools to the function list
    result["function"] = non_gt_tools + all_similar_tools + gt_tools

    return result


async def perturb_entry_obs_typos(entry: dict) -> dict:
    """Apply obs_typos perturbation to an entry."""
    result = copy.deepcopy(entry)

    query = result["question"]
    typo_query = await generate_typos(query=query)
    result["question"] = typo_query

    return result


async def perturb_entry_obs_paraphrase(entry: dict) -> dict:
    """Apply obs_paraphrase perturbation to an entry."""
    result = copy.deepcopy(entry)

    query = result["question"]
    paraphrased = await generate_paraphrase(query=query)
    result["question"] = paraphrased

    return result


async def perturb_entry_obs_tool_desc(entry: dict) -> dict:
    """Apply obs_tool_desc perturbation to an entry."""
    result = copy.deepcopy(entry)
    functions = result["function"]

    for func in functions:
        tool_name = func["name"]
        description = func["description"]

        paraphrased = await generate_tool_desc_paraphrase(
            tool_name=tool_name,
            description=description,
        )
        func["description"] = paraphrased

    return result


async def perturb_entry_obs_param_desc(entry: dict) -> dict:
    """Apply obs_param_desc perturbation to an entry."""
    result = copy.deepcopy(entry)
    functions = result["function"]

    for func in functions:
        # Handle both "parameters" and "arguments" keys
        params = func.get("parameters") or func.get("arguments")
        properties = params.get("properties", {})

        for param_name, param_info in properties.items():
            description = param_info.get("description")
            if not description:
                continue

            param_type = param_info.get("type", "string")

            paraphrased = await generate_param_desc_paraphrase(
                param_name=param_name,
                param_type=param_type,
                description=description,
            )
            param_info["description"] = paraphrased

    return result


async def perturb_entry(
    perturbation_type: LlmAidedPerturbationType,
    entry: dict,
    gt_info: dict,
    is_agent: bool = False,
) -> dict:
    """Apply the specified perturbation type to an entry."""
    async with _semaphore:
        if perturbation_type == LlmAidedPerturbationType.ACTION_REDUNDANT:
            return await perturb_entry_action_redundant(
                entry=entry,
                gt_info=gt_info,
                is_agent=is_agent,
            )
        elif perturbation_type == LlmAidedPerturbationType.OBS_TYPOS:
            return await perturb_entry_obs_typos(entry=entry)
        elif perturbation_type == LlmAidedPerturbationType.OBS_PARAPHRASE:
            return await perturb_entry_obs_paraphrase(entry=entry)
        elif perturbation_type == LlmAidedPerturbationType.OBS_TOOL_DESC:
            return await perturb_entry_obs_tool_desc(entry=entry)
        elif perturbation_type == LlmAidedPerturbationType.OBS_PARAM_DESC:
            return await perturb_entry_obs_param_desc(entry=entry)
        else:
            print(f"Error: Unknown perturbation type: {perturbation_type}")
            exit(1)


async def process_dataset_file(
    perturbation_type: LlmAidedPerturbationType,
    dataset_file_path: Path,
    gt_file_path: Path,
    output_file_path: Path,
    is_agent: bool = False,
) -> None:
    """Process a single dataset file and generate perturbed data."""
    # Create output folder
    output_file_path.parent.mkdir(parents=True, exist_ok=True)

    # Load existing entry IDs to skip already-processed entries
    existing_ids = load_existing_entry_ids(output_file_path)

    # Load dataset file and ground truth info
    dataset = read_json_lines_from_file(dataset_file_path)
    gt_info_map = load_gt_info_map(gt_file_path)

    # Filter out already-processed entries
    entries_to_process = [
        entry for entry in dataset if entry["id"] not in existing_ids
    ]

    if not entries_to_process:
        # Still sort the file even if all entries are present
        is_multi_turn = "normal_multi_turn" in output_file_path.name
        sort_output_file(output_file_path, is_multi_turn)
        print(f"Skipping {output_file_path.name} (all {len(dataset)} entries already processed, sorted)")
        return

    print(f"Processing {output_file_path.name}: {len(entries_to_process)} remaining / {len(dataset)} total")

    async def process_and_write(entry: dict) -> None:
        """Process a single entry and write result immediately."""
        result = await perturb_entry(
            perturbation_type=perturbation_type,
            entry=entry,
            gt_info=gt_info_map[entry["id"]],
            is_agent=is_agent,
        )
        await write_result_to_file(result, output_file_path)

    # Process all entries concurrently
    tasks = [process_and_write(entry) for entry in entries_to_process]
    await asyncio.gather(*tasks)

    # Sort output file by entry ID
    is_multi_turn = "normal_multi_turn" in output_file_path.name
    sort_output_file(output_file_path, is_multi_turn)

    print(f"  Completed {output_file_path.name}: {len(entries_to_process)} new entries")


async def main_async(args):
    """Async main function."""
    global _client, _model_name, _semaphore

    print("=" * 60)
    print("LLM-Aided Perturbation Generator for ACEBench")
    print("=" * 60)
    print(f"Model: {args.model_name}")
    print(f"Dataset folder: {args.dataset_folder_path}")
    print(f"Max workers: {MAX_WORKERS}")
    print("=" * 60)

    # Initialize global API client, model name, and semaphore
    _client = create_api_backend(model_name=args.model_name)
    _model_name = args.model_name
    _semaphore = asyncio.Semaphore(MAX_WORKERS)

    # Determine which perturbation types to process
    if args.perturbation_type:
        perturbation_types = [LlmAidedPerturbationType(args.perturbation_type)]
    else:
        perturbation_types = list(LlmAidedPerturbationType)

    # Determine which file names to process
    if args.file_name:
        normal_files = [args.file_name] if args.file_name in NORMAL_DATASET_FILE_NAMES else []
        agent_files = [args.file_name] if args.file_name in AGENT_DATASET_FILE_NAMES else []
    else:
        normal_files = NORMAL_DATASET_FILE_NAMES
        agent_files = AGENT_DATASET_FILE_NAMES

    # Create tasks for all (perturbation_type, file) combinations
    tasks = []

    for perturbation_type in perturbation_types:
        # Normal dataset files
        for file_name in normal_files:
            dataset_file_path = args.dataset_folder_path / "original_modified" / file_name
            gt_file_path = args.dataset_folder_path / "original_modified" / "possible_answer_hygienic" / file_name
            output_file_path = args.dataset_folder_path / PERTURBATION_TYPE_TO_FOLDER_NAME[perturbation_type] / file_name

            if not dataset_file_path.exists():
                print(f"Skipping {file_name} (file not found)")
                continue

            task = asyncio.create_task(
                process_dataset_file(
                    perturbation_type=perturbation_type,
                    dataset_file_path=dataset_file_path,
                    gt_file_path=gt_file_path,
                    output_file_path=output_file_path,
                    is_agent=False,
                )
            )
            tasks.append(task)

        # Agent dataset files
        for file_name in agent_files:
            dataset_file_path = args.dataset_folder_path / "original_modified" / file_name
            gt_file_path = args.dataset_folder_path / "original_modified" / "possible_answer_hygienic" / file_name
            output_file_path = args.dataset_folder_path / PERTURBATION_TYPE_TO_FOLDER_NAME[perturbation_type] / file_name

            if not dataset_file_path.exists():
                print(f"Skipping {file_name} (file not found)")
                continue

            task = asyncio.create_task(
                process_dataset_file(
                    perturbation_type=perturbation_type,
                    dataset_file_path=dataset_file_path,
                    gt_file_path=gt_file_path,
                    output_file_path=output_file_path,
                    is_agent=True,
                )
            )
            tasks.append(task)

    print(f"\nCreated {len(tasks)} tasks for concurrent processing...")

    # Wait for all tasks to complete
    if tasks:
        await asyncio.wait(tasks)

    print("\n" + "=" * 60)
    print("Generation complete!")
    print("=" * 60)


def main():
    # Load environment variables from .env file
    load_dotenv()

    # Parse command line arguments
    parser = argparse.ArgumentParser(
        description="Generate LLM-aided perturbations for ACEBench dataset"
    )
    parser.add_argument(
        "--dataset-folder-path",
        type=Path,
        required=True,
        help="Path to the dataset folder (e.g., ./acebench_perturbed)",
    )
    parser.add_argument(
        "--model-name",
        type=str,
        default="gpt-4.1",
        help="Model name for LLM API (default: gpt-4.1)",
    )
    parser.add_argument(
        "--perturbation-type",
        type=str,
        choices=[p.value for p in LlmAidedPerturbationType],
        help="Specific perturbation type to generate (default: all)",
    )
    parser.add_argument(
        "--file-name",
        type=str,
        help="Specific file name to process (default: all files)",
    )
    args = parser.parse_args()

    # Run async main
    asyncio.run(main_async(args))


if __name__ == "__main__":
    main()