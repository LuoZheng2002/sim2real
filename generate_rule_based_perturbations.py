import argparse
import copy
from enum import Enum
import json
from pathlib import Path
import random

from dotenv import load_dotenv

# Global storage for wrong params and descriptions from previous tool calls
_previous_wrong_params: dict | None = None
_previous_other_description: str | None = None


class RuleBasedPerturbationType(Enum):
    # Action: Same Name Tools variants
    SAME_NAME_EMPTY = "action_a"
    SAME_NAME_GT_NO_PARAM = "action_b"
    SAME_NAME_NO_GT_WRONG_PARAM = "action_c"
    SAME_NAME_GT_WRONG_PARAM = "action_d"
    SAME_NAME_OTHER_DESC_WRONG_PARAM = "action_e"
    # Reward: Cost Description variants
    COST_MISLEADING_SUFFIX = "reward_cd"  # GT: original name, Distractor: _Budget suffix (misleading)
    COST_ABBREVIATED = "reward_cd_ab"  # GT: abbreviated name, Distractor: full name with _1
    COST_NEUTRAL = "reward_cd_nt"  # GT: original name, Distractor: _1 suffix (neutral)
    # Reward: Time Description variants
    TIME_MISLEADING_SUFFIX = "reward_td"  # GT: original name, Distractor: _Fast suffix (misleading)
    TIME_ABBREVIATED = "reward_td_ab"  # GT: abbreviated name, Distractor: full name with _1
    TIME_NEUTRAL = "reward_td_nt"  # GT: original name, Distractor: _1 suffix (neutral)


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
    # "data_special_error_param.json",
    # "data_special_incomplete.json",
    # "data_special_irrelevant.json",
]

AGENT_DATASET_FILE_NAMES = [
    "data_agent_multi_step.json",
    "data_agent_multi_turn.json",
]

PERTURBATION_TYPE_TO_FOLDER_NAME = {
    # Action perturbations
    RuleBasedPerturbationType.SAME_NAME_EMPTY: "action_a",
    RuleBasedPerturbationType.SAME_NAME_GT_NO_PARAM: "action_b",
    RuleBasedPerturbationType.SAME_NAME_NO_GT_WRONG_PARAM: "action_c",
    RuleBasedPerturbationType.SAME_NAME_GT_WRONG_PARAM: "action_d",
    RuleBasedPerturbationType.SAME_NAME_OTHER_DESC_WRONG_PARAM: "action_e",
    # Reward perturbations
    RuleBasedPerturbationType.COST_MISLEADING_SUFFIX: "reward_cd",
    RuleBasedPerturbationType.COST_ABBREVIATED: "reward_cd_ab",
    RuleBasedPerturbationType.COST_NEUTRAL: "reward_cd_nt",
    RuleBasedPerturbationType.TIME_MISLEADING_SUFFIX: "reward_td",
    RuleBasedPerturbationType.TIME_ABBREVIATED: "reward_td_ab",
    RuleBasedPerturbationType.TIME_NEUTRAL: "reward_td_nt",
}

PERTURBATION_TYPE_TO_DESCRIPTION = {
    # Action perturbations
    RuleBasedPerturbationType.SAME_NAME_EMPTY: "同名 + 空壳（无描述无参数）",
    RuleBasedPerturbationType.SAME_NAME_GT_NO_PARAM: "同名 + GT描述 + 无参数",
    RuleBasedPerturbationType.SAME_NAME_NO_GT_WRONG_PARAM: "同名 + 无描述 + 错误参数",
    RuleBasedPerturbationType.SAME_NAME_GT_WRONG_PARAM: "同名 + GT描述 + 错误参数",
    RuleBasedPerturbationType.SAME_NAME_OTHER_DESC_WRONG_PARAM: "同名 + 其他描述 + 错误参数",
    # Reward perturbations
    RuleBasedPerturbationType.COST_MISLEADING_SUFFIX: "成本描述 + 误导性后缀(_Budget)",
    RuleBasedPerturbationType.COST_ABBREVIATED: "成本描述 + GT缩写名称",
    RuleBasedPerturbationType.COST_NEUTRAL: "成本描述 + 中性后缀(_1)",
    RuleBasedPerturbationType.TIME_MISLEADING_SUFFIX: "时间描述 + 误导性后缀(_Fast)",
    RuleBasedPerturbationType.TIME_ABBREVIATED: "时间描述 + GT缩写名称",
    RuleBasedPerturbationType.TIME_NEUTRAL: "时间描述 + 中性后缀(_1)",
}


