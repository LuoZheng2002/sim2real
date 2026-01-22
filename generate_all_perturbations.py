#!/usr/bin/env python3
"""
Comprehensive BFCL V3 Perturbation Generator

This script generates all perturbation types for all applicable BFCL V3 categories.

Usage:
    # Test mode (5 samples per category)
    python generate_all_perturbations.py --test

    # Full generation
    python generate_all_perturbations.py --full

    # Generate specific perturbation type
    python generate_all_perturbations.py --type realistic_typos

    # Generate specific category
    python generate_all_perturbations.py --category simple

Author: Generated for BFCL V3 Robustness Testing
Date: 2026-01-12
"""

import os
import json
import random
import argparse
from pathlib import Path
from typing import Dict, List, Optional, Tuple
from copy import deepcopy
from tqdm import tqdm
from openai import OpenAI

# =============================================================================
# Configuration
# =============================================================================

BFCL_DATA_DIR = Path("/work/hdd/beaa/xzhou10/bfcl_repo/gorilla/berkeley-function-call-leaderboard/data")
OUTPUT_DIR = Path("/u/xzhou10/Robust/Robustness_benchmark/perturbations")

# All BFCL V3 categories
ALL_CATEGORIES = [
    # Non-live single-turn
    "simple", "multiple", "parallel", "parallel_multiple",
    # Language-specific
    "java", "javascript",
    # Live single-turn
    "live_simple", "live_multiple", "live_parallel", "live_parallel_multiple",
    # Multi-turn
    "multi_turn_base", "multi_turn_miss_func", "multi_turn_miss_param", "multi_turn_long_context",
    # Relevance detection
    "irrelevance", "live_irrelevance", "live_relevance",
]

# Categories that can have additional tools added
MULTI_TOOL_CATEGORIES = [
    "multiple", "parallel_multiple",
    "live_multiple", "live_parallel_multiple",
    "multi_turn_base", "multi_turn_miss_func", "multi_turn_miss_param", "multi_turn_long_context",
]

# Categories for transient timeout (excludes irrelevance/relevance)
TRANSITION_CATEGORIES = [
    "simple", "multiple", "parallel", "parallel_multiple",
    "java", "javascript",
    "live_simple", "live_multiple", "live_parallel", "live_parallel_multiple",
    "multi_turn_base", "multi_turn_miss_func", "multi_turn_miss_param", "multi_turn_long_context",
]

# Multi-turn class name to func_doc file mapping
MULTI_TURN_CLASS_TO_FILE = {
    "GorillaFileSystem": "gorilla_file_system.json",
    "MathAPI": "math_api.json",
    "MessageAPI": "message_api.json",
    "TicketAPI": "ticket_api.json",
    "TradingBot": "trading_bot.json",
    "TravelAPI": "travel_booking.json",
    "TwitterAPI": "posting_api.json",
    "VehicleControlAPI": "vehicle_control.json",
}

MULTI_TURN_FUNC_DOC_DIR = BFCL_DATA_DIR / "multi_turn_func_doc"


def load_multi_turn_tools(involved_classes: List[str]) -> List[Dict]:
    """Load tool definitions from multi_turn_func_doc files for given classes"""
    tools = []
    for class_name in involved_classes:
        filename = MULTI_TURN_CLASS_TO_FILE.get(class_name)
        if not filename:
            print(f"Warning: Unknown class {class_name}, skipping")
            continue

        filepath = MULTI_TURN_FUNC_DOC_DIR / filename
        if not filepath.exists():
            print(f"Warning: File {filepath} not found, skipping")
            continue

        with open(filepath, 'r') as f:
            for line in f:
                try:
                    tool = json.loads(line.strip())
                    tools.append(tool)
                except json.JSONDecodeError:
                    continue
    return tools

