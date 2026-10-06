import argparse
import hashlib
import json
import os
import subprocess
import sys
import tomllib
from pathlib import Path
from uuid import UUID

from agent_loops.util import codex_output

RECOMMENDED_SKILLS = (
    "karpathy-guidelines",
    "google-developer-documentation-style",
)
RECOMMENDED_CONFIG_PATHS = (
    ("mcp_servers", "codebase-memory-mcp"),
    ("marketplaces", "ponytail"),
    ("plugins", "ponytail@ponytail"),
)


def shared_codex_home() -> Path:
    return Path(os.environ.get("CODEX_HOME", Path.home() / ".codex"))


def link_directory(source: Path, destination: Path) -> None:
    if not source.is_dir():
        raise RuntimeError(f"recommended Codex setup not found: {source}")
    if destination.is_symlink():
        if destination.resolve() != source.resolve():
            raise RuntimeError(f"unexpected Codex setup link: {destination}")
        return
    if destination.exists():
        raise RuntimeError(f"unexpected Codex setup path: {destination}")
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.symlink_to(source, target_is_directory=True)


def inherit_recommended_setup(shared_home: Path, role_home: Path) -> None:
    for skill in RECOMMENDED_SKILLS:
        link_directory(shared_home / "skills" / skill, role_home / "skills" / skill)
    link_directory(
        shared_home / "plugins" / "cache" / "ponytail",
        role_home / "plugins" / "cache" / "ponytail",
    )

    source_marker = shared_home / "plugins" / "data" / "ponytail-ponytail" / ".ponytail-active"
    if not source_marker.is_file():
        raise RuntimeError(f"recommended Codex setup not found: {source_marker}")
    role_marker = role_home / "plugins" / "data" / "ponytail-ponytail" / ".ponytail-active"
    role_marker.parent.mkdir(parents=True, exist_ok=True)
    role_marker.write_text(source_marker.read_text())


def toml_key(key: str) -> str:
    return (
        key
        if all(character.isalnum() or character in "_-" for character in key)
        else json.dumps(key)
    )


def toml_value(value: object) -> str:
    if isinstance(value, str):
        return json.dumps(value)
    if isinstance(value, bool):
        return str(value).lower()
    if isinstance(value, (int, float)):
        return str(value)
    if isinstance(value, list):
        return f"[{', '.join(toml_value(item) for item in value)}]"
    raise RuntimeError(f"unsupported recommended Codex config value: {value!r}")


def flatten_config(path: tuple[str, ...], value: object) -> list[str]:
    if isinstance(value, dict):
        return [
            override
            for key, child in value.items()
            for override in flatten_config((*path, key), child)
        ]
    return [f"{'.'.join(map(toml_key, path))}={toml_value(value)}"]


def recommended_configs(shared_home: Path) -> list[str]:
    config_file = shared_home / "config.toml"
    if not config_file.is_file():
        raise RuntimeError(f"recommended Codex setup not found: {config_file}")
    config = tomllib.loads(config_file.read_text())
    overrides: list[str] = []
    for path in RECOMMENDED_CONFIG_PATHS:
        value: object = config
        for key in path:
            if not isinstance(value, dict) or key not in value:
                raise RuntimeError(f"recommended Codex config not found: {'.'.join(path)}")
            value = value[key]
        overrides.extend(flatten_config(path, value))
    return overrides


def role_codex_home(role: str, task_file: Path) -> Path:
    task_key = hashlib.sha256(str(task_file.resolve()).encode()).hexdigest()[:16]
    shared_home = shared_codex_home()
    session_root = shared_home / "actor-critic" / task_key
    role_home = session_root / role
    role_home.mkdir(parents=True, exist_ok=True, mode=0o700)
    session_root.chmod(0o700)
    role_home.chmod(0o700)

    shared_auth = shared_home / "auth.json"
    role_auth = role_home / "auth.json"
    if shared_auth.exists() and not role_auth.exists():
        role_auth.symlink_to(shared_auth)
    inherit_recommended_setup(shared_home, role_home)
    return role_home


def validated_session_id(value: str) -> str:
    try:
        UUID(value)
    except ValueError as error:
        raise RuntimeError(f"invalid persisted Codex session ID: {value!r}") from error
    return value