def separate_gt_and_non_gt_tools(
    functions: list[dict], gt_tool_names: set[str]
) -> tuple[list[dict], list[dict]]:
    """
    Separate functions into GT tools and non-GT tools.

    Args:
        functions: List of all function definitions
        gt_tool_names: Set of ground truth tool names

    Returns:
        Tuple of (gt_tools, non_gt_tools)
    """
    gt_tools = []
    non_gt_tools = []
    for func in functions:
        if func["name"] in gt_tool_names:
            gt_tools.append(func)
        else:
            non_gt_tools.append(func)
    return gt_tools, non_gt_tools


def get_wrong_params_from_non_gt_tools(non_gt_tools: list[dict]) -> dict:
    """Get parameters from a non-GT tool to use as wrong params.

    If no valid parameters are found in non_gt_tools, falls back to
    previously stored wrong params from earlier tool calls.
    """
    global _previous_wrong_params

    for func in non_gt_tools:
        # Try to get parameters or arguments (different tools use different keys)
        params = func.get("parameters") or func.get("arguments")
        if params and params.get("properties"):
            # Store for future use
            _previous_wrong_params = params
            return params

    # Fall back to previously stored wrong params
    if _previous_wrong_params is not None:
        print("Warning: No non-GT tool with valid parameters found, using previous wrong params")
        return _previous_wrong_params

    print("Error: No non-GT tool with valid parameters found and no previous wrong params available")
    exit(1)


def get_other_description(non_gt_tools: list[dict]) -> str:
    """Get a description from a non-GT tool.

    If no valid description is found in non_gt_tools, falls back to
    previously stored description from earlier tool calls.
    """
    global _previous_other_description

    for func in non_gt_tools:
        if func.get("description"):
            # Store for future use
            _previous_other_description = func["description"]
            return func["description"]

    # Fall back to previously stored description
    if _previous_other_description is not None:
        print("Warning: No non-GT tool with a description found, using previous description")
        return _previous_other_description

    print("Error: No non-GT tool with a description found and no previous description available")
    exit(1)


def generate_abbreviated_name(full_name: str) -> str:
    """Generate an abbreviated name from a full tool name.

    Examples:
        ProteinRichMealPlanner_generateList -> PRMP_GL
        HomeProjectManager.trackProgress -> HPM_TP
    """
    # Split by common separators
    parts = []
    # First split by _ or .
    for segment in full_name.replace(".", "_").split("_"):
        if segment:
            # Extract capital letters or first letter of each word
            capitals = "".join(c for c in segment if c.isupper())
            if capitals:
                parts.append(capitals)
            else:
                # If no capitals, use first letter uppercase
                parts.append(segment[0].upper())
    return "_".join(parts)


def get_tool_name_mapping(
    perturbation_type: RuleBasedPerturbationType,
    gt_tool_names: set[str],
) -> dict[str, str]:
    """
    Get a mapping from original tool names to new tool names based on perturbation type.

    For COST_ABBREVIATED and TIME_ABBREVIATED, the GT tool name is abbreviated.
    For all other perturbation types, the tool name stays the same.

    Args:
        perturbation_type: The type of perturbation
        gt_tool_names: Set of ground truth tool names

    Returns:
        Dictionary mapping original name -> new name
    """
    if perturbation_type in [
        RuleBasedPerturbationType.COST_ABBREVIATED,
        RuleBasedPerturbationType.TIME_ABBREVIATED,
    ]:
        return {name: generate_abbreviated_name(name) for name in gt_tool_names}
    return {name: name for name in gt_tool_names}


