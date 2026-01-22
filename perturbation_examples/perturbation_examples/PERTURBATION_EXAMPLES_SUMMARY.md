# 扰动类型示例总结 (Perturbation Types Examples Summary)

本文档展示了针对工具调用任务设计的各种扰动类型示例，基于MDP (Markov Decision Process) 框架的分类。

## 原始Sample信息

**Sample ID**: `normal_atom_bool_0`

**用户查询**:
```
user: I'm looking for a list of high protein meals for dinner that include some vegetarian options.
system: Could you please specify your preferred cuisine type, such as Italian or Asian?
user: I would like Asian cuisine, please.
```

**Ground Truth (正确答案)**:
```json
{
  "name": "ProteinRichMealPlanner_generateList",
  "parameters": {
    "meal_type": "dinner",
    "include_vegetarian_options": true,
    "cuisine_preference": "Asian"
  }
}
```

---

## MDP框架扰动分类

我们的扰动设计基于强化学习的MDP框架，将扰动分为四大类：

1. **Observation (观察扰动)** - 影响输入层面
2. **Action (动作扰动)** - 影响工具选择层面
3. **Reward (奖励扰动)** - 影响元数据解读层面
4. **Transition (转移扰动)** - 影响执行反馈层面（运行时）

---

## 1. Observation 扰动 (输入层面)

### 1.1 Realistic Typos (真实打字错误)

**文件**: `perturbation_obs_typos_example.json`

**扰动内容**: 在用户查询中注入基于QWERTY键盘的真实打字错误

**示例改动**:
- `looking` → `lookign` (g和n相邻键)
- `list` → `lisr` (t和r相邻键)
- `high` → `hogh` (i和g相邻键)
- `vegetarian` → `vegetariqn` (a和q相邻键)
- `Asian` → `Asuab` (i和u相邻，n和b相邻)

**测试目标**: 模型是否能理解含有打字错误的输入

**期望行为**: 模型应该正确识别用户意图，忽略打字错误，选择正确的工具

---

### 1.2 Query Paraphrase (查询改写)

**文件**: `perturbation_obs_paraphrase_example.json`

**扰动内容**: 改写用户查询，保持语义不变但使用不同表达方式

**示例改动**:
```
原始: "I'm looking for a list of high protein meals"
改写: "Could you help me compile a selection of protein-heavy dinner dishes"

原始: "include some vegetarian options"
改写: "feature plant-based alternatives"

原始: "I would like Asian cuisine, please"
改写: "Asian-style food would be great"
```

**测试目标**: 模型是否能理解语义等价但表达不同的查询

**期望行为**: 模型应该识别改写后的查询语义，正确选择工具

---

### 1.3 Tool Description Paraphrase (工具描述改写)

**文件**: `perturbation_obs_tool_desc_example.json`

**扰动内容**: 改写目标工具的描述，保持语义不变但使用不同表达

**示例改动**:
```
原始描述: "Create a list of meals that are high in protein and low in fat."
改写描述: "Generate meal options with elevated protein content while maintaining minimal fat levels."
```

**测试目标**: 模型是否能理解语义等价但表达不同的工具描述

**期望行为**: 模型应该识别改写后的描述语义，正确选择工具

---

### 1.4 Parameter Description Paraphrase (参数描述改写)

**文件**: `perturbation_obs_param_desc_example.json`

**扰动内容**: 改写工具参数的描述，保持语义不变但使用不同表达

**示例改动**:
```
参数: meal_type
原始: "Type of meal to focus on, e.g., breakfast, lunch, dinner."
改写: "Specifies the meal category to generate recommendations for, such as morning meal, midday meal, or evening meal."

参数: include_vegetarian_options
原始: "Whether to include vegetarian meal options."
改写: "Indicates if plant-based dietary choices should be incorporated into the meal suggestions."

参数: cuisine_preference
原始: "Preferred cuisine type, e.g., Italian, Asian."
改写: "The culinary tradition or regional cooking style desired, for example Mediterranean, Eastern, or Western."
```

**测试目标**: 模型是否能理解改写后的参数描述并正确填充参数

**期望行为**: 模型应该识别改写后的参数描述语义，正确填充参数值

---

## 2. Action 扰动 (工具选择层面)

### 2.1 Same Name Tools (同名工具干扰)

