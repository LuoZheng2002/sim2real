#!/usr/bin/env python3
"""
Generate redundant_similar_tools for Multi-turn data using OpenAI API

This script generates semantically similar but different tools as distractors,
matching the quality of single-turn redundant_similar_tools data.

Author: Generated for BFCL V3 Robustness Testing
Date: 2026-01-14
"""

import json
import os
import copy
import random
from pathlib import Path
from typing import Dict, List, Optional
from tqdm import tqdm
from openai import OpenAI

# Load API key
from dotenv import load_dotenv
load_dotenv('/u/xzhou10/Robust/.env')

# Configuration
BFCL_DATA_DIR = Path("/work/hdd/beaa/xzhou10/bfcl_repo/gorilla/berkeley-function-call-leaderboard/data")
FUNC_DOC_DIR = BFCL_DATA_DIR / "multi_turn_func_doc"
OUTPUT_DIR = Path("/u/xzhou10/Robust/Robustness_benchmark/perturbations/Action/redundant_similar_tools_multiturn_llm")

# Multi-turn class name to func_doc file mapping
CLASS_TO_FILE = {
    "GorillaFileSystem": "gorilla_file_system.json",
    "MathAPI": "math_api.json",
    "MessageAPI": "message_api.json",
    "TicketAPI": "ticket_api.json",
    "TradingBot": "trading_bot.json",
    "TravelAPI": "travel_booking.json",
    "TwitterAPI": "posting_api.json",
    "VehicleControlAPI": "vehicle_control.json",
}

MULTI_TURN_CATEGORIES = [
    "multi_turn_base",
    "multi_turn_miss_func",
    "multi_turn_miss_param",
    "multi_turn_long_context",
]

# Prompt template for generating similar tools
GENERATE_SIMILAR_TOOLS_PROMPT = """You are an API designer. Given the following existing tools from an API, generate {num_tools} NEW tools that are semantically related but serve DIFFERENT purposes.

Existing tools from {class_name}:
{existing_tools}

Requirements:
1. The new tools should belong to the same API/class ({class_name})
2. They should be plausible extensions of the existing API
3. They should NOT duplicate existing functionality
4. Each tool should have:
   - A descriptive name following the same naming convention
   - A clear description explaining what it does
   - Appropriate parameters with types and descriptions

Output format (JSON array):
[
  {{
    "name": "tool_name",
    "description": "What this tool does",
    "parameters": {{
      "type": "dict",
      "properties": {{
        "param1": {{"type": "string", "description": "Description of param1"}},
        "param2": {{"type": "integer", "description": "Description of param2"}}
      }},
      "required": ["param1"]
    }}
  }}
]

Generate exactly {num_tools} new tools as a JSON array:"""


class RedundantToolsGenerator:
    """Generate redundant similar tools using OpenAI API"""

    def __init__(self, api_key: str, model: str = "gpt-4o-mini"):
        self.client = OpenAI(api_key=api_key)
        self.model = model
        self.cache = {}  # Cache generated tools per class
        random.seed(42)

    def load_class_tools(self, class_name: str) -> List[Dict]:
        """Load existing tools for a class"""
        filename = CLASS_TO_FILE.get(class_name)
        if not filename:
            return []

        filepath = FUNC_DOC_DIR / filename
        if not filepath.exists():
            return []

        tools = []
        with open(filepath, 'r') as f:
            for line in f:
                line = line.strip()
                if line:
                    try:
                        tools.append(json.loads(line))
                    except json.JSONDecodeError:
                        continue
        return tools

    def format_tools_for_prompt(self, tools: List[Dict], max_tools: int = 5) -> str:
        """Format tools for the prompt"""
        # Select a subset of tools to avoid too long prompts
        selected = tools[:max_tools] if len(tools) > max_tools else tools

        formatted = []
        for tool in selected:
            tool_str = f"- {tool.get('name', 'unknown')}: {tool.get('description', 'No description')[:100]}"
            formatted.append(tool_str)

        return "\n".join(formatted)

    def generate_similar_tools(self, class_name: str, existing_tools: List[Dict],
                               num_tools: int = 2) -> List[Dict]:
        """Generate similar tools using LLM"""
        # Check cache first - return deep copy to avoid modification issues
        cache_key = f"{class_name}_{num_tools}"
        if cache_key in self.cache:
            return copy.deepcopy(self.cache[cache_key])

        tools_str = self.format_tools_for_prompt(existing_tools)
        existing_names = {t.get('name', '').lower() for t in existing_tools}

        prompt = GENERATE_SIMILAR_TOOLS_PROMPT.format(
            class_name=class_name,
            existing_tools=tools_str,
            num_tools=num_tools
        )

        try:
            response = self.client.chat.completions.create(
                model=self.model,
                messages=[
                    {"role": "system", "content": "You are an expert API designer. Output only valid JSON. Create tools with UNIQUE names that don't duplicate any existing tool names."},
                    {"role": "user", "content": prompt}
                ],
                max_tokens=2000,
                temperature=0.8,
            )

            content = response.choices[0].message.content.strip()

            # Extract JSON from response
            if "```json" in content:
                content = content.split("```json")[1].split("```")[0].strip()
            elif "```" in content:
                content = content.split("```")[1].split("```")[0].strip()

            generated_tools = json.loads(content)

            # Validate: filter out tools with duplicate names
            if isinstance(generated_tools, list):
                valid_tools = []
                for tool in generated_tools:
                    tool_name = tool.get('name', '').lower()
                    if tool_name and tool_name not in existing_names:
                        valid_tools.append(tool)
                        existing_names.add(tool_name)
                    else:
                        print(f"  Filtered duplicate tool: {tool.get('name')}")

                if valid_tools:
                    self.cache[cache_key] = valid_tools
                    return copy.deepcopy(valid_tools)

        except Exception as e:
            print(f"Error generating tools for {class_name}: {e}")

        return []

    def generate_for_sample(self, sample: Dict, all_class_tools: Dict[str, List[Dict]],
                            gt_tool_names: List[str]) -> Optional[Dict]:
        """Generate redundant similar tools for a sample"""
        perturbed = copy.deepcopy(sample)

        # Get tools for this sample's involved classes
        involved_classes = sample.get('involved_classes', [])
        sample_tools = []
        for class_name in involved_classes:
            sample_tools.extend(all_class_tools.get(class_name, []))

        perturbed['function'] = copy.deepcopy(sample_tools)

        # Generate similar tools for each involved class
        added_tools = []
        for class_name in involved_classes:
            class_tools = all_class_tools.get(class_name, [])
            if not class_tools:
                continue

            # Generate 1-2 similar tools per class
            num_to_generate = random.randint(1, 2)
            similar_tools = self.generate_similar_tools(class_name, class_tools, num_to_generate)

            for tool in similar_tools:
                # Add class context to description
                if 'description' in tool:
                    tool['description'] = f"[{class_name}] " + tool['description']
                added_tools.append(tool)

        if not added_tools:
            return None

        # Add generated tools to the sample
        perturbed['function'].extend(added_tools)

        return perturbed