def transform_possible_answer_normal(
    perturbation_type: RuleBasedPerturbationType,
    gt_info: dict,
) -> dict:
    """
    Transform a possible answer entry for normal datasets based on the perturbation type.

    Args:
        perturbation_type: The type of perturbation
        gt_info: The original ground truth info

    Returns:
        The transformed ground truth info with updated tool names
    """
    result = copy.deepcopy(gt_info)
    gt_tool_list = result.get("ground_truth", [])

    if not gt_tool_list:
        return result

    # Get tool name mapping
    gt_tool_names = set(tool["name"] for tool in gt_tool_list)
    name_mapping = get_tool_name_mapping(perturbation_type, gt_tool_names)

    # Update tool names in ground_truth
    for tool in result["ground_truth"]:
        original_name = tool["name"]
        tool["name"] = name_mapping.get(original_name, original_name)

    return result


def transform_possible_answer_agent(
    perturbation_type: RuleBasedPerturbationType,
    gt_info: dict,
    perturbed_gt_tool_name: str | None,
    original_gt_tool_name: str | None,
) -> dict:
    """
    Transform a possible answer entry for agent datasets based on the perturbation type.

    For agent datasets, we only perturb one randomly chosen tool, so we need to know
    which tool was perturbed to update the mile_stone correctly.

    Args:
        perturbation_type: The type of perturbation
        gt_info: The original ground truth info
        perturbed_gt_tool_name: The new name of the perturbed GT tool (after perturbation)
        original_gt_tool_name: The original name of the perturbed GT tool

    Returns:
        The transformed ground truth info with updated tool names in mile_stone
    """
    result = copy.deepcopy(gt_info)

    # Only COST_ABBREVIATED and TIME_ABBREVIATED change tool names
    if perturbation_type not in [
        RuleBasedPerturbationType.COST_ABBREVIATED,
        RuleBasedPerturbationType.TIME_ABBREVIATED,
    ]:
        return result

    if not original_gt_tool_name or not perturbed_gt_tool_name:
        return result

    if original_gt_tool_name == perturbed_gt_tool_name:
        return result

    # Update mile_stone by replacing the original tool name with the new one
    mile_stone = result.get("mile_stone", [])
    if mile_stone:
        updated_mile_stone = _replace_tool_name_in_milestone(
            mile_stone, original_gt_tool_name, perturbed_gt_tool_name
        )
        result["mile_stone"] = updated_mile_stone

    return result


def _replace_tool_name_in_milestone(
    mile_stone: list,
    original_name: str,
    new_name: str,
) -> list:
    """
    Replace tool name in mile_stone entries.

    Mile_stone can be:
    - A list of strings: ["[func1(...)]", "[func2(...)]"]
    - A list of lists: [["[func1(...)]", "[func2(...)]"], ["[func1(...)]"]]
    """
    result = []
    for item in mile_stone:
        if isinstance(item, list):
            # Nested list - recurse
            result.append(_replace_tool_name_in_milestone(item, original_name, new_name))
        elif isinstance(item, str):
            # Replace the function name in the string
            # The format is "[func_name(...)]" so we replace "func_name(" with "new_name("
            result.append(item.replace(f"{original_name}(", f"{new_name}("))
        else:
            result.append(item)
    return result