# QWERTY keyboard adjacent keys for typo generation
QWERTY_ADJACENT = {
    'a': ['s', 'q', 'w', 'z'],
    'b': ['v', 'g', 'h', 'n'],
    'c': ['x', 'd', 'f', 'v'],
    'd': ['s', 'e', 'r', 'f', 'c', 'x'],
    'e': ['w', 's', 'd', 'r'],
    'f': ['d', 'r', 't', 'g', 'v', 'c'],
    'g': ['f', 't', 'y', 'h', 'b', 'v'],
    'h': ['g', 'y', 'u', 'j', 'n', 'b'],
    'i': ['u', 'j', 'k', 'o'],
    'j': ['h', 'u', 'i', 'k', 'm', 'n'],
    'k': ['j', 'i', 'o', 'l', 'm'],
    'l': ['k', 'o', 'p'],
    'm': ['n', 'j', 'k'],
    'n': ['b', 'h', 'j', 'm'],
    'o': ['i', 'k', 'l', 'p'],
    'p': ['o', 'l'],
    'q': ['w', 'a'],
    'r': ['e', 'd', 'f', 't'],
    's': ['a', 'w', 'e', 'd', 'x', 'z'],
    't': ['r', 'f', 'g', 'y'],
    'u': ['y', 'h', 'j', 'i'],
    'v': ['c', 'f', 'g', 'b'],
    'w': ['q', 'a', 's', 'e'],
    'x': ['z', 's', 'd', 'c'],
    'y': ['t', 'g', 'h', 'u'],
    'z': ['a', 's', 'x'],
}

# Common words that can have typos (not entities, numbers, or important terms)
COMMON_WORDS = {
    'the', 'and', 'for', 'with', 'that', 'this', 'from', 'have', 'are', 'was',
    'were', 'been', 'being', 'has', 'had', 'would', 'could', 'should', 'will',
    'can', 'may', 'might', 'must', 'need', 'want', 'like', 'just', 'also',
    'find', 'get', 'give', 'make', 'take', 'use', 'know', 'see', 'come', 'think',
    'look', 'want', 'give', 'use', 'find', 'tell', 'ask', 'work', 'seem', 'feel',
    'calculate', 'compute', 'determine', 'obtain', 'retrieve', 'search', 'query',
    'please', 'help', 'show', 'display', 'list', 'return', 'provide', 'create',
}


# =============================================================================
# Perturbation Classes
# =============================================================================

class PerturbationGenerator:
    """Base class for perturbation generators"""

    def __init__(self, api_key: Optional[str] = None, model: str = "gpt-4o-mini"):
        self.api_key = api_key
        self.model = model
        self.client = None
        if api_key:
            self.client = OpenAI(api_key=api_key)

    def _call_llm(self, prompt: str, max_tokens: int = 1500) -> Optional[str]:
        """Call OpenAI API

        Args:
            prompt: The prompt to send to the LLM
            max_tokens: Maximum tokens for completion (default 1500, increased from 500
                       to handle long queries in paraphrase tasks)
        """
        if not self.client:
            raise ValueError("API client not initialized")

        try:
            response = self.client.chat.completions.create(
                model=self.model,
                messages=[
                    {"role": "system", "content": "You are a helpful assistant for generating high-quality data perturbations."},
                    {"role": "user", "content": prompt}
                ],
                max_completion_tokens=max_tokens,
                temperature=1.0,
            )
            content = response.choices[0].message.content
            return content.strip() if content else None
        except Exception as e:
            print(f"LLM API Error: {e}")
            return None