def load_ground_truth(category: str) -> Dict[str, List]:
    """Load ground truth for a category"""
    gt_file = BFCL_DATA_DIR / "possible_answer" / f"BFCL_v3_{category}.json"
    gt_data = {}
    if gt_file.exists():
        with open(gt_file, 'r') as f:
            for line in f:
                line = line.strip()
                if line:
                    entry = json.loads(line)
                    gt_data[entry['id']] = entry.get('ground_truth', [])
    return gt_data


def get_gt_tool_names(ground_truth: List) -> List[str]:
    """Extract tool names from ground truth"""
    tool_names = []
    for gt_entry in ground_truth:
        if isinstance(gt_entry, list):
            for func_call in gt_entry:
                if isinstance(func_call, str) and '(' in func_call:
                    func_name = func_call.split('(')[0].strip()
                    tool_names.append(func_name)
    return list(set(tool_names))


def main():
    import argparse

    parser = argparse.ArgumentParser(description="Generate redundant_similar_tools for Multi-turn using LLM")
    parser.add_argument("--category", type=str, default=None, help="Specific category to process")
    parser.add_argument("--limit", type=int, default=None, help="Limit number of samples")
    args = parser.parse_args()

    print("=" * 60)
    print("Redundant Similar Tools Generator (LLM-based)")
    print("=" * 60)

    # Initialize generator
    api_key = os.getenv("OPENAI_API_KEY")
    if not api_key:
        print("Error: OPENAI_API_KEY not found")
        return

    generator = RedundantToolsGenerator(api_key)

    # Load all class tools
    print("\nLoading func_doc tools...")
    all_class_tools = {}
    for class_name in CLASS_TO_FILE.keys():
        tools = generator.load_class_tools(class_name)
        all_class_tools[class_name] = tools
        print(f"  {class_name}: {len(tools)} tools")

    # Create output directory
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

    # Determine categories to process
    categories = [args.category] if args.category else MULTI_TURN_CATEGORIES

    for category in categories:
        print(f"\n{'='*60}")
        print(f"Processing: {category}")
        print(f"{'='*60}")

        # Load samples
        data_file = BFCL_DATA_DIR / f"BFCL_v3_{category}.json"
        if not data_file.exists():
            print(f"Data file not found: {data_file}")
            continue

        samples = []
        with open(data_file, 'r') as f:
            for line in f:
                line = line.strip()
                if line:
                    samples.append(json.loads(line))

        if args.limit:
            samples = samples[:args.limit]

        print(f"Loaded {len(samples)} samples")

        # Load ground truth
        gt_data = load_ground_truth(category)
        print(f"Loaded {len(gt_data)} ground truth entries")

        # Generate perturbed samples
        output_file = OUTPUT_DIR / f"BFCL_v3_{category}.jsonl"
        perturbed_samples = []
        skipped = 0

        for sample in tqdm(samples, desc="Generating"):
            gt = gt_data.get(sample['id'], [])
            gt_tool_names = get_gt_tool_names(gt)

            perturbed = generator.generate_for_sample(sample, all_class_tools, gt_tool_names)

            if perturbed:
                perturbed['ground_truth'] = gt
                perturbed['perturbation_type'] = 'redundant_similar_tools'
                perturbed['original_id'] = sample['id']
                perturbed['id'] = f"redundant_{category}_{sample['id']}"
                perturbed_samples.append(perturbed)
            else:
                skipped += 1

        # Save
        with open(output_file, 'w') as f:
            for s in perturbed_samples:
                f.write(json.dumps(s) + '\n')

        print(f"Generated {len(perturbed_samples)} samples (skipped: {skipped})")
        print(f"  -> {output_file}")

    print("\n" + "=" * 60)
    print("Generation Complete!")
    print("=" * 60)


if __name__ == "__main__":
    main()
