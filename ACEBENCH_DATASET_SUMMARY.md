# ACEBench Dataset Analysis and Testing Logic Summary

## Overview

ACEBench is a comprehensive benchmark for evaluating Large Language Models' (LLMs) tool usage capabilities. It addresses three key limitations in existing benchmarks:
1. Limited evaluation scenarios (lack of real multi-turn dialogue)
2. Narrow evaluation dimensions (insufficient detailed assessments)
3. High evaluation costs (reliance on LLMs or real API executions)

The benchmark contains **2,000 annotated entries** in both Chinese and English, covering **4,538 APIs** across **8 major domains** and **68 sub-domains**.

---

## Dataset Categories

ACEBench is divided into three main categories: **Normal**, **Special**, and **Agent**.

### 1. NORMAL DATA (Fixed Question-Answer Pairs)

Normal data consists of fixed question-answer pairs with deterministic ground truth for validation.

#### 1.1 Single-Turn Data
**Files**:
- `data_normal_single_turn_single_function.json`
- `data_normal_single_turn_parallel_function.json`

**Format**:
```json
{
  "id": "normal_single_turn_single_function_0",
  "question": "user: [User query requiring tool usage]",
  "function": [
    {
      "name": "FunctionName",
      "description": "Function description",
      "parameters": {
        "type": "object",
        "properties": {...},
        "required": [...]
      }
    }
  ],
  "answer": ["FunctionName(param1='value1', param2='value2')"]
}
```

**Relationship with Ground Truth**:
- Single function call: Exact match of function name and all parameters
- Parallel function calls: Multiple function calls must all match (order-independent)

**Evaluation**: AST-based parsing comparing model output against ground truth
- Function name must match exactly
- Parameter types must match
- Parameter values must match exactly
- Output format must follow `[FunctionName(key='value')]` syntax

---

#### 1.2 Multi-Turn Data
**Files**:
- `data_normal_multi_turn_user_switch.json` (topic switching)
- `data_normal_multi_turn_user_adjust.json` (query refinement)

**Format**:
```json
{
  "id": "normal_multi_turn_0",
  "conversation": [
    {"role": "user", "content": "Initial query"},
    {"role": "system", "content": "Response/clarification"},
    {"role": "tool", "content": "[FunctionCall(...)]"},
    {"role": "user", "content": "Follow-up query"}
  ],
  "function": [...],
  "answer": ["FinalFunctionCall(...)"]
}
```

**Relationship with Ground Truth**:
- Evaluates only the final function call in the conversation chain
- Must correctly use context from all previous turns
- For "switch" type: Tests ability to handle topic changes
- For "adjust" type: Tests ability to refine/modify previous requests

**Evaluation**: Same AST-based matching as single-turn, but evaluated on final turn only

---

#### 1.3 Preference Data
**File**: `data_normal_preference.json`

**Format**:
```json
{
  "id": "normal_preference_0",
  "question": "user: [Query requiring personalized information]",
  "function": [...],
  "profile": {
    "basic_features": {
      "UserName": "...",
      "UserEmail": "...",
      "UserHomeLocation": "..."
    },
    "user_history": {
      "shopping": [...],
      "takeout": [...]
    }
  },
  "answer": ["FunctionCall(param='value_from_profile')"]
}
```

**Relationship with Ground Truth**:
- Model must extract parameter values from user profile data
- Tests ability to mine user-specific factors (past interactions, preferences)
- Ground truth includes values that should be inferred from profile, not explicitly stated in query

**Evaluation**: Parameter values must match those in the profile data

---

#### 1.4 Similar API Data
**File**: `data_normal_similar_api.json`

**Format**:
```json
{
  "id": "normal_similar_api_0",
  "question": "user: [Ambiguous query that could match multiple APIs]",
  "function": [
    {"name": "SimilarFunction_A", "description": "..."},
    {"name": "SimilarFunction_B", "description": "..."}
  ],
  "answer": ["CorrectFunction(...)"]
}
```

**Relationship with Ground Truth**:
- Multiple candidate APIs with similar functionality
- Model must distinguish subtle differences and select correct API
- Tests fine-grained function selection capability

**Evaluation**: Must select the correct function among similar alternatives

---

#### 1.5 Atom Data
**Files** (by parameter type):
- `data_normal_atom_bool.json`
- `data_normal_atom_enum.json`
- `data_normal_atom_list.json`
- `data_normal_atom_number.json`
- `data_normal_atom_object_short.json`
- `data_normal_atom_object_deep.json`

