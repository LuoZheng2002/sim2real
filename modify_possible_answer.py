



import json
import os

import re


base_path = "ACEBench/data_all/data_en"

possible_answer_folder_path = os.path.join(base_path, "possible_answer")
possible_answer_hygienic_folder_path = os.path.join(base_path, "possible_answer_hygienic")

# get all file names under possible_answer_folder_path
possible_answer_file_names = os.listdir(possible_answer_folder_path)

for file_name in possible_answer_file_names:
    # if "agent" in file_name:
    #     continue
    print(f"Processing file: {file_name}")
    file_path = os.path.join(possible_answer_folder_path, file_name)
    hygienic_file_path = os.path.join(possible_answer_hygienic_folder_path, file_name)
    # create directory if not exist
    os.makedirs(os.path.dirname(hygienic_file_path), exist_ok=True)

    with open(file_path, "r") as f_in, open(hygienic_file_path, "w") as f_out:
        for line in f_in:
            line_parsed = json.loads(line)
            ground_truth = line_parsed["ground_truth"]
            if "agent" in file_name:
                assert isinstance(ground_truth, list), f"ground_truth is not a list: {ground_truth}"
                merged = {}
                for gt in ground_truth:
                    for key, value in gt.items():
                        merged[key] = value
                line_parsed["ground_truth"] = merged
                line_hygienic = json.dumps(line_parsed, ensure_ascii=False)
            elif "special_error" in file_name or "special_incomplete" in file_name:
                hygienic_ground_truth = []
                for key, value in ground_truth.items():
                    hygienic_ground_truth.append({"name": key, "values": value})
                line_parsed["ground_truth"] = hygienic_ground_truth
                line_hygienic = json.dumps(line_parsed, ensure_ascii=False)
            elif "special_irrelevant" in file_name:
                assert isinstance(ground_truth, str)
                line_hygienic = json.dumps(line_parsed, ensure_ascii=False)
            else:
                if not isinstance(ground_truth, dict):
                    print(f"warning: ground_truth is not a dict: {ground_truth}, using the first element: {ground_truth[0]}")
                    ground_truth = ground_truth[0]
                    print(f"new ground_truth: {ground_truth}")
                hygienic_ground_truth = []
                for key, value in ground_truth.items():
                    key_trimmed = re.sub(r'_\d+$', '', key)
                    hygienic_ground_truth.append({"name": key_trimmed, "parameters": value})
                line_parsed["ground_truth"] = hygienic_ground_truth
                line_hygienic = json.dumps(line_parsed, ensure_ascii=False)
            f_out.write(line_hygienic + "\n")
    
    


    