**文件**: `perturbation_action_same_name_example.json`

**扰动内容**: 添加一个与正确工具同名的干扰工具（空壳版本）

**示例改动**:
```json
// 干扰工具（空壳，无描述无参数）
{
  "name": "ProteinRichMealPlanner_generateList",
  "description": "",
  "parameters": {
    "type": "object",
    "properties": {}
  }
}

// 正确工具（完整描述和参数）
{
  "name": "ProteinRichMealPlanner_generateList",
  "description": "Create a list of meals that are high in protein and low in fat.",
  "parameters": {
    "type": "object",
    "properties": {
      "meal_type": {...},
      "include_vegetarian_options": {...},
      "cuisine_preference": {...}
    }
  }
}
```

**测试目标**: 模型是否能区分同名但内容不同的工具

**期望行为**: 模型应该选择有完整描述和参数的正确工具，而非空壳干扰工具

**变体说明**:
- **Type A**: 同名 + 空壳（无描述无参数）
- **Type B**: 同名 + GT描述 + 无参数
- **Type C**: 同名 + 无描述 + 错误参数
- **Type D**: 同名 + GT描述 + 错误参数
- **Type E**: 同名 + 其他描述 + 错误参数

---

### 2.2 Redundant Similar Tools (冗余相似工具)

**文件**: `perturbation_action_redundant_example.json`

**扰动内容**: 添加2-3个功能相似但不完全相同的干扰工具

**示例改动**:
```json
// 干扰工具1: 侧重营养和卡路里（功能相似但不精确）
{
  "name": "NutritionPlanner_getMeals",
  "description": "Provides nutritional meal recommendations based on dietary requirements and calorie goals.",
  "parameters": {
    "calorie_target": {...},
    "dietary_restrictions": {...},
    "meal_count": {...}
  }
}

// 干扰工具2: 生成完整meal plan（功能相似但更复杂）
{
  "name": "DietaryPlanner_createMealPlan",
  "description": "Creates comprehensive meal plans with macronutrient breakdown and shopping lists.",
  "parameters": {
    "duration_days": {...},
    "diet_type": {...},
    "include_snacks": {...}
  }
}

// 正确工具: 精确匹配需求（高蛋白、特定餐型、素食选项、菜系）
{
  "name": "ProteinRichMealPlanner_generateList",
  "description": "Create a list of meals that are high in protein and low in fat.",
  "parameters": {
    "meal_type": "dinner",
    "include_vegetarian_options": true,
    "cuisine_preference": "Asian"
  }
}
```

**测试目标**: 模型是否能从多个相似工具中选择最精确匹配用户需求的工具

**期望行为**: 模型应该选择最精确匹配需求的工具，而非功能相似但不完全匹配的工具

---

## 3. Reward 扰动 (元数据解读层面)

### 3.1 Cost in Description (成本在描述中)

**文件**: `perturbation_reward_CD_example.json`

**扰动内容**: 在工具描述中添加成本信息，并使用误导性后缀命名干扰工具

**示例改动**:
```json
// 正确工具: 实际成本低 ($0.01)
{
  "name": "ProteinRichMealPlanner_generateList",
  "description": "Create a list of meals that are high in protein and low in fat. [Cost: $0.01 per call]",
  "parameters": {...}
}

// 干扰工具: 名字暗示便宜(Budget)，但实际成本高 ($0.15)
{
  "name": "ProteinRichMealPlanner_generateList_Budget",
  "description": "Create a list of meals that are high in protein and low in fat. [Cost: $0.15 per call]",
  "parameters": {}
}
```

**测试目标**: 模型是否会被误导性的工具名称欺骗，还是能读取实际的成本信息

**期望行为**: 模型应该读取描述中的实际成本，选择成本更低的工具，而非被"_Budget"后缀误导

**误导性后缀示例**:
- `_Budget` / `_Economy` (暗示便宜，实际更贵)
- `_Premium` / `_Enterprise` (暗示昂贵，实际便宜)
- `_Free` / `_Discount` / `_Cheap` (暗示免费/便宜，实际更贵)

---

### 3.2 Time in Description (时间在描述中)

**文件**: `perturbation_reward_TD_example.json`

**扰动内容**: 在工具描述中添加响应时间信息，并使用误导性后缀命名干扰工具