def create_distractor_tool(
    perturbation_type: RuleBasedPerturbationType,
    gt_tool: dict,
    non_gt_tools: list[dict],
) -> tuple[dict, dict]:
    """
    Create a modified GT tool and distractor tool pair based on the perturbation type.

    For action perturbations (SAME_NAME_*):
    - GT tool is returned unchanged
    - Distractor has the same name as GT but different description/params

    For reward perturbations (COST_*, TIME_*):
    - GT tool may have modified name (abbreviated) and metadata added to description
    - Distractor has different name and worse metadata

    Returns:
        Tuple of (modified_gt_tool, distractor_tool)
    """
    gt_tool_name = gt_tool["name"]
    gt_description = gt_tool["description"]

    # Check if this is a reward perturbation (cost or time based)
    is_reward_perturbation = perturbation_type in [
        RuleBasedPerturbationType.COST_MISLEADING_SUFFIX,
        RuleBasedPerturbationType.COST_ABBREVIATED,
        RuleBasedPerturbationType.COST_NEUTRAL,
        RuleBasedPerturbationType.TIME_MISLEADING_SUFFIX,
        RuleBasedPerturbationType.TIME_ABBREVIATED,
        RuleBasedPerturbationType.TIME_NEUTRAL,
    ]

    if is_reward_perturbation:
        # Determine if this is cost or time based
        is_cost = perturbation_type in [
            RuleBasedPerturbationType.COST_MISLEADING_SUFFIX,
            RuleBasedPerturbationType.COST_ABBREVIATED,
            RuleBasedPerturbationType.COST_NEUTRAL,
        ]

        # Set metadata values
        if is_cost:
            gt_metadata = " [Cost: $0.01 per call]"
            distractor_metadata = " [Cost: $0.15 per call]"
        else:
            gt_metadata = " [Response time: ~50ms]"
            distractor_metadata = " [Response time: ~2100ms]"

        # Determine naming based on perturbation type
        if perturbation_type in [
            RuleBasedPerturbationType.COST_MISLEADING_SUFFIX,
            RuleBasedPerturbationType.TIME_MISLEADING_SUFFIX,
        ]:
            # GT: original name, Distractor: misleading suffix
            modified_gt_name = gt_tool_name
            if is_cost:
                distractor_name = f"{gt_tool_name}_Budget"
            else:
                distractor_name = f"{gt_tool_name}_Fast"
        elif perturbation_type in [
            RuleBasedPerturbationType.COST_ABBREVIATED,
            RuleBasedPerturbationType.TIME_ABBREVIATED,
        ]:
            # GT: abbreviated name, Distractor: full name with _1
            modified_gt_name = generate_abbreviated_name(gt_tool_name)
            distractor_name = f"{gt_tool_name}_1"
        else:  # NEUTRAL variants
            # GT: original name, Distractor: _1 suffix
            modified_gt_name = gt_tool_name
            distractor_name = f"{gt_tool_name}_1"

        # Create modified GT tool (deep copy and modify)
        modified_gt_tool = copy.deepcopy(gt_tool)
        modified_gt_tool["name"] = modified_gt_name
        modified_gt_tool["description"] = gt_description + gt_metadata

        # Create distractor tool (empty parameters)
        distractor_tool = {
            "name": distractor_name,
            "description": gt_description + distractor_metadata,
            "parameters": {"type": "object", "properties": {}},
        }

    else:
        # Action perturbations: GT tool unchanged, distractor has same name
        modified_gt_tool = copy.deepcopy(gt_tool)

        distractor_tool = {"name": gt_tool_name}

        if perturbation_type == RuleBasedPerturbationType.SAME_NAME_EMPTY:
            # Type A: 同名 + 空壳（无描述无参数）
            distractor_tool["description"] = ""
            distractor_tool["parameters"] = {"type": "object", "properties": {}}

        elif perturbation_type == RuleBasedPerturbationType.SAME_NAME_GT_NO_PARAM:
            # Type B: 同名 + GT描述 + 无参数
            distractor_tool["description"] = gt_description
            distractor_tool["parameters"] = {"type": "object", "properties": {}}

        elif perturbation_type == RuleBasedPerturbationType.SAME_NAME_NO_GT_WRONG_PARAM:
            # Type C: 同名 + 无描述 + 错误参数
            distractor_tool["description"] = ""
            distractor_tool["parameters"] = get_wrong_params_from_non_gt_tools(non_gt_tools)

        elif perturbation_type == RuleBasedPerturbationType.SAME_NAME_GT_WRONG_PARAM:
            # Type D: 同名 + GT描述 + 错误参数
            distractor_tool["description"] = gt_description
            distractor_tool["parameters"] = get_wrong_params_from_non_gt_tools(non_gt_tools)

        elif perturbation_type == RuleBasedPerturbationType.SAME_NAME_OTHER_DESC_WRONG_PARAM:
            # Type E: 同名 + 其他描述 + 错误参数
            distractor_tool["description"] = get_other_description(non_gt_tools)
            distractor_tool["parameters"] = get_wrong_params_from_non_gt_tools(non_gt_tools)

    return modified_gt_tool, distractor_tool


