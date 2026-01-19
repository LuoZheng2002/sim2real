


from dotenv import load_dotenv
import argparse
import subprocess
import time
import asyncio
import json

from src_py.api_backend import call_api_model_async, create_api_backend
from src_py.vllm_backend import call_vllm_model_async, call_vllm_model_async, create_vllm_backend
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
    type=bool,
    required=True,
    help="Whether to use API backend",
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

    # create a AceGenerator instance
    runner = AceGenerator(args.model_name)
    async def process_single_task_async(task: dict) -> dict:
            global client, api_backend_created, engine, vllm_backend_created, tokenizer
            currently_using_api = args.use_api_for_all or task['role'] == 'user'
            if currently_using_api and not api_backend_created:
                client = create_api_backend(args.user_api_model_name)
                api_backend_created = True
            elif not currently_using_api and not vllm_backend_created:
                engine, tokenizer = create_vllm_backend(args.model_name)
                vllm_backend_created = True
            if currently_using_api:
                response = await call_api_model_async(
                    client,
                    args.user_api_model_name,
                    task["system_prompt"],
                    task["user_prompt"],
                )
            else:
                response = await call_vllm_model_async(
                    engine,
                    tokenizer,
                    task["system_prompt"],
                    task["user_prompt"],
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
    workers = [asyncio.create_task(worker(wid)) for wid in range(5)]
    await asyncio.gather(*workers)

if __name__ == "__main__":
    asyncio.run(main())




