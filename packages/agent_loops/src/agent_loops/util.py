import json


def codex_output(jsonl: str) -> tuple[str | None, str]:
    thread_id: str | None = None
    messages: list[str] = []
    for line_number, line in enumerate(jsonl.splitlines(), 1):
        if not line.strip():
            continue
        try:
            event = json.loads(line)
        except json.JSONDecodeError as error:
            raise RuntimeError(f"invalid Codex JSON on line {line_number}: {error}") from error
        if event.get("type") == "thread.started" and isinstance(event.get("thread_id"), str):
            thread_id = event["thread_id"]
        item = event.get("item", {})
        if event.get("type") == "item.completed" and item.get("type") == "agent_message":
            messages.append(item.get("text", ""))
    if not messages:
        raise RuntimeError("Codex JSON output contained no final agent message")
    return thread_id, messages[-1]


def final_message(jsonl: str) -> str:
    return codex_output(jsonl)[1]