**示例改动**:
```json
// 正确工具: 实际响应快 (~50ms)
{
  "name": "ProteinRichMealPlanner_generateList",
  "description": "Create a list of meals that are high in protein and low in fat. [Response time: ~50ms]",
  "parameters": {...}
}

// 干扰工具: 名字暗示快速(Fast)，但实际响应慢 (~2100ms)
{
  "name": "ProteinRichMealPlanner_generateList_Fast",
  "description": "Create a list of meals that are high in protein and low in fat. [Response time: ~2100ms]",
  "parameters": {}
}
```

**测试目标**: 模型是否会被误导性的工具名称欺骗，还是能读取实际的响应时间信息

**期望行为**: 模型应该读取描述中的实际响应时间，选择更快的工具，而非被"_Fast"后缀误导

**误导性后缀示例**:
- `_Fast` / `_Quick` / `_Express` (暗示快速，实际更慢)
- `_Slow` / `_Delayed` (暗示慢速，实际更快)
- `_Instant` / `_Rapid` / `_Swift` (暗示即时，实际更慢)

---

## 4. Transition 扰动 (运行时反馈层面)

### 4.1 Transient Timeout (瞬态超时)

**说明**: Transition扰动是运行时注入的，不体现在静态数据中，因此没有单独的JSON示例文件。

**扰动机制**:
1. 模型第一次调用某个工具时，系统返回超时错误（不实际执行）
2. 模型需要识别这是瞬态错误，并进行重试
3. 模型第二次调用相同工具时，系统正常执行并返回结果

**错误消息示例**:
```
ERROR: Timeout occurred while executing the tool call. The operation took too long to respond. Please retry your request.
```

**测试目标**: 模型是否能识别瞬态错误并进行重试

**期望行为**: 模型应该：
1. 识别超时错误是临时性的
2. 重新调用相同的工具（而非放弃或切换工具）
3. 在第二次调用时获得正确结果

**实现方式**:
- 在评估代码中包装工具执行函数
- 第一次调用拦截并返回错误
- 后续调用正常执行

---

## 扰动效果总结

| MDP类别 | 扰动类型 | 难度等级 | 测试能力 |
|---------|---------|---------|---------|
| **Observation** | Realistic Typos | Medium | 输入鲁棒性、拼写容错 |
| **Observation** | Query Paraphrase | Easy | 语义理解、表达多样性 |
| **Observation** | Tool Desc Paraphrase | Easy | 工具理解、描述多样性 |
| **Observation** | Param Desc Paraphrase | Medium | 参数理解、映射能力 |
| **Action** | Same Name Tools | Hard | 工具区分、细节判断 |
| **Action** | Redundant Similar Tools | Medium | 精确匹配、相似度判断 |
| **Reward** | Cost in Description (CD) | Hard | 元数据解读、抗误导能力 |
| **Reward** | Time in Description (TD) | Hard | 元数据解读、抗误导能力 |
| **Transition** | Transient Timeout | Hard | 错误处理、重试策略 |

---

## 使用说明

1. **单个扰动测试**: 使用对应的JSON文件作为测试输入
2. **组合扰动测试**: 可以同时应用多个扰动类型（例如：Typos + Same Name）
3. **评估指标**: 模型是否选择正确的工具及参数
4. **分析维度**:
   - 准确率下降幅度（相比clean baseline）
   - 各扰动类型的难度排序
   - 模型对不同扰动类型的敏感度

---

## 文件清单

1. `perturbation_obs_typos_example.json` - 打字错误示例
2. `perturbation_obs_paraphrase_example.json` - 查询改写示例
3. `perturbation_obs_tool_desc_example.json` - 工具描述改写示例
4. `perturbation_obs_param_desc_example.json` - 参数描述改写示例
5. `perturbation_action_same_name_example.json` - 同名工具干扰示例
6. `perturbation_action_redundant_example.json` - 冗余相似工具示例
7. `perturbation_reward_CD_example.json` - 成本描述扰动示例
8. `perturbation_reward_TD_example.json` - 时间描述扰动示例
9. `PERTURBATION_EXAMPLES_SUMMARY.md` - 本总结文档

---

## 联系信息

如有疑问，请联系项目负责人。

**生成日期**: 2026-01-20