def run_codex(
    role: str,
    task_file: Path,
    model: str | None,
    configs: list[str],
    prompt: str,
    persist_sessions: bool = False,
) -> str:
    role_home = role_codex_home(role, task_file)
    session_file = role_home / "session-id"
    session_id = None
    if persist_sessions and session_file.exists():
        session_id = validated_session_id(session_file.read_text().strip())

    command = ["codex", "exec", "--approve-for-me"]
    if session_id:
        command.append("resume")
    command.append("--json")
    if model:
        command.extend(("--model", model))
    for config in (*configs, *recommended_configs(shared_codex_home())):
        command.extend(("--config", config))
    if session_id:
        command.append(session_id)
    command.append("-")

    environment = os.environ.copy()
    environment["CODEX_HOME"] = str(role_home)
    result = subprocess.run(command, input=prompt, text=True, capture_output=True, env=environment)
    if result.stderr:
        print(result.stderr, file=sys.stderr, end="")
    if result.returncode:
        raise RuntimeError(f"{role} Codex exited with status {result.returncode}")
    started_thread_id, message = codex_output(result.stdout)
    if persist_sessions and session_id is None:
        if started_thread_id is None:
            raise RuntimeError(f"{role} Codex JSON output contained no thread ID")
        session_file.write_text(f"{validated_session_id(started_thread_id)}\n")
    if session_id and started_thread_id and started_thread_id != session_id:
        raise RuntimeError(f"{role} Codex resumed an unexpected session")
    return message


def actor_once(
    task_file: Path,
    model: str | None,
    configs: list[str],
    persist_sessions: bool = False,
) -> None:
    message = run_codex(
        "actor",
        task_file,
        model,
        configs,
        f"""Read {task_file} before working.

1. Find the highest-priority incomplete task section, including its latest critic comment.
2. Implement only that section's requested work and run relevant checks.
3. Do not edit {task_file}, mark any task complete, or write a progress summary. The critic alone controls task status and whether work can advance.
4. Stop after this one task section; the critic will review it next.
""",
        persist_sessions=persist_sessions,
    )
    print(message)


def critic_once(
    task_file: Path,
    model: str | None,
    configs: list[str],
    persist_sessions: bool = False,
) -> bool:
    status = subprocess.run(["git", "status", "--porcelain"], text=True, capture_output=True)
    if status.returncode:
        raise RuntimeError("git status failed")
    if not status.stdout.strip():
        raise RuntimeError("no changed files for the critic to review")

    message = run_codex(
        "critic",
        task_file,
        model,
        configs,
        f"""Review the current repository and {task_file}. You are the sole owner of task state.

Find the highest-priority incomplete task section, inspect the actor's work, and run relevant checks.
- If it is acceptable, mark only that section complete and add a concise critic comment in that section explaining the approval.
- If it needs changes, leave it incomplete and replace or add a concise critic comment in that section with actionable feedback.
- Edit only {task_file}; do not modify source files or append progress elsewhere.
- The actor may advance only after you mark its current section complete.

End with exactly one verdict:
- <promise>COMPLETE</promise> only when no incomplete task sections remain after your edit.
- <promise>CONTINUE</promise> otherwise.
""",
        persist_sessions=persist_sessions,
    )
    print(message)

    complete = "<promise>COMPLETE</promise>"
    proceed = "<promise>CONTINUE</promise>"
    if message.count(complete) == 1 and proceed not in message:
        return True
    if message.count(proceed) == 1 and complete not in message:
        return False
    raise RuntimeError("critic response must contain exactly one verdict")


def run_actor_critic(args: argparse.Namespace) -> None:
    actor_configs = [
        f'model_reasoning_effort="{args.actor_reasoning_level}"',
    ]
    critic_configs = [
        f'model_reasoning_effort="{args.critic_reasoning_level}"',
    ]
    for iteration in range(1, args.max_iterations + 1):
        print(f"\n=== actor-critic iteration {iteration} / {args.max_iterations} ===")
        actor_once(args.task, args.actor_model, actor_configs, args.persist_sessions)
        if critic_once(args.task, args.critic_model, critic_configs, args.persist_sessions):
            return print(f"\nAll task sections approved after {iteration} iteration(s).")
    raise RuntimeError(f"max iterations ({args.max_iterations}) reached without critic completion")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--task", required=True, type=Path)
    parser.add_argument("--max-iterations", required=True, type=int)
    parser.add_argument("--actor-model", default="gpt-5.6-luna")
    parser.add_argument("--critic-model", default="gpt-5.6-sol")
    parser.add_argument("--actor-reasoning-level", default="medium")
    parser.add_argument("--critic-reasoning-level", default="high")
    parser.add_argument("--persist-sessions", action="store_true")
    args = parser.parse_args()
    if not args.task.is_file():
        parser.error(f"task file not found: {args.task}")
    if args.max_iterations < 1:
        parser.error("--max-iterations must be positive")
    return args


def main() -> int:
    try:
        run_actor_critic(parse_args())
    except (OSError, RuntimeError) as error:
        print(f"Error: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