class RealisticTyposGenerator(PerturbationGenerator):
    """Generate realistic typos using LLM API for natural human-like typos"""

    PROMPT_TEMPLATE = """Add realistic typing errors to the following query, simulating natural human typos.

Original query: {query}

Requirements:
- Add 2-4 realistic typos that humans commonly make when typing quickly
- Include common typo types: adjacent key hits (e→r), character swaps (teh→the), missing letters, doubled letters, common misspellings (definitely→definately)
- DO NOT change any numbers, dates, proper nouns, or technical terms
- DO NOT change the meaning or intent of the query
- The query should still be understandable despite the typos
- Output ONLY the query with typos (no explanation)

Query with typos:"""

    def generate(self, sample: Dict, num_typos: int = 2) -> Optional[Dict]:
        """Add realistic typos to query using LLM"""
        query = self._extract_query(sample)
        if not query:
            return None

        prompt = self.PROMPT_TEMPLATE.format(query=query)
        typo_query = self._call_llm(prompt)

        if not typo_query:
            return None

        # Verify typo_query is different from original (has typos)
        if typo_query.strip() == query.strip():
            return None

        perturbed = self._update_query(sample, typo_query)
        perturbed['perturbation_type'] = 'realistic_typos'
        perturbed['perturbation_metadata'] = {
            'type': 'realistic_typos',
            'original_query': query,
            'generation_method': 'llm_api'
        }

        return perturbed

    def _extract_query(self, sample: Dict) -> Optional[str]:
        """Extract query from BFCL sample"""
        question = sample.get('question', [])
        if isinstance(question, list) and len(question) > 0:
            if isinstance(question[0], list) and len(question[0]) > 0:
                return question[0][0].get('content', '')
            elif isinstance(question[0], dict):
                return question[0].get('content', '')
        return None

    def _update_query(self, sample: Dict, new_query: str) -> Dict:
        """Update query in BFCL sample"""
        perturbed = deepcopy(sample)
        if isinstance(perturbed['question'][0], list):
            perturbed['question'][0][0]['content'] = new_query
        else:
            perturbed['question'][0]['content'] = new_query
        return perturbed


class QueryParaphraseGenerator(PerturbationGenerator):
    """Generate query paraphrases using LLM"""

    PROMPT_TEMPLATE = """Paraphrase the following user query while preserving its exact meaning and intent.

Original query: {query}

Requirements:
- Use different wording but keep the same semantic meaning
- DO NOT change any locations, person names, numbers, dates, or specific entities
- Maintain all technical terms and important details
- Output ONLY the paraphrased query (no explanation)

Paraphrased query:"""

    def generate(self, sample: Dict) -> Optional[Dict]:
        """Generate paraphrased query"""
        query = self._extract_query(sample)
        if not query:
            return None

        prompt = self.PROMPT_TEMPLATE.format(query=query)
        paraphrased = self._call_llm(prompt)

        if not paraphrased:
            return None

        perturbed = self._update_query(sample, paraphrased)
        perturbed['perturbation_type'] = 'query_paraphrase'
        perturbed['perturbation_metadata'] = {
            'type': 'query_paraphrase',
            'original_query': query
        }

        return perturbed

    def _extract_query(self, sample: Dict) -> Optional[str]:
        question = sample.get('question', [])
        if isinstance(question, list) and len(question) > 0:
            if isinstance(question[0], list) and len(question[0]) > 0:
                return question[0][0].get('content', '')
            elif isinstance(question[0], dict):
                return question[0].get('content', '')
        return None

    def _update_query(self, sample: Dict, new_query: str) -> Dict:
        perturbed = deepcopy(sample)
        if isinstance(perturbed['question'][0], list):
            perturbed['question'][0][0]['content'] = new_query
        else:
            perturbed['question'][0]['content'] = new_query
        return perturbed