**Format**:
```json
{
  "id": "normal_atom_bool_0",
  "question": "user: [Query]",
  "function": [{
    "name": "Function",
    "parameters": {
      "properties": {
        "param": {"type": "boolean|string|number|array|object"}
      }
    }
  }],
  "answer": ["Function(param=specific_type_value)"]
}
```

**Relationship with Ground Truth**:
- Tests atomic-level capability for specific parameter types
- Each file focuses on a single parameter type to isolate capability
- Ground truth enforces correct type handling

**Evaluation**:
- Type correctness (boolean vs string vs number, etc.)
- Value correctness within type constraints
- Format correctness for complex types (lists, objects)

---

### 2. SPECIAL DATA (Imperfect Instructions)

Special data tests model robustness with incomplete or incorrect instructions.

#### 2.1 Incomplete Data
**File**: `data_special_incomplete.json`

**Format**:
```json
{
  "id": "special_incomplete_0",
  "question": "user: [Query missing required parameters]",
  "function": [{
    "name": "Function",
    "parameters": {
      "required": ["param1", "param2", "param3"]
    }
  }],
  "answer": "Missing necessary parameters (param2) for the API (Function)"
}
```

**Relationship with Ground Truth**:
- Query intentionally lacks required parameter information
- Ground truth specifies which parameters are missing
- Model should detect and report missing parameters, NOT fabricate values

**Evaluation**:
- Accuracy = 1 if model correctly identifies missing parameters
- Accuracy = 0 if model attempts to call function or fails to detect issue

---

#### 2.2 Error Parameter Data
**File**: `data_special_error_param.json`

**Format**:
```json
{
  "id": "special_error_param_0",
  "question": "user: [Query with invalid parameter format/value]",
  "function": [{
    "parameters": {
      "properties": {
        "param": {
          "type": "string",
          "pattern": "^[a-zA-Z]+$",
          "enum": ["option1", "option2"]
        }
      }
    }
  }],
  "answer": "There is incorrect value (invalid_value) for the (param)"
}
```

**Relationship with Ground Truth**:
- Query contains parameters that violate format constraints or enum restrictions
- Ground truth identifies which parameter values are incorrect
- Model should detect constraint violations

**Evaluation**:
- Must identify that parameter value doesn't meet specifications
- Must specify which parameter is incorrect

---

#### 2.3 Irrelevant Data
**File**: `data_special_irrelevant.json`

**Format**:
```json
{
  "id": "special_irrelevant_0",
  "question": "user: [Query that cannot be solved with provided functions]",
  "function": [
    {"name": "UnrelatedFunction1", "..."},
    {"name": "UnrelatedFunction2", "..."}
  ],
  "answer": "Due to the limitations of the function, I cannot solve this problem."
}
```

**Relationship with Ground Truth**:
- No candidate function can address the user's request
- Ground truth indicates task impossibility
- Model should recognize capability limitations

**Evaluation**:
- Must refuse to call any function
- Must explain that capabilities are insufficient

---

### 3. AGENT DATA (Interactive Multi-Turn Scenarios)

Agent data simulates real-world multi-turn interactions in sandbox environments.

#### 3.1 Agent Multi-Turn Data
**File**: `data_agent_multi_turn.json`

**Format**:
```json
{
  "id": "agent_multi_turn_0",
  "question": "[Complex task description with rules]",
  "initial_config": {
    "Class": {
      "attribute1": value,
      "attribute2": value
    }
  },
  "path": [
    "function1(param='value')",
    "function2(param='value')",
    ...
  ],
  "function": [...],
  "target": {
    "Class": {
      "attribute1": final_value,
      "attribute2": final_value
    }
  }
}
```

**Relationship with Ground Truth**:
- **Initial Config**: Starting state of the sandbox environment
- **Path**: Ideal sequence of function calls to complete the task
- **Target**: Expected final state of the environment
- User simulator (GPT-4o) interacts dynamically based on model responses

**Evaluation - Two Metrics**:

1. **Process Accuracy** = n/m where:
   - m = length of ideal path (ground truth sequence)
   - n = number of matching function calls in correct order
   - Tests whether model follows correct procedure

2. **End-to-End Accuracy** = 1 or 0:
   - Compare final environment state with target state
   - All attributes must match exactly
   - Tests whether task objective is achieved

