import argparse
import copy
from enum import Enum
import json
from pathlib import Path

from dotenv import load_dotenv


class ActionSameNamePerturbationType(Enum):
    SAME_NAME_EMPTY = "action_a"
    SAME_NAME_GT_NO_PARAM = "action_b"
    SAME_NAME_NO_GT_WRONG_PARAM = "action_c"
    SAME_NAME_GT_WRONG_PARAM = "action_d"
    SAME_NAME_OTHER_DESC_WRONG_PARAM = "action_e"


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
    "data_special_error_param.json",
    "data_special_incomplete.json",
    "data_special_irrelevant.json",
]

AGENT_DATASET_FILE_NAMES = [
    "data_agent_multi_step.json",
    "data_agent_multi_turn.json",
]

PERTURBATION_TYPE_TO_FOLDER_NAME = {
    ActionSameNamePerturbationType.SAME_NAME_EMPTY: "action_a",
    ActionSameNamePerturbationType.SAME_NAME_GT_NO_PARAM: "action_b",
    ActionSameNamePerturbationType.SAME_NAME_NO_GT_WRONG_PARAM: "action_c",
    ActionSameNamePerturbationType.SAME_NAME_GT_WRONG_PARAM: "action_d",
    ActionSameNamePerturbationType.SAME_NAME_OTHER_DESC_WRONG_PARAM: "action_e",
}

PERTURBATION_TYPE_TO_DESCRIPTION = {
    ActionSameNamePerturbationType.SAME_NAME_EMPTY: "同名 + 空壳（无描述无参数）",
    ActionSameNamePerturbationType.SAME_NAME_GT_NO_PARAM: "同名 + GT描述 + 无参数",
    ActionSameNamePerturbationType.SAME_NAME_NO_GT_WRONG_PARAM: "同名 + 无描述 + 错误参数",
    ActionSameNamePerturbationType.SAME_NAME_GT_WRONG_PARAM: "同名 + GT描述 + 错误参数",
    ActionSameNamePerturbationType.SAME_NAME_OTHER_DESC_WRONG_PARAM: "同名 + 其他描述 + 错误参数",
}


def find_gt_tool_index(entry: dict, gt_tool_name: str) -> int:
    """Find the index of the ground truth tool in the function list."""
    functions = entry.get("function", [])
    for i, func in enumerate(functions):
        if func.get("name") == gt_tool_name:
            return i
    return -1


def get_wrong_params_from_other_tools(entry: dict, gt_tool_name: str) -> dict:
    """Get parameters from another tool (not the GT tool) to use as wrong params."""
    functions = entry.get("function", [])
    for func in functions:
        if func.get("name") != gt_tool_name:
            # Try to get parameters or arguments (different tools use different keys)
            params = func.get("parameters") or func.get("arguments")
            if params and params.get("properties"):
                return params
    # Fallback: return a simple wrong parameter structure
    return {
        "type": "object",
        "properties": {
            "wrong_param_1": {"type": "string", "description": "An incorrect parameter."},
            "wrong_param_2": {"type": "number", "description": "Another incorrect parameter."},
        },
    }


def get_other_description(entry: dict, gt_tool_name: str) -> str:
    """Get a description from another tool (not the GT tool)."""
    functions = entry.get("function", [])
    for func in functions:
        if func.get("name") != gt_tool_name and func.get("description"):
            return func["description"]
    return "A generic tool for various operations."


