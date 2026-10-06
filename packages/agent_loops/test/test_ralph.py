import json
import subprocess
from pathlib import Path

from agent_loops import ralph


def test_ralph_once_uses_json_and_detects_completion(tmp_path: Path, monkeypatch) -> None:
    task = tmp_path / "TASK.md"
    progress = tmp_path / "PROGRESS.md"
    task.write_text("- [ ] work\n")
    event = {
        "type": "item.completed",
        "item": {"type": "agent_message", "text": "<promise>COMPLETE</promise>"},
    }

    def run(command, **kwargs):
        assert command == [
            "codex",
            "exec",
            "--json",
            "--dangerously-bypass-approvals-and-sandbox",
            "--model",
            "test-model",
            "--config",
            'model_reasoning_effort="high"',
            "-",
        ]
        return subprocess.CompletedProcess(command, 0, stdout=json.dumps(event), stderr="")

    monkeypatch.setattr(ralph.subprocess, "run", run)

    assert ralph.ralph_once(task, progress, "test-model", "high")
    assert progress.is_file()
