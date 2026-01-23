



def create_vllm_backend(model_name: str):
    print("importing vllm and transformers...")
    from vllm import LLMEngine
    from transformers import AutoTokenizer
    print("vllm and transformers imported.")
    print("Creating vLLM backend...")
    engine = LLMEngine.from_pretrained(
        model_name,
        tensor_parallel_size=1,
        max_batch_size=16,
        max_input_length=2048,
        max_output_length=1024,
        temperature=0.7,
        top_p=0.9,
        repetition_penalty=1.0,
        device="cuda",
    )

    tokenizer = AutoTokenizer.from_pretrained(model_name)
    print("vLLM backend created.")
    return engine, tokenizer

async def call_vllm_model_async(
    engine,
    tokenizer,
    system_prompt: str,
    user_prompt: str,
) -> str:
    from vllm import SamplingParams, Request, Batch
    import asyncio

    prompt = f"{system_prompt}\n{user_prompt}"
    inputs = tokenizer(prompt, return_tensors="pt")
    input_ids = inputs["input_ids"][0].tolist()

    request = Request(
        input_ids=input_ids,
        sampling_params=SamplingParams(
            max_tokens=1000,
            temperature=0.7,
            top_p=0.9,
        ),
    )

    batch = Batch(requests=[request])
    outputs = await engine.generate_async(batch)

    response_ids = outputs[0].sequences[0][len(input_ids):]
    response = tokenizer.decode(response_ids, skip_special_tokens=True)
    return response