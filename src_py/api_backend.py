from typing import Any
import os

import re

def create_api_backend(
    model_name: str,
) -> Any:
    try:
        from openai import AsyncOpenAI
        # import httpx
    except ImportError:
        raise ImportError(
            "API backend requires the openai library. "
            "Install with: pip install openai"
        )
    # api_key_name = api_model.api_key_name()
    if "gpt" in model_name.lower():
        api_key_name = "OPENAI_API_KEY"
    elif "deepseek" in model_name.lower():
        api_key_name = "DEEPSEEK_API_KEY"
    else:
        raise ValueError(f"Unsupported model name: {model_name}")
    
    # base_url = api_model.base_url()
    if "gpt" in model_name.lower():
        base_url = "https://api.openai.com/v1"
    elif "deepseek" in model_name.lower():
        base_url = "https://api.deepseek.com"
    else:
        raise ValueError(f"Unsupported model name: {model_name}")
    api_key = os.getenv(api_key_name)
    if not api_key:
        raise ValueError(f"API key for model {model_name} not found. Please set the environment variable '{api_key_name}'.")

    client = AsyncOpenAI(
        api_key=api_key,
        base_url=base_url,
    )
    print(f"Created API backend for model {model_name}")
    return client

async def call_api_model_async(
    client: Any,
    model_name: str,
    system_prompt: str,
    user_prompt: str,
) -> str:
    message = [
            {
                "role": "system",
                "content": system_prompt,
            },
            {
                "role": "user",
                "content": user_prompt,
            },
        ]
    try:
        response = await client.chat.completions.create(
            messages=message,
            model=model_name,
            temperature=0.001,
            max_tokens=1000,
            top_p=1.0,
        )
        result = response.choices[0].message.content

        if "deepseek-r1" in model_name:
            match = re.search(r'</think>\s*(.*)$', result, re.DOTALL)
            result = match.group(1).strip()
    except Exception as e:
        # # Check if it's a specific error type, skip current iteration
        # if 'data_inspection_failed' in str(e):
        #     print(id)
        #     continue  # Skip current iteration, continue to next attempt
        # elif attempt == 6:
        #     raise e  # If maximum attempts reached, raise exception
        print(f"Error calling model {model_name}: {e}")
        exit(1)
    return result