---

#### 3.2 Agent Multi-Step Data
**File**: `data_agent_multi_step.json`

**Format**: Similar to multi-turn, but user participates only once.

**Difference from Multi-Turn**:
- Multi-step: User gives initial instruction, model completes entire workflow
- Multi-turn: User interacts multiple times throughout the dialogue

**Evaluation**: Same Process Accuracy and End-to-End Accuracy metrics

---

## Dataset Statistics

### Domain Distribution
- **8 Major Domains**: Technology, Finance, Entertainment, Society, Health, Culture, Environment, Others
- **68 Sub-domains**: Detailed coverage of daily life scenarios
- **4,538 Total APIs**: In both Chinese and English

### Data Composition
- **Normal Data**: ~1,400 samples
  - Atom: 300 samples
  - Single-Turn: 1,000+ samples
  - Multi-Turn: 400+ samples
  - Similar API: 100 samples
  - Preference: 300 samples

- **Special Data**: ~300 samples
  - Incomplete: 100 samples
  - Error: 100 samples
  - Irrelevant: 100 samples

- **Agent Data**: ~300 samples
  - Multi-Turn: 150 samples
  - Multi-Step: 150 samples

### Complexity Distribution
- **Dialogue Turns**: 0-8 turns
- **API Arguments**: 0-7 arguments
- Most common: 1-2 turns with 1-3 arguments

---

## Evaluation Logic & Code Testing Strategy

### 1. Normal Data Testing Logic

```python
def evaluate_normal_data(model_output, ground_truth):
    """
    AST-based function call validation
    """
    # Parse model output and ground truth into AST
    model_ast = parse_function_call(model_output)
    truth_ast = parse_function_call(ground_truth)

    # Check 1: Function name match
    if model_ast.function_name != truth_ast.function_name:
        return {"accuracy": 0, "error_type": "function_name"}

    # Check 2: Parameter count match
    if len(model_ast.params) != len(truth_ast.params):
        return {"accuracy": 0, "error_type": "param_num"}

    # Check 3: Parameter types match
    for param in truth_ast.params:
        if type(model_ast.params[param]) != type(truth_ast.params[param]):
            return {"accuracy": 0, "error_type": "param_type"}

    # Check 4: Parameter values match
    for param, value in truth_ast.params.items():
        if model_ast.params[param] != value:
            return {"accuracy": 0, "error_type": "param_value"}

    # Check 5: Output format validation
    if not matches_format_pattern(model_output):
        return {"accuracy": 0, "error_type": "output_format"}

    return {"accuracy": 1, "error_type": None}
```

### 2. Special Data Testing Logic

```python
def evaluate_special_data(model_output, ground_truth, special_type):
    """
    Problem detection validation
    """
    if special_type == "incomplete":
        # Model should identify missing required parameters
        expected_pattern = "Missing necessary parameters"
        if expected_pattern in model_output:
            # Check if correct parameters identified
            missing_params = extract_missing_params(ground_truth)
            identified_params = extract_missing_params(model_output)
            if set(missing_params) == set(identified_params):
                return {"accuracy": 1}
        return {"accuracy": 0, "error": "detection_failed"}

    elif special_type == "error":
        # Model should identify incorrect parameter values
        expected_pattern = "incorrect value"
        if expected_pattern in model_output:
            error_param = extract_error_param(ground_truth)
            identified_param = extract_error_param(model_output)
            if error_param == identified_param:
                return {"accuracy": 1}
        return {"accuracy": 0, "error": "correction_failed"}

    elif special_type == "irrelevant":
        # Model should refuse to call any function
        refusal_pattern = "cannot solve this problem|limitations of the function"
        if re.search(refusal_pattern, model_output):
            return {"accuracy": 1}
        return {"accuracy": 0, "error": "false_positive"}
```

### 3. Agent Data Testing Logic

