


from dotenv import load_dotenv
import argparse
import subprocess
import time
import asyncio
import json

from src_py.api_backend import call_api_model_async, create_api_backend
from src_py.vllm_backend import call_vllm_model_async, create_vllm_backend
load_dotenv(".env")

api_backend_created = False
client=None
vllm_backend_created = False
engine = None
tokenizer = None



# Parse command-line arguments
parser = argparse.ArgumentParser(
    description="Run BFCL evaluation with custom configuration"
)
parser.add_argument(
    "--model-name",
    type=str,
    required=True,
    help="Name of the model to use",
)
parser.add_argument(
    "--user-api-model-name",
    type=str,
    required=True,
    help="Name of the user API model to use",
)
parser.add_argument(
    "--use-api-for-all",
    action="store_true",
    help="Whether to use API backend",
)
parser.add_argument(
    "--num-gpus",
    type=int,
    default=1,
    help="Number of GPUs to use for local inference (default: 1)"
)
parser.add_argument(
    "--fc",
    action="store_true",
    help="Enable function calling mode: pass tools to apply_chat_template instead of including in prompt"
)


args = parser.parse_args()

import fcntl
lock_file_path = "/tmp/maturin_build_lock"
print("Acquiring build lock...")
with open(lock_file_path, "w") as lock_file:
    # Acquire exclusive lock (blocks until available)
    fcntl.flock(lock_file.fileno(), fcntl.LOCK_EX)
    try:
        print("Building Rust extension with maturin develop...")
        # result = subprocess.run(["maturin", "develop"], check=True)
        result = subprocess.run(["maturin", "develop", "--release"], check=True)
        print("Installed Rust extension successfully.")
        time.sleep(2)  # Give some time for the build to complete
    finally:
        # Release lock
        fcntl.flock(lock_file.fileno(), fcntl.LOCK_UN)
        print("Released build lock.")


from rust_code import *


async def main():
    print("use_api_for_all:", args.use_api_for_all)
    print("fc (function calling):", args.fc)

    # create a AceGenerator instance
    runner = AceGenerator(args.model_name, args.fc)
    async def process_single_task_async(task: dict) -> dict:
            global client, api_backend_created, engine, vllm_backend_created, tokenizer
            currently_using_api = args.use_api_for_all or task['role'] == 'user'
            if currently_using_api and not api_backend_created:
                client = create_api_backend(args.user_api_model_name)
                api_backend_created = True
            elif not currently_using_api and not vllm_backend_created:
                engine, tokenizer = create_vllm_backend(args.model_name, args.num_gpus)
                vllm_backend_created = True
            if currently_using_api:
                response = await call_api_model_async(
                    client,
                    args.user_api_model_name,
                    task["system_prompt"],
                    task["user_prompt"],
                )
            else:
                # Validate tools field based on FC mode
                tools = task.get("tools")
                assert (args.fc and tools is not None) or (not args.fc and tools is None), \
                    f"FC mode mismatch: fc={args.fc}, tools={'present' if tools else 'missing'} in task {task['identifier']}"
                response = await call_vllm_model_async(
                    args.model_name,
                    engine,
                    tokenizer,
                    task["system_prompt"],
                    task["user_prompt"],
                    tools,
                )
            response_dict = {
                "identifier": task["identifier"],
                "response": response,
            }
            return response_dict
    async def worker(wid):
        while True:
            task = runner.next_task()
            if task is None:
                # print(f"Worker {wid} exiting.")
                break
            task = json.loads(task)
            # print(f"Worker {wid} processing task {task['identifier']}")
            response_dict = await process_single_task_async(task)
            runner.receive_response(json.dumps(response_dict))
    workers = [asyncio.create_task(worker(wid)) for wid in range(200)]
    await asyncio.gather(*workers)

    # sort
    runner.sort_all_files_after_generation()
    evaluate_all_results(args.model_name, args.fc)


if __name__ == "__main__":
    asyncio.run(main())




