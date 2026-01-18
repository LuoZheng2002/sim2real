



import json
import os

import re


base_path = "ACEBench/data_all/data_en"

possible_answer_folder_path = os.path.join(base_path, "possible_answer")
possible_answer_hygienic_folder_path = os.path.join(base_path, "possible_answer_hygienic")

# get all file names under possible_answer_folder_path
possible_answer_file_names = os.listdir(possible_answer_folder_path)

for file_name in possible_answer_file_names:
    if "agent" in file_name:
        continue
    file_path = os.path.join(possible_answer_folder_path, file_name)
    hygienic_file_path = os.path.join(possible_answer_hygienic_folder_path, file_name)
    possible_answers = []
    # create directory if not exist
    os.makedirs(os.path.dirname(hygienic_file_path), exist_ok=True)

    with open(file_path, "r") as f_in, open(hygienic_file_path, "w") as f_out:
        for line in f_in:
            line_parsed = json.loads(line)
            ground_truth = line_parsed["ground_truth"]
            assert isinstance(ground_truth, dict), "ground_truth should be a dict"
            hygienic_possible_answers = []
            for key, value in ground_truth.items():
                key_trimmed = re.sub(r'_\d+$', '', key)
                hygienic_possible_answers.append({"name": key, "parameters": value})
            f_out.write(possible_answers)
    
    


    