def generate_one_normal(
    perturbation_type: RuleBasedPerturbationType,
    entry: dict,
    gt_info: dict,
) -> dict:
    """
    Generate a perturbed entry by adding distractor tools for each GT tool.

    Args:
        perturbation_type: The type of perturbation to apply
        entry: The original dataset entry
        gt_info: The ground truth info containing the tool names and parameters

    Returns:
        The perturbed entry with distractor tools added
    """
    result = copy.deepcopy(entry)

    # Get the GT tool names from ground_truth info
    gt_tool_list = gt_info.get("ground_truth", [])
    if not gt_tool_list:
        return result  # No ground truth, return as-is

    gt_tool_names = set(tool["name"] for tool in gt_tool_list)

    # Separate GT tools from non-GT tools
    functions = result.get("function", [])
    gt_tools, non_gt_tools = separate_gt_and_non_gt_tools(functions, gt_tool_names)

    if not gt_tools:
        return result  # No GT tools found in function list

    # Create distractor for each GT tool and pair them together
    perturbed_gt_tools = []
    for gt_tool in gt_tools:
        # Create the modified GT tool and distractor tool
        modified_gt, distractor = create_distractor_tool(perturbation_type, gt_tool, non_gt_tools)
        # Add distractor before modified GT tool
        perturbed_gt_tools.append(distractor)
        perturbed_gt_tools.append(modified_gt)

    # Combine: non-GT tools first, then perturbed GT tools (distractor + modified GT pairs)
    result["function"] = non_gt_tools + perturbed_gt_tools

    return result

def generate_one_agent(
    perturbation_type: RuleBasedPerturbationType,
    entry: dict,
    gt_info: dict,
) -> tuple[dict, str | None, str | None]:
    """
    Generate a perturbed entry for agent datasets by adding a distractor tool
    for one randomly chosen GT function that appears in both the function list
    and the mile_stone.

    Args:
        perturbation_type: The type of perturbation to apply
        entry: The original dataset entry
        gt_info: The ground truth info containing mile_stone

    Returns:
        Tuple of (perturbed_entry, original_tool_name, new_tool_name)
        - perturbed_entry: The perturbed entry with distractor tool added for one GT function
        - original_tool_name: The original name of the perturbed GT tool
        - new_tool_name: The new name of the perturbed GT tool (may be same as original)
    """
    result = copy.deepcopy(entry)

    # Get the mile_stone from ground_truth info and convert to string
    mile_stone = gt_info.get("mile_stone", [])
    if not mile_stone:
        return result, None, None  # No mile_stone, return as-is

    mile_stone_str = json.dumps(mile_stone, ensure_ascii=False)

    # Collect all function names that appear in the mile_stone
    functions = result.get("function", [])
    gt_tool_names_in_milestone = []
    for func in functions:
        func_name = func["name"]
        if func_name in mile_stone_str:
            gt_tool_names_in_milestone.append(func_name)

    if not gt_tool_names_in_milestone:
        return result, None, None  # No GT tools found in mile_stone

    # Randomly choose one tool to perturb
    chosen_gt_name = random.choice(gt_tool_names_in_milestone)

    # Separate the chosen GT tool from other tools
    chosen_gt_tool = None
    other_tools = []

    for func in functions:
        if func["name"] == chosen_gt_name and chosen_gt_tool is None:
            chosen_gt_tool = func
        else:
            other_tools.append(func)

    if not chosen_gt_tool:
        return result, None, None  # Chosen GT tool not found

    # Create modified GT tool and distractor
    modified_gt, distractor = create_distractor_tool(perturbation_type, chosen_gt_tool, other_tools)

    # Combine: other tools first, then distractor, then modified GT tool
    result["function"] = other_tools + [distractor, modified_gt]

    # Return the original and new tool names for possible answer transformation
    return result, chosen_gt_name, modified_gt["name"]


def read_json_lines_from_file(file_path: Path) -> list[dict]:
    """Read a JSON lines file and return a list of dictionaries."""
    with open(file_path, "r") as f:
        return [json.loads(line) for line in f.readlines()]