```python
def evaluate_agent_data(dialogue_history, ground_truth):
    """
    Sandbox environment state validation
    """
    # Initialize sandbox with initial_config
    sandbox = create_sandbox(ground_truth["initial_config"])

    # Extract function calls from dialogue
    model_function_calls = extract_function_calls(dialogue_history)
    ideal_function_calls = ground_truth["path"]

    # Execute all function calls in sandbox
    for func_call in model_function_calls:
        sandbox.execute(func_call)

    # Metric 1: Process Accuracy
    process_acc = calculate_sequence_match(
        model_function_calls,
        ideal_function_calls
    )

    # Metric 2: End-to-End Accuracy
    final_state = sandbox.get_state()
    target_state = ground_truth["target"]

    end_to_end_acc = 1 if states_match(final_state, target_state) else 0

    return {
        "process_accuracy": process_acc,
        "end_to_end_accuracy": end_to_end_acc
    }

def calculate_sequence_match(model_calls, ideal_calls):
    """
    Calculate n/m where n is matching calls, m is ideal sequence length
    """
    m = len(ideal_calls)
    n = 0

    for i, ideal_call in enumerate(ideal_calls):
        if i < len(model_calls) and calls_match(model_calls[i], ideal_call):
            n += 1
        else:
            break  # Stop at first mismatch

    return n / m
```

### 4. Overall Testing Pipeline

```python
class ACEBenchEvaluator:
    def __init__(self, dataset_path):
        self.load_datasets(dataset_path)

    def evaluate_model(self, model):
        results = {
            "normal": self.eval_normal(model),
            "special": self.eval_special(model),
            "agent": self.eval_agent(model)
        }

        # Calculate weighted overall accuracy
        overall = self.calculate_overall_accuracy(results)
        return overall

    def calculate_overall_accuracy(self, results):
        """
        Overall Accuracy = A·AccNormal + B·AccSpecial + C·AccAgent

        Where weights are:
        A = sqrt(n_normal) / (sqrt(n_normal) + sqrt(n_special) + sqrt(n_agent))
        B = sqrt(n_special) / (sqrt(n_normal) + sqrt(n_special) + sqrt(n_agent))
        C = sqrt(n_agent) / (sqrt(n_normal) + sqrt(n_special) + sqrt(n_agent))
        """
        import math

        n_normal = results["normal"]["count"]
        n_special = results["special"]["count"]
        n_agent = results["agent"]["count"]

        total_sqrt = (math.sqrt(n_normal) +
                      math.sqrt(n_special) +
                      math.sqrt(n_agent))

        A = math.sqrt(n_normal) / total_sqrt
        B = math.sqrt(n_special) / total_sqrt
        C = math.sqrt(n_agent) / total_sqrt

        overall_acc = (A * results["normal"]["accuracy"] +
                       B * results["special"]["accuracy"] +
                       C * results["agent"]["accuracy"])

        return overall_acc
```

---

## Key Testing Considerations

### 1. Error Type Analysis (Normal Data)
The evaluation tracks 5 main error types:
- **Function Name Error**: Wrong API selected
- **Parameter Number Error**: Missing or extra parameters
- **Parameter Type Error**: Wrong data type (string vs int)
- **Parameter Value Error**: Correct type but wrong value (most common ~60%)
- **Output Format Error**: Doesn't follow `[FuncName(key='value')]` syntax

### 2. Special Data Focus
- **Error Detection**: Can model identify problems?
- **Error Correction**: Can model specify what's wrong?
- Most errors are "detection failures" (model doesn't recognize issues)

### 3. Agent Data Challenges
- **Rule Violations**: Model ignores predefined scenario rules
- **Function Call Errors**: Wrong function or invalid parameters
- **Information Mismanagement**: Loses context across turns
- Most models achieve <50% end-to-end accuracy on agent tasks

### 4. Prompt Engineering Impact
- **Standard Prompt**: Comprehensive instructions (best performance)
- **Condensed Prompt**: Core instructions only (-2% accuracy)
- **Minimal Prompt**: Keywords only (-8% accuracy)

---

## Summary

ACEBench provides a comprehensive evaluation framework with:

1. **Normal Data**: Tests correct tool usage under clear instructions
   - Ground truth: Exact function call with all parameters
   - Evaluation: AST-based exact matching

2. **Special Data**: Tests robustness with imperfect instructions
   - Ground truth: Problem identification messages
   - Evaluation: Detection and correction capability

3. **Agent Data**: Tests real-world multi-turn interactions
   - Ground truth: Ideal process path + target environment state
   - Evaluation: Process accuracy (sequence matching) + End-to-end accuracy (state matching)

The benchmark is **LLM-free** (no LLM judges), **cost-effective** (deterministic evaluation), and **comprehensive** (covers diverse scenarios from atomic capabilities to complex multi-step tasks).