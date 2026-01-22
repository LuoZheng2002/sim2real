#!/usr/bin/env python3
"""
Same-Name Tool Perturbation Generator for BFCL V3

Generates 5 types of same-name tool perturbations to test model robustness
when facing distractor tools with identical names but different content.

Perturbation Types:
A. Same name, no description, no params (empty shell)
B. Same name, has GT description, no params (shell with description)
C. Same name, no description, wrong params
D. Same name, has GT description, wrong params
E. Same name, has other description, wrong params
"""

import json
import random
import copy
from pathlib import Path
from typing import Dict, List, Any
from tqdm import tqdm

class SameNameToolPerturbation:
    """Generator for same-name tool perturbations"""

    def __init__(self, seed: int = 42):
        """
        Initialize perturbation generator

        Args:
            seed: Random seed for reproducibility
        """
        self.seed = seed
        random.seed(seed)

        self.perturbation_types = {
            "same_name_empty": {
                "description": "Same tool name, no description, no params (empty shell)",
                "tier": "A"
            },
            "same_name_desc_only": {
                "description": "Same tool name, has GT description, no params",
                "tier": "B"
            },
            "same_name_wrong_params_no_desc": {
                "description": "Same tool name, no description, wrong params",
                "tier": "C"
            },
            "same_name_gt_desc_wrong_params": {
                "description": "Same tool name, has GT description, wrong params",
                "tier": "D"
            },
            "same_name_other_desc_wrong_params": {
                "description": "Same tool name, has other description, wrong params",
                "tier": "E"
            }
        }

    def get_ground_truth_tool_names(self, ground_truth: List[Any]) -> List[str]:
        """
        Extract tool names from ground truth

        Args:
            ground_truth: Ground truth data (list of dicts)

        Returns:
            List of tool names used in ground truth
        """
        tool_names = []
        for gt_entry in ground_truth:
            if isinstance(gt_entry, dict):
                tool_names.extend(gt_entry.keys())
        return tool_names

    def find_non_gt_tool(self, available_tools: List[Dict], gt_tool_names: List[str]) -> Dict:
        """
        Find a tool that is NOT in the ground truth

        Args:
            available_tools: List of available tools
            gt_tool_names: List of ground truth tool names

        Returns:
            A non-GT tool, or None if all tools are in GT
        """
        non_gt_tools = [tool for tool in available_tools if tool.get("name") not in gt_tool_names]
        if not non_gt_tools:
            return None
        return random.choice(non_gt_tools)

    def find_other_gt_tool(self, available_tools: List[Dict], target_tool_name: str, gt_tool_names: List[str]) -> Dict:
        """
        Find a different GT tool to steal parameters from (NEW: Tier 2 strategy)

        This is used when there are no real non-GT tools, but we have multiple GT tools
        that can "steal" parameters from each other.

        Args:
            available_tools: List of available tools
            target_tool_name: The GT tool we're creating distractor for
            gt_tool_names: List of all GT tool names

        Returns:
            A different GT tool, or None if only one tool exists
        """
        # Find other GT tools (excluding the target tool)
        other_gt_tools = [
            tool for tool in available_tools
            if tool.get("name") in gt_tool_names and tool.get("name") != target_tool_name
        ]

        if not other_gt_tools:
            return None

        return random.choice(other_gt_tools)

    def get_gt_tool(self, available_tools: List[Dict], tool_name: str) -> Dict:
        """
        Get the ground truth tool by name

        Args:
            available_tools: List of available tools
            tool_name: Name of the tool to find

        Returns:
            The GT tool dict
        """
        for tool in available_tools:
            if tool.get("name") == tool_name:
                return tool
        return None

    def perturb_same_name_empty(self, sample: Dict) -> Dict:
        """
        Type A: Add same-name empty shell tool (no description, no params)

        Args:
            sample: Original BFCL sample

        Returns:
            Perturbed sample
        """
        perturbed = copy.deepcopy(sample)

        # Get ground truth tool names
        gt_tool_names = self.get_ground_truth_tool_names(perturbed["ground_truth"])
        if not gt_tool_names:
            return perturbed

        # Pick a random GT tool name
        target_tool_name = random.choice(gt_tool_names)

        # Create empty shell distractor
        distractor = {
            "name": target_tool_name,
            "description": "",  # No description
            "parameters": {
                "type": "dict",
                "properties": {},  # No params
                "required": []
            }
        }

        # Add to available tools
        perturbed["function"].append(distractor)

        return perturbed

    def perturb_same_name_desc_only(self, sample: Dict) -> Dict:
        """
        Type B: Add same-name tool with GT description but no params

        Args:
            sample: Original BFCL sample

        Returns:
            Perturbed sample
        """
        perturbed = copy.deepcopy(sample)

        # Get ground truth tool names
        gt_tool_names = self.get_ground_truth_tool_names(perturbed["ground_truth"])
        if not gt_tool_names:
            return perturbed

        # Pick a random GT tool name
        target_tool_name = random.choice(gt_tool_names)

        # Find the GT tool to copy its description
        gt_tool = self.get_gt_tool(perturbed["function"], target_tool_name)
        if not gt_tool:
            return perturbed

        # Create distractor with description but no params
        distractor = {
            "name": target_tool_name,
            "description": gt_tool.get("description", ""),  # Copy GT description
            "parameters": {
                "type": "dict",
                "properties": {},  # No params
                "required": []
            }
        }

        # Add to available tools
        perturbed["function"].append(distractor)

        return perturbed

    def perturb_same_name_wrong_params_no_desc(self, sample: Dict) -> Dict:
        """
        Type C: Add same-name tool with no description but wrong params

        NEW: Creates distractor for EVERY GT tool (not just one random GT tool)

        Uses tiered strategy:
        - Tier 1: Steal params from real non-GT tool (if available)
        - Tier 2: Steal params from other GT tool (parallel_multiple strategy)
        - Tier 3: Cannot generate (return None)

        Args:
            sample: Original BFCL sample

        Returns:
            Perturbed sample, or None if perturbation cannot be generated
        """
        perturbed = copy.deepcopy(sample)

        # Get ground truth tool names (get unique names)
        gt_tool_names_all = self.get_ground_truth_tool_names(perturbed["ground_truth"])
        if not gt_tool_names_all:
            return None

        # Get unique GT tool names
        gt_tool_names = list(set(gt_tool_names_all))

        # Track how many distractors we successfully create
        distractors_created = 0

        # Create distractor for EACH GT tool
        for target_tool_name in gt_tool_names:
            # Tier 1: Try to find a real non-GT tool
            source_tool = self.find_non_gt_tool(perturbed["function"], gt_tool_names)

            if source_tool is None:
                # Tier 2: Try to find another GT tool to steal params from
                source_tool = self.find_other_gt_tool(perturbed["function"], target_tool_name, gt_tool_names)

            if source_tool is None:
                # Tier 3: Cannot generate for this GT tool (skip it)
                continue

            # Create distractor with wrong params, no description
            distractor = copy.deepcopy(source_tool)
            distractor["name"] = target_tool_name  # Replace with GT tool name
            distractor["description"] = ""  # Remove description

            # Add to available tools
            perturbed["function"].append(distractor)
            distractors_created += 1

        # If we couldn't create any distractors, return None
        if distractors_created == 0:
            return None

        return perturbed

    def perturb_same_name_gt_desc_wrong_params(self, sample: Dict) -> Dict:
        """
        Type D: Add same-name tool with GT description but wrong params

        NEW: Creates distractor for EVERY GT tool (not just one random GT tool)

        Uses tiered strategy:
        - Tier 1: Steal params from real non-GT tool (if available)
        - Tier 2: Steal params from other GT tool (parallel_multiple strategy)
        - Tier 3: Cannot generate (return None)

        Args:
            sample: Original BFCL sample

        Returns:
            Perturbed sample, or None if perturbation cannot be generated
        """
        perturbed = copy.deepcopy(sample)

        # Get ground truth tool names (get unique names)
        gt_tool_names_all = self.get_ground_truth_tool_names(perturbed["ground_truth"])
        if not gt_tool_names_all:
            return None

        # Get unique GT tool names
        gt_tool_names = list(set(gt_tool_names_all))

        # Track how many distractors we successfully create
        distractors_created = 0

        # Create distractor for EACH GT tool
        for target_tool_name in gt_tool_names:
            # Find the GT tool to copy its description
            gt_tool = self.get_gt_tool(perturbed["function"], target_tool_name)
            if not gt_tool:
                continue

            # Tier 1: Try to find a real non-GT tool
            source_tool = self.find_non_gt_tool(perturbed["function"], gt_tool_names)

            if source_tool is None:
                # Tier 2: Try to find another GT tool to steal params from
                source_tool = self.find_other_gt_tool(perturbed["function"], target_tool_name, gt_tool_names)

            if source_tool is None:
                # Tier 3: Cannot generate for this GT tool (skip it)
                continue

            # Create distractor with GT description + wrong params
            distractor = copy.deepcopy(source_tool)
            distractor["name"] = target_tool_name  # GT tool name
            distractor["description"] = gt_tool.get("description", "")  # GT description

            # Add to available tools
            perturbed["function"].append(distractor)
            distractors_created += 1

        # If we couldn't create any distractors, return None
        if distractors_created == 0:
            return None

        return perturbed

    def perturb_same_name_other_desc_wrong_params(self, sample: Dict) -> Dict:
        """
        Type E: Add same-name tool with other tool's description and wrong params

        NEW: Creates distractor for EVERY GT tool (not just one random GT tool)

        Uses tiered strategy:
        - Tier 1: Use real non-GT tool's description & params (if available)
        - Tier 2: Use other GT tool's description & params (parallel_multiple strategy)
        - Tier 3: Cannot generate (return None)

        Args:
            sample: Original BFCL sample

        Returns:
            Perturbed sample, or None if perturbation cannot be generated
        """
        perturbed = copy.deepcopy(sample)

        # Get ground truth tool names (get unique names)
        gt_tool_names_all = self.get_ground_truth_tool_names(perturbed["ground_truth"])
        if not gt_tool_names_all:
            return None

        # Get unique GT tool names
        gt_tool_names = list(set(gt_tool_names_all))

        # Track how many distractors we successfully create
        distractors_created = 0

        # Create distractor for EACH GT tool
        for target_tool_name in gt_tool_names:
            # Tier 1: Try to find a real non-GT tool
            source_tool = self.find_non_gt_tool(perturbed["function"], gt_tool_names)

            if source_tool is None:
                # Tier 2: Try to find another GT tool to use its description & params
                source_tool = self.find_other_gt_tool(perturbed["function"], target_tool_name, gt_tool_names)

            if source_tool is None:
                # Tier 3: Cannot generate for this GT tool (skip it)
                continue

            # Create distractor by copying source tool and renaming
            distractor = copy.deepcopy(source_tool)
            distractor["name"] = target_tool_name  # Replace with GT tool name
            # Keep source tool's description and params

            # Add to available tools
            perturbed["function"].append(distractor)
            distractors_created += 1

        # If we couldn't create any distractors, return None
        if distractors_created == 0:
            return None

        return perturbed

    def perturb_all_types_combined(self, sample: Dict) -> Dict:
        """
        NEW: Combine ALL perturbation types (A/B/C/D/E) in a single sample

        For each GT tool, creates 5 distractors:
        - A: Same name + empty shell (no desc, no params)
        - B: Same name + GT desc + no params
        - C: Same name + no desc + wrong params
        - D: Same name + GT desc + wrong params
        - E: Same name + other desc + wrong params

        Args:
            sample: Original BFCL sample

        Returns:
            Perturbed sample with all distractor types, or None if cannot generate
        """
        perturbed = copy.deepcopy(sample)

        # Get ground truth tool names (get unique names)
        gt_tool_names_all = self.get_ground_truth_tool_names(perturbed["ground_truth"])
        if not gt_tool_names_all:
            return None

        # Get unique GT tool names
        gt_tool_names = list(set(gt_tool_names_all))

        # IMPORTANT: Save original available tools BEFORE adding distractors
        # This prevents find_other_gt_tool from finding already-added distractors
        original_tools = copy.deepcopy(perturbed["function"])

        # Track total distractors created
        total_distractors = 0

        # For each GT tool, create all 5 types of distractors
        for target_tool_name in gt_tool_names:
            # Get the GT tool for A/B/D types (from original tools)
            gt_tool = None
            for tool in original_tools:
                if tool.get("name") == target_tool_name:
                    gt_tool = tool
                    break

            if not gt_tool:
                continue

            # A: Empty shell (no description, no params)
            distractor_a = {
                "name": target_tool_name,
                "description": "",
                "parameters": {
                    "type": "dict",
                    "properties": {},
                    "required": []
                }
            }
            perturbed["function"].append(distractor_a)
            total_distractors += 1

            # B: GT description only (no params)
            distractor_b = {
                "name": target_tool_name,
                "description": gt_tool.get("description", ""),
                "parameters": {
                    "type": "dict",
                    "properties": {},
                    "required": []
                }
            }
            perturbed["function"].append(distractor_b)
            total_distractors += 1

            # For C/D/E: Need a source tool to steal params from
            # IMPORTANT: Search in ORIGINAL tools, not perturbed (which may contain distractors)
            source_tool = self.find_non_gt_tool(original_tools, gt_tool_names)
            if source_tool is None:
                # Find another GT tool from ORIGINAL tools
                other_gt_tools = [t for t in original_tools
                                if t.get("name") in gt_tool_names and t.get("name") != target_tool_name]
                if other_gt_tools:
                    source_tool = random.choice(other_gt_tools)

            if source_tool is not None:
                # C: No description + wrong params
                distractor_c = copy.deepcopy(source_tool)
                distractor_c["name"] = target_tool_name
                distractor_c["description"] = ""
                perturbed["function"].append(distractor_c)
                total_distractors += 1

                # D: GT description + wrong params
                distractor_d = copy.deepcopy(source_tool)
                distractor_d["name"] = target_tool_name
                distractor_d["description"] = gt_tool.get("description", "")
                perturbed["function"].append(distractor_d)
                total_distractors += 1

                # E: Other description + wrong params (keep source tool's description)
                distractor_e = copy.deepcopy(source_tool)
                distractor_e["name"] = target_tool_name
                # Keep source tool's description
                perturbed["function"].append(distractor_e)
                total_distractors += 1

        # If we couldn't create any distractors, return None
        if total_distractors == 0:
            return None

        return perturbed

    def generate_perturbed_dataset(
        self,
        input_file: str,
        gt_file: str,
        output_file: str,
        perturbation_type: str = "all"
    ):
        """
        Generate perturbed dataset for a single category

        Args:
            input_file: Path to clean BFCL data file (e.g., BFCL_v3_simple.json)
            gt_file: Path to ground truth file (e.g., possible_answer/BFCL_v3_simple.json)
            output_file: Path to output perturbed file
            perturbation_type: Which perturbation to apply ("all" or specific type)
        """
        # Read input data
        with open(input_file, 'r') as f:
            samples = [json.loads(line) for line in f]

        print(f"Loaded {len(samples)} samples from {input_file}")

        # Read ground truth data
        gt_data = {}
        with open(gt_file, 'r') as f:
            for line in f:
                gt_entry = json.loads(line)
                gt_data[gt_entry["id"]] = gt_entry["ground_truth"]

        print(f"Loaded {len(gt_data)} ground truth entries from {gt_file}")

        # Merge ground truth into samples
        for sample in samples:
            sample_id = sample["id"]
            if sample_id in gt_data:
                sample["ground_truth"] = gt_data[sample_id]
            else:
                print(f"⚠️ Warning: No ground truth found for {sample_id}")
                sample["ground_truth"] = []

        # Check if using combined mode
        if perturbation_type == "combined":
            # COMBINED MODE: Apply all A/B/C/D/E perturbations to each sample
            print("Using COMBINED mode: All A/B/C/D/E perturbations in each sample")

            perturbed_samples = []

            # Add clean samples first
            for i, sample in enumerate(samples):
                clean_sample = copy.deepcopy(sample)
                clean_sample["perturbation_type"] = "clean"
                clean_sample["perturbation_metadata"] = {
                    "type": "clean",
                    "tier": "Baseline",
                    "description": "Original unperturbed sample"
                }
                clean_sample["sample_id"] = f"clean_{i}"
                clean_sample["original_index"] = i
                perturbed_samples.append(clean_sample)

            # Add combined perturbed samples
            skipped = 0
            print("Applying combined perturbations (A/B/C/D/E)")

            for i, sample in enumerate(tqdm(samples, desc="combined")):
                perturbed = self.perturb_all_types_combined(sample)

                if perturbed is None:
                    skipped += 1
                    continue

                perturbed["perturbation_type"] = "combined"
                perturbed["perturbation_metadata"] = {
                    "type": "combined",
                    "tier": "All",
                    "description": "All perturbation types (A/B/C/D/E) combined in single sample"
                }
                perturbed["sample_id"] = f"combined_{i}"
                perturbed["original_index"] = i
                perturbed["original_id"] = sample["id"]

                perturbed_samples.append(perturbed)

            if skipped > 0:
                print(f"  ⚠️ Skipped {skipped}/{len(samples)} samples (cannot generate perturbation)")

            skipped_counts = {"combined": skipped} if skipped > 0 else {}
            perturbations_to_apply = ["combined"]  # For summary generation

        else:
            # SEPARATE MODE: Apply each perturbation type separately
            # Determine which perturbations to apply
            if perturbation_type == "all":
                perturbations_to_apply = list(self.perturbation_types.keys())
            else:
                perturbations_to_apply = [perturbation_type]

            # Map perturbation type to method
            perturbation_methods = {
                "same_name_empty": self.perturb_same_name_empty,
                "same_name_desc_only": self.perturb_same_name_desc_only,
                "same_name_wrong_params_no_desc": self.perturb_same_name_wrong_params_no_desc,
                "same_name_gt_desc_wrong_params": self.perturb_same_name_gt_desc_wrong_params,
                "same_name_other_desc_wrong_params": self.perturb_same_name_other_desc_wrong_params
            }

            # Generate perturbed samples
            perturbed_samples = []

            # Add clean samples first
            for i, sample in enumerate(samples):
                clean_sample = copy.deepcopy(sample)
                clean_sample["perturbation_type"] = "clean"
                clean_sample["perturbation_metadata"] = {
                    "type": "clean",
                    "tier": "Baseline",
                    "description": "Original unperturbed sample"
                }
                clean_sample["sample_id"] = f"clean_{i}"
                clean_sample["original_index"] = i
                perturbed_samples.append(clean_sample)

            # Add perturbed samples
            skipped_counts = {}  # Track how many samples were skipped per perturbation type

            for pert_type in perturbations_to_apply:
                print(f"Applying perturbation: {pert_type}")
                method = perturbation_methods[pert_type]
                skipped = 0

                for i, sample in enumerate(tqdm(samples, desc=f"{pert_type}")):
                    perturbed = method(sample)

                    # Skip if perturbation cannot be generated (returns None)
                    if perturbed is None:
                        skipped += 1
                        continue

                    perturbed["perturbation_type"] = pert_type
                    perturbed["perturbation_metadata"] = {
                        "type": pert_type,
                        "tier": self.perturbation_types[pert_type]["tier"],
                        "description": self.perturbation_types[pert_type]["description"]
                    }
                    perturbed["sample_id"] = f"{pert_type}_{i}"
                    perturbed["original_index"] = i
                    perturbed["original_id"] = sample["id"]

                    perturbed_samples.append(perturbed)

                if skipped > 0:
                    skipped_counts[pert_type] = skipped
                    print(f"  ⚠️ Skipped {skipped}/{len(samples)} samples (cannot generate this perturbation)")

        # Write output
        output_path = Path(output_file)
        output_path.parent.mkdir(exist_ok=True, parents=True)

        with open(output_file, 'w') as f:
            for sample in perturbed_samples:
                f.write(json.dumps(sample) + '\n')

        print(f"\n✅ Generated {len(perturbed_samples)} perturbed samples")
        print(f"   - Clean: {len(samples)}")
        print(f"   - Perturbed: {len(perturbed_samples) - len(samples)}")
        print(f"   - Output: {output_file}")

        # Generate summary
        self.generate_summary(input_file, output_file, perturbed_samples, perturbations_to_apply, skipped_counts)

    def generate_summary(
        self,
        input_file: str,
        output_file: str,
        perturbed_samples: List[Dict],
        perturbations_applied: List[str],
        skipped_counts: Dict[str, int] = None
    ):
        """
        Generate summary statistics

        Args:
            input_file: Input file path
            output_file: Output file path
            perturbed_samples: List of perturbed samples
            perturbations_applied: List of perturbation types applied
            skipped_counts: Dict of skipped sample counts per perturbation type
        """
        summary = {
            "input_file": input_file,
            "output_file": output_file,
            "total_samples": len(perturbed_samples),
            "perturbation_types": len(perturbations_applied) + 1,  # +1 for clean
            "perturbations": {},
            "skipped": skipped_counts if skipped_counts else {}
        }

        # Count per perturbation type
        for pert_type in ["clean"] + perturbations_applied:
            count = sum(1 for s in perturbed_samples if s["perturbation_type"] == pert_type)
            summary["perturbations"][pert_type] = count

        # Save summary
        summary_file = output_file.replace(".json", "_summary.json")
        with open(summary_file, 'w') as f:
            json.dump(summary, f, indent=2)

        print(f"✅ Summary saved to: {summary_file}")