def load_gt_info_map(gt_file_path: Path) -> dict[str, dict]:
    """Load ground truth info and return a map from id to gt_info."""
    gt_entries = read_json_lines_from_file(gt_file_path)
    return {entry["id"]: entry for entry in gt_entries}


def main():
    # Load environment variables from .env file (if it exists)
    load_dotenv()

    # Parse command line arguments
    parser = argparse.ArgumentParser(description="Generates action 'abcde'")
    parser.add_argument("--dataset-folder-path", type=Path, required=True)
    args = parser.parse_args()

    print(f"Input path: {args.dataset_folder_path}")

    # Iterate over normal dataset files
    for perturbation_type in RuleBasedPerturbationType:
        for file_name in NORMAL_DATASET_FILE_NAMES:
            dataset_file_path = args.dataset_folder_path / "original_modified" / file_name
            gt_file_path = args.dataset_folder_path / "original_modified" / "possible_answer_hygienic" / file_name
            output_file_path = args.dataset_folder_path / PERTURBATION_TYPE_TO_FOLDER_NAME[perturbation_type] / file_name
            output_answer_path = args.dataset_folder_path / PERTURBATION_TYPE_TO_FOLDER_NAME[perturbation_type] / "possible_answer_hygienic" / file_name
            print(f"Processing file: {dataset_file_path} -> {output_file_path}")

            # Load dataset file and ground truth info
            dataset = read_json_lines_from_file(dataset_file_path)
            gt_info_map = load_gt_info_map(gt_file_path)

            # Create output folders
            output_file_path.parent.mkdir(parents=True, exist_ok=True)
            output_answer_path.parent.mkdir(parents=True, exist_ok=True)

            # Process each entry in the dataset
            with open(output_file_path, "w") as f, open(output_answer_path, "w") as f_answer:
                for entry in dataset:
                    entry_id = entry["id"]
                    gt_info = gt_info_map.get(entry_id, {})

                    # Generate perturbed dataset entry
                    result = generate_one_normal(perturbation_type, entry, gt_info)
                    f.write(json.dumps(result, ensure_ascii=False) + "\n")

                    # Generate corresponding possible answer entry
                    answer_result = transform_possible_answer_normal(perturbation_type, gt_info)
                    f_answer.write(json.dumps(answer_result, ensure_ascii=False) + "\n")

    # Iterate over agent dataset files
    for perturbation_type in RuleBasedPerturbationType:
        for file_name in AGENT_DATASET_FILE_NAMES:
            dataset_file_path = args.dataset_folder_path / "original_modified" / file_name
            gt_file_path = args.dataset_folder_path / "original_modified" / "possible_answer_hygienic" / file_name
            output_file_path = args.dataset_folder_path / PERTURBATION_TYPE_TO_FOLDER_NAME[perturbation_type] / file_name
            output_answer_path = args.dataset_folder_path / PERTURBATION_TYPE_TO_FOLDER_NAME[perturbation_type] / "possible_answer_hygienic" / file_name
            print(f"Processing agent file: {dataset_file_path} -> {output_file_path}")

            # Load dataset file and ground truth info
            dataset = read_json_lines_from_file(dataset_file_path)
            gt_info_map = load_gt_info_map(gt_file_path)

            # Create output folders
            output_file_path.parent.mkdir(parents=True, exist_ok=True)
            output_answer_path.parent.mkdir(parents=True, exist_ok=True)

            # Process each entry in the dataset
            with open(output_file_path, "w") as f, open(output_answer_path, "w") as f_answer:
                for entry in dataset:
                    entry_id = entry["id"]
                    gt_info = gt_info_map.get(entry_id, {})

                    # Generate perturbed dataset entry
                    result, original_name, new_name = generate_one_agent(perturbation_type, entry, gt_info)
                    f.write(json.dumps(result, ensure_ascii=False) + "\n")

                    # Generate corresponding possible answer entry
                    answer_result = transform_possible_answer_agent(
                        perturbation_type, gt_info, new_name, original_name
                    )
                    f_answer.write(json.dumps(answer_result, ensure_ascii=False) + "\n")



if __name__ == "__main__":
    main()
