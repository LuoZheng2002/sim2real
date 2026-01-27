from transformers import AutoTokenizer
from vllm import LLM

model_path = "Qwen/Qwen3-8B"
tokenizer = AutoTokenizer.from_pretrained(model_path, trust_remote_code=True)
tokenizer.pad_token_id = tokenizer.eos_token_id
llm = LLM(model=model_path, dtype="float16", trust_remote_code=True)

tool_schemas = [
    {
        "name": "filesystem_read_file",
        "description": "读取指定路径的文件内容。",
        "parameters": {
            "type": "object",
            "properties": {
                "file_path": {"type": "string", "description": "文件路径"},
                "max_size": {"type": "integer", "default": 102400}
            },
            "required": ["file_path"]
        }
    },
    # 更多工具...
]

messages = [{
    "role": "system",
    "content": "你是一个文件系统助手。"
}, {
    "role": "user",
    "content": input_data.user_query
}]

prompt = tokenizer.apply_chat_template(
    messages,
    tools=tool_schemas, # 带工具 schema 的 提示词
    add_generation_prompt=True,
    tokenize=False,
    enable_thinking=False,
    output_tool_calls=True
)

outputs = llm.generate(prompt, SamplingParams(temperature=0.0, max_tokens=512, stop=["<|im_end|>"]))
generated_text = outputs[0].outputs[0].text
tool_calls = extract_tool_calls(generated_text)

if not tool_calls:
    return {"response": clean_output_strict(generated_text)}

tool_messages = []
for call in tool_calls:
    func = tool_map.get(call["name"])
    response = func(**call.get("arguments", {}))
    tool_messages.append({"role": "tool", "name": call["name"], "content": format_function_response(call["name"], response)})

messages += [{"role": "assistant", "tool_calls": tool_calls}] + tool_messages

followup_prompt = tokenizer.apply_chat_template(
    messages,
    tools=[],
    add_generation_prompt=True,
    tokenize=False
)

final_outputs = llm.generate(followup_prompt, SamplingParams(temperature=0.0, max_tokens=512, stop=["<|im_end|>"]))
final_text = final_outputs[0].outputs[0].text

return {"response": clean_output_strict(final_text)}