class ParaphraseToolDescriptionGenerator(PerturbationGenerator):
    """Generate paraphrased tool descriptions using LLM"""

    PROMPT_TEMPLATE = """Paraphrase the following tool/function description while preserving its exact meaning.

Tool name: {tool_name}
Original description: {description}

Requirements:
- Use different wording but keep the same semantic meaning
- Maintain all technical details and constraints
- Keep similar length (±20%)
- Output ONLY the paraphrased description (no explanation)

Paraphrased description:"""

    def generate(self, sample: Dict) -> Optional[Dict]:
        """Generate sample with paraphrased tool descriptions"""
        perturbed = deepcopy(sample)
        functions = perturbed.get('function', [])

        # Handle multi-turn data: load tools from func_doc if no function field
        is_multi_turn = False
        if not functions:
            involved_classes = sample.get('involved_classes', [])
            if involved_classes:
                functions = load_multi_turn_tools(involved_classes)
                if functions:
                    perturbed['function'] = functions
                    is_multi_turn = True

        if not functions:
            return None

        original_descriptions = {}

        for func in functions:
            tool_name = func.get('name', '')
            original_desc = func.get('description', '')

            if not original_desc:
                continue

            original_descriptions[tool_name] = original_desc

            prompt = self.PROMPT_TEMPLATE.format(
                tool_name=tool_name,
                description=original_desc
            )

            paraphrased = self._call_llm(prompt)
            if paraphrased:
                func['description'] = paraphrased

        if not original_descriptions:
            return None

        perturbed['perturbation_type'] = 'paraphrase_tool_description'
        perturbed['perturbation_metadata'] = {
            'type': 'paraphrase_tool_description',
            'original_descriptions': original_descriptions,
            'is_multi_turn': is_multi_turn
        }

        return perturbed


class ParaphraseParameterDescriptionGenerator(PerturbationGenerator):
    """Generate paraphrased parameter descriptions using LLM"""

    PROMPT_TEMPLATE = """Paraphrase the following API parameter description while preserving its exact meaning.

Parameter name: {param_name}
Parameter type: {param_type}
Original description: {description}

Requirements:
- Use different wording but keep the same semantic meaning
- Maintain type constraints and valid values
- Keep similar length
- Output ONLY the paraphrased description (no explanation)

Paraphrased description:"""

    def generate(self, sample: Dict) -> Optional[Dict]:
        """Generate sample with paraphrased parameter descriptions"""
        perturbed = deepcopy(sample)
        functions = perturbed.get('function', [])

        # Handle multi-turn data: load tools from func_doc if no function field
        is_multi_turn = False
        if not functions:
            involved_classes = sample.get('involved_classes', [])
            if involved_classes:
                functions = load_multi_turn_tools(involved_classes)
                if functions:
                    perturbed['function'] = functions
                    is_multi_turn = True

        if not functions:
            return None

        original_descriptions = {}

        for func in functions:
            tool_name = func.get('name', '')
            params = func.get('parameters', {})
            properties = params.get('properties', {})

            if not properties:
                continue

            original_descriptions[tool_name] = {}

            for param_name, param_info in properties.items():
                original_desc = param_info.get('description', '')
                param_type = param_info.get('type', 'string')

                if not original_desc:
                    continue

                original_descriptions[tool_name][param_name] = original_desc

                prompt = self.PROMPT_TEMPLATE.format(
                    param_name=param_name,
                    param_type=param_type,
                    description=original_desc
                )

                paraphrased = self._call_llm(prompt)
                if paraphrased:
                    param_info['description'] = paraphrased

        if not any(original_descriptions.values()):
            return None

        perturbed['perturbation_type'] = 'paraphrase_parameter_description'
        perturbed['perturbation_metadata'] = {
            'type': 'paraphrase_parameter_description',
            'original_descriptions': original_descriptions,
            'is_multi_turn': is_multi_turn
        }

        return perturbed


# =============================================================================
# Main Processing Functions
# =============================================================================

def load_bfcl_data(category: str) -> List[Dict]:
    """Load BFCL V3 data for a category"""
    filepath = BFCL_DATA_DIR / f"BFCL_v3_{category}.json"

    if not filepath.exists():
        print(f"Warning: {filepath} not found")
        return []

    samples = []
    with open(filepath) as f:
        for line in f:
            line = line.strip()
            if line:
                try:
                    samples.append(json.loads(line))
                except json.JSONDecodeError:
                    continue

    return samples