def create_distractor_tool(
    perturbation_type: ActionSameNamePerturbationType,
    gt_tool: dict,
    entry: dict,
) -> dict:
    """
    Create a distractor tool based on the perturbation type.

    - Type A (SAME_NAME_EMPTY): 同名 + 空壳（无描述无参数）
    - Type B (SAME_NAME_GT_NO_PARAM): 同名 + GT描述 + 无参数
    - Type C (SAME_NAME_NO_GT_WRONG_PARAM): 同名 + 无描述 + 错误参数
    - Type D (SAME_NAME_GT_WRONG_PARAM): 同名 + GT描述 + 错误参数
    - Type E (SAME_NAME_OTHER_DESC_WRONG_PARAM): 同名 + 其他描述 + 错误参数
    """
    gt_tool_name = gt_tool["name"]
    gt_description = gt_tool.get("description", "")

    distractor = {
        "name": gt_tool_name,
        "distractor_info": {
            "type": perturbation_type.name.lower(),
            "purpose": f"测试模型是否能区分同名但内容不同的干扰工具 ({PERTURBATION_TYPE_TO_DESCRIPTION[perturbation_type]})",
            "is_distractor": True,
        },
    }

    if perturbation_type == ActionSameNamePerturbationType.SAME_NAME_EMPTY:
        # Type A: 同名 + 空壳（无描述无参数）
        distractor["description"] = ""
        distractor["parameters"] = {"type": "object", "properties": {}}

    elif perturbation_type == ActionSameNamePerturbationType.SAME_NAME_GT_NO_PARAM:
        # Type B: 同名 + GT描述 + 无参数
        distractor["description"] = gt_description
        distractor["parameters"] = {"type": "object", "properties": {}}

    elif perturbation_type == ActionSameNamePerturbationType.SAME_NAME_NO_GT_WRONG_PARAM:
        # Type C: 同名 + 无描述 + 错误参数
        distractor["description"] = ""
        distractor["parameters"] = get_wrong_params_from_other_tools(entry, gt_tool_name)

    elif perturbation_type == ActionSameNamePerturbationType.SAME_NAME_GT_WRONG_PARAM:
        # Type D: 同名 + GT描述 + 错误参数
        distractor["description"] = gt_description
        distractor["parameters"] = get_wrong_params_from_other_tools(entry, gt_tool_name)

    elif perturbation_type == ActionSameNamePerturbationType.SAME_NAME_OTHER_DESC_WRONG_PARAM:
        # Type E: 同名 + 其他描述 + 错误参数
        distractor["description"] = get_other_description(entry, gt_tool_name)
        distractor["parameters"] = get_wrong_params_from_other_tools(entry, gt_tool_name)

    return distractor


def generate_one_action_abcde_normal(
    perturbation_type: ActionSameNamePerturbationType,
    entry: dict,
    gt_info: dict,
) -> dict:
    """
    Generate a perturbed entry by adding a distractor tool with the same name as the GT tool.

    Args:
        perturbation_type: The type of perturbation to apply
        entry: The original dataset entry
        gt_info: The ground truth info containing the tool name and parameters

    Returns:
        The perturbed entry with the distractor tool added
    """
    result = copy.deepcopy(entry)

    # Get the GT tool name from ground_truth info
    gt_tools = gt_info.get("ground_truth", [])
    if not gt_tools:
        return result  # No ground truth, return as-is

    gt_tool_name = gt_tools[0]["name"]
    gt_tool_index = find_gt_tool_index(entry, gt_tool_name)

    if gt_tool_index == -1:
        return result  # GT tool not found in function list

    gt_tool = entry["function"][gt_tool_index]

    # Create the distractor tool
    distractor = create_distractor_tool(perturbation_type, gt_tool, entry)

    # Mark the original GT tool
    result["function"][gt_tool_index]["ground_truth_info"] = {"is_correct_tool": True}

    # Insert distractor before the GT tool
    result["function"].insert(gt_tool_index, distractor)

    # Add perturbation metadata
    result["perturbation_type"] = f"action_same_name_{perturbation_type.value}"
    result["perturbation_description"] = PERTURBATION_TYPE_TO_DESCRIPTION[perturbation_type]
    result["expected_behavior"] = "模型应该能够识别并选择有完整描述和参数的正确工具，而不是同名的干扰工具"

    return result

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
    for perturbation_type in ActionSameNamePerturbationType:
        for file_name in NORMAL_DATASET_FILE_NAMES:
            dataset_file_path = args.dataset_folder_path / "original_modified" / file_name
            gt_file_path = args.dataset_folder_path / "original_modified" / "possible_answer_hygienic" / file_name
            output_file_path = args.dataset_folder_path / PERTURBATION_TYPE_TO_FOLDER_NAME[perturbation_type] / file_name
            print(f"Processing file: {dataset_file_path} -> {output_file_path}")

            # Load dataset file and ground truth info
            dataset = read_json_lines_from_file(dataset_file_path)
            gt_info_map = load_gt_info_map(gt_file_path)

            # Create output folder
            output_file_path.parent.mkdir(parents=True, exist_ok=True)

            # Process each entry in the dataset
            with open(output_file_path, "w") as f:
                for entry in dataset:
                    entry_id = entry["id"]
                    gt_info = gt_info_map.get(entry_id, {})
                    result = generate_one_action_abcde_normal(perturbation_type, entry, gt_info)
                    f.write(json.dumps(result) + "\n")



if __name__ == "__main__":
    main()
