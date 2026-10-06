import argparse
import subprocess
import sys
from pathlib import Path

from agent_loops.util import final_message


def ralph_once(
    task_file: Path,
    progress_file: Path,
    model: str | None,
    reasoning_effort: str | None,
) -> bool:
    progress_file.touch(exist_ok=True)
    command = ["codex", "exec", "--json", "--dangerously-bypass-approvals-and-sandbox"]
    if model:
        command.extend(("--model", model))
    if reasoning_effort:
        command.extend(("--config", f'model_reasoning_effort="{reasoning_effort}"'))
    command.append("-")

    result = subprocess.run(
        command,
        input=f"""Read these files before working:
- {Path.home()}/.codex/skills/karpathy-guidelines/SKILL.md
- {task_file}
- {progress_file}

1. Find the highest-priority incomplete task and implement it.
2. Run tests and type checks.
3. Update {task_file} to mark what was completed.
4. Append a brief progress summary to {progress_file}.
5. Commit changes.
ONLY WORK ON A SINGLE TASK.
If all tasks are complete, output <promise>COMPLETE</promise>.
""",
        text=True,
        capture_output=True,
    )
    if result.stderr:
        print(result.stderr, file=sys.stderr, end="")
    if result.returncode:
        raise RuntimeError(f"Codex exited with status {result.returncode}")

    message = final_message(result.stdout)
    print(message)
    return "<promise>COMPLETE</promise>" in message


def run_ralph(args: argparse.Namespace) -> None:
    for iteration in range(1, args.max_iterations + 1):
        print(f"\n=== ralph iteration {iteration} / {args.max_iterations} ===")
        if ralph_once(args.task, args.progress, args.model, args.reasoning_effort):
            return print(f"\nAll tasks complete after {iteration} iteration(s).")
    raise RuntimeError(f"max iterations ({args.max_iterations}) reached without COMPLETE")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--task", required=True, type=Path)
    parser.add_argument("--progress", required=True, type=Path)
    parser.add_argument("--max-iterations", required=True, type=int)
    parser.add_argument("--model")
    parser.add_argument("--reasoning-effort")
    args = parser.parse_args()
    if not args.task.is_file():
        parser.error(f"task file not found: {args.task}")
    if args.max_iterations < 1:
        parser.error("--max-iterations must be positive")
    return args


def main() -> int:
    try:
        run_ralph(parse_args())
    except (OSError, RuntimeError) as error:
        print(f"Error: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
