



def create_vllm_backend(model_name: str, num_gpus:int):
    print("importing vllm and transformers...")
    from vllm import AsyncLLMEngine
    from vllm.engine.arg_utils import AsyncEngineArgs
    from transformers import AutoTokenizer
    print("vllm and transformers imported.")
    print("Creating vLLM backend...")
    # Create engine args
    engine_args = AsyncEngineArgs(
        model=model_name,
        tensor_parallel_size=num_gpus,
        gpu_memory_utilization=0.9,
        trust_remote_code=True,
        enable_lora=False,
        max_model_len=15000,
    )
    engine = AsyncLLMEngine.from_engine_args(engine_args)

    tokenizer = AutoTokenizer.from_pretrained(model_name, trust_remote_code=True)
    print("vLLM backend created.")
    return engine, tokenizer


def should_disable_thinking(model_name: str) -> bool:
    name = model_name.lower()
    return ("looptool" in name) or ("qwen3" in name) or ("mua-rl" in name)


async def call_vllm_model_async(
    model_name: str,
    engine,
    tokenizer,
    system_prompt: str,
    user_prompt: str,
) -> str:
    from vllm import SamplingParams
    try:
        messages = [
            {"role": "system", "content": system_prompt},
            {"role": "user", "content": user_prompt},
        ]


        disable_thinking = should_disable_thinking(model_name)

        if disable_thinking:
            formatted_prompt = tokenizer.apply_chat_template(
                messages,
                add_generation_prompt=True,
                tokenize=False,
                enable_thinking=False,
            )
        else:
            formatted_prompt = tokenizer.apply_chat_template(
                messages,
                add_generation_prompt=True,
                tokenize=False,
            )
        # Use vLLM to generate the response
        from vllm.sampling_params import SamplingParams
        stop_token_ids = [tokenizer.eos_token_id]

        eot_id = tokenizer.convert_tokens_to_ids("<|eot_id|>")
        if eot_id is not None and eot_id != tokenizer.unk_token_id:
            stop_token_ids.append(eot_id)
        sampling_params = SamplingParams(
            temperature=0.0,  # Greedy decoding for tool calls
            max_tokens=2048,
            stop_token_ids=stop_token_ids,
        )
        import uuid
        # Generate with vLLM engine
        request_id = f"tool_call_{uuid.uuid4()}"
        results_generator = engine.generate(
            formatted_prompt,
            sampling_params,
            request_id
        )

        # Wait for completion
        final_output = None
        async for request_output in results_generator:
            final_output = request_output

        if final_output is None:
            raise RuntimeError("vLLM generation returned no output")

        # Extract the generated text
        generated_text = final_output.outputs[0].text.strip()
    except Exception as e:
        print(f"Warning during generation: {e}")
        generated_text = "finish conversation"
    return generated_text