def save_perturbations(samples: List[Dict], perturbation_type: str, category: str, mdp_category: str):
    """Save perturbed samples to appropriate directory"""
    output_dir = OUTPUT_DIR / mdp_category / perturbation_type
    output_dir.mkdir(parents=True, exist_ok=True)

    output_file = output_dir / f"BFCL_v3_{category}.jsonl"

    with open(output_file, 'w') as f:
        for sample in samples:
            f.write(json.dumps(sample) + '\n')

    print(f"  Saved {len(samples)} samples to {output_file}")


def generate_observation_perturbations(
    api_key: str,
    categories: List[str],
    perturbation_types: List[str],
    limit: Optional[int] = None
):
    """Generate Observation perturbations"""

    # Initialize generators
    generators = {}

    if 'realistic_typos' in perturbation_types:
        generators['realistic_typos'] = RealisticTyposGenerator(api_key=api_key, model="gpt-4o-mini")

    if 'query_paraphrase' in perturbation_types:
        generators['query_paraphrase'] = QueryParaphraseGenerator(api_key=api_key, model="gpt-4o-mini")

    if 'paraphrase_tool_description' in perturbation_types:
        generators['paraphrase_tool_description'] = ParaphraseToolDescriptionGenerator(api_key=api_key, model="gpt-4o-mini")

    if 'paraphrase_parameter_description' in perturbation_types:
        generators['paraphrase_parameter_description'] = ParaphraseParameterDescriptionGenerator(api_key=api_key, model="gpt-4o-mini")

    for category in categories:
        print(f"\nProcessing category: {category}")
        samples = load_bfcl_data(category)

        if limit:
            samples = samples[:limit]

        if not samples:
            print(f"  No samples found for {category}")
            continue

        for ptype, generator in generators.items():
            print(f"  Generating {ptype}...")
            perturbed_samples = []

            for sample in tqdm(samples, desc=f"    {ptype}"):
                try:
                    perturbed = generator.generate(sample)
                    if perturbed:
                        perturbed_samples.append(perturbed)
                except Exception as e:
                    print(f"    Error: {e}")
                    continue

            if perturbed_samples:
                save_perturbations(perturbed_samples, ptype, category, "Observation")


# =============================================================================
# Main Entry Point
# =============================================================================

def main():
    parser = argparse.ArgumentParser(description="Generate BFCL V3 perturbations")
    parser.add_argument('--test', action='store_true', help='Test mode (5 samples per category)')
    parser.add_argument('--full', action='store_true', help='Full generation')
    parser.add_argument('--type', type=str, help='Specific perturbation type')
    parser.add_argument('--category', type=str, help='Specific category')
    parser.add_argument('--api-key', type=str, default=os.getenv('OPENAI_API_KEY'), help='OpenAI API key')

    args = parser.parse_args()

    # Determine categories
    categories = ALL_CATEGORIES
    if args.category:
        categories = [args.category]

    # Determine perturbation types
    perturbation_types = ['realistic_typos', 'query_paraphrase',
                          'paraphrase_tool_description', 'paraphrase_parameter_description']
    if args.type:
        perturbation_types = [args.type]

    # API key only required for LLM-based perturbations
    needs_api = any(t in perturbation_types for t in ['query_paraphrase', 'paraphrase_tool_description', 'paraphrase_parameter_description'])
    if needs_api and not args.api_key:
        print("Error: OPENAI_API_KEY not set (required for LLM-based perturbations)")
        return

    # Determine limit
    limit = 5 if args.test else None

    print("=" * 60)
    print("BFCL V3 Perturbation Generator")
    print("=" * 60)
    print(f"Categories: {categories}")
    print(f"Perturbation types: {perturbation_types}")
    print(f"Limit: {limit if limit else 'None (full)'}")
    print("=" * 60)

    # Generate Observation perturbations
    generate_observation_perturbations(
        api_key=args.api_key,
        categories=categories,
        perturbation_types=perturbation_types,
        limit=limit
    )

    print("\n" + "=" * 60)
    print("Generation complete!")
    print("=" * 60)


if __name__ == "__main__":
    main()