def main():
    """Main function to generate same-name tool perturbations"""

    # Configuration
    BFCL_DATA_DIR = Path("/work/hdd/beaa/xzhou10/bfcl_repo/gorilla/berkeley-function-call-leaderboard/data")
    BFCL_GT_DIR = BFCL_DATA_DIR / "possible_answer"
    OUTPUT_DIR = Path("/u/xzhou10/Robust/Robustness_benchmark/perturbed_datasets_same_name")

    # Categories to process (Non-Live + Tool Call GT)
    categories = [
        "BFCL_v3_simple",
        "BFCL_v3_multiple",
        "BFCL_v3_parallel",
        "BFCL_v3_parallel_multiple",
        # Multi-turn categories
        "BFCL_v3_multi_turn_base",
        "BFCL_v3_multi_turn_miss_func",
        "BFCL_v3_multi_turn_miss_param",
        "BFCL_v3_multi_turn_long_context",
        # Code generation
        "BFCL_v3_java",
        "BFCL_v3_javascript"
    ]

    # Initialize generator
    generator = SameNameToolPerturbation(seed=42)

    print("=" * 80)
    print("Same-Name Tool Perturbation Generator")
    print("=" * 80)
    print()

    # Process each category
    for category in categories:
        input_file = BFCL_DATA_DIR / f"{category}.json"
        gt_file = BFCL_GT_DIR / f"{category}.json"
        output_file = OUTPUT_DIR / f"{category}_perturbed.json"

        if not input_file.exists():
            print(f"⚠️ Skipping {category} (input file not found)")
            continue

        if not gt_file.exists():
            print(f"⚠️ Skipping {category} (ground truth file not found)")
            continue

        print(f"\n{'='*80}")
        print(f"Processing: {category}")
        print(f"{'='*80}")

        generator.generate_perturbed_dataset(
            str(input_file),
            str(gt_file),
            str(output_file),
            perturbation_type="all"
        )

    print("\n" + "=" * 80)
    print("All Categories Processed!")
    print("=" * 80)


if __name__ == "__main__":
    main()
