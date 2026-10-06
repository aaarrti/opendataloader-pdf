import json
import subprocess
import sys
from pathlib import Path

import pytest

from agent_loops import actor_critic
from agent_loops import util


@pytest.fixture
def task(tmp_path: Path) -> Path:
    repo = tmp_path / "repo"
    repo.mkdir()
    task_file = repo / "TASK.md"
    task_file.write_text("# Tasks\n\n- [ ] implement it\n")
    return task_file


@pytest.fixture
def codex_home(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> Path:
    home = tmp_path / "shared-codex"
    for skill in (
        "karpathy-guidelines",
        "google-developer-documentation-style",
        "unrelated-skill",
    ):
        skill_dir = home / "skills" / skill
        skill_dir.mkdir(parents=True)
        (skill_dir / "SKILL.md").write_text(skill)

    for plugin in ("ponytail", "unrelated"):
        (home / "plugins" / "cache" / plugin).mkdir(parents=True)
    ponytail_data = home / "plugins" / "data" / "ponytail-ponytail"
    ponytail_data.mkdir(parents=True)
    (ponytail_data / ".ponytail-active").write_text("full")
    (ponytail_data / "private-state").write_text("do not share")
    (home / "plugins" / "data" / "unrelated").mkdir()

    (home / "auth.json").write_text("{}")
    (home / "config.toml").write_text("""[mcp_servers.codebase-memory-mcp]
command = "codebase-memory-mcp"

[mcp_servers.unrelated]
command = "unrelated-mcp"

[marketplaces.ponytail]
source_type = "git"
source = "https://example.test/ponytail.git"

[marketplaces.unrelated]
source_type = "git"
source = "https://example.test/unrelated.git"

[plugins."ponytail@ponytail"]
enabled = true

[plugins."unrelated@unrelated"]
enabled = true
""")
    monkeypatch.setenv("CODEX_HOME", str(home))
    return home


def codex_json(thread_id: str | None, message: str = "done") -> str:
    events = []
    if thread_id:
        events.append({"type": "thread.started", "thread_id": thread_id})
    events.append({"type": "item.completed", "item": {"type": "agent_message", "text": message}})
    return "\n".join(map(json.dumps, events))


def test_final_message_returns_last_completed_agent_message() -> None:
    events = [
        {"type": "thread.started"},
        {"type": "item.completed", "item": {"type": "agent_message", "text": "first"}},
        {"type": "item.completed", "item": {"type": "reasoning", "text": "ignored"}},
        {"type": "item.completed", "item": {"type": "agent_message", "text": "last"}},
    ]

    assert util.final_message("\n".join(map(json.dumps, events))) == "last"


def test_role_homes_are_distinct_stable_and_outside_worktree(task: Path, codex_home: Path) -> None:
    actor_home = actor_critic.role_codex_home("actor", task)
    critic_home = actor_critic.role_codex_home("critic", task)
    actor_home_again = actor_critic.role_codex_home("actor", task)

    assert actor_home == actor_home_again
    assert actor_home != critic_home
    assert actor_home.is_relative_to(codex_home)
    assert critic_home.is_relative_to(codex_home)
    assert not actor_home.is_relative_to(task.parent)
    assert not critic_home.is_relative_to(task.parent)
    for role_home in (actor_home, critic_home):
        for skill in ("karpathy-guidelines", "google-developer-documentation-style"):
            assert (role_home / "skills" / skill).is_symlink()
        assert (role_home / "plugins" / "cache" / "ponytail").is_symlink()
        assert not (
            role_home / "plugins" / "data" / "ponytail-ponytail" / ".ponytail-active"
        ).is_symlink()


def test_clean_worktree_prevents_critic_codex_call(
    task: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    clean_status = subprocess.CompletedProcess([], 0, stdout="", stderr="")
    called = False

    def run_codex(*args, **kwargs):
        nonlocal called
        called = True

    monkeypatch.setattr(actor_critic.subprocess, "run", lambda *args, **kwargs: clean_status)
    monkeypatch.setattr(actor_critic, "run_codex", run_codex)

    with pytest.raises(RuntimeError, match="no changed files"):
        actor_critic.critic_once(task, None, [])

    assert not called


def test_role_defaults_reach_only_their_codex_calls(
    task: Path,
    codex_home: Path,
    monkeypatch: pytest.MonkeyPatch,
    capsys: pytest.CaptureFixture[str],
) -> None:
    argv = [
        "actor_critic.py",
        "--task",
        str(task),
        "--max-iterations",
        "1",
        "--actor-model",
        "actor-model",
        "--critic-model",
        "critic-model",
    ]
    codex_calls = []

    def run(command, **kwargs):
        if command[:3] == ["git", "status", "--porcelain"]:
            return subprocess.CompletedProcess(command, 0, stdout=" M changed.txt\n", stderr="")
        codex_calls.append((command, kwargs["env"]["CODEX_HOME"]))
        role = Path(kwargs["env"]["CODEX_HOME"]).name
        text = "actor done" if role == "actor" else "<promise>COMPLETE</promise>"
        return subprocess.CompletedProcess(command, 0, stdout=codex_json(None, text), stderr="")

    monkeypatch.setattr(sys, "argv", argv)
    monkeypatch.setattr(actor_critic.subprocess, "run", run)

    actor_critic.run_actor_critic(actor_critic.parse_args())
    capsys.readouterr()

    actor_call, critic_call = codex_calls
    assert actor_call[0][:4] == ["codex", "exec", "--approve-for-me", "--json"]
    assert critic_call[0][:4] == ["codex", "exec", "--approve-for-me", "--json"]
    assert actor_call[0][-1] == critic_call[0][-1] == "-"
    assert ["--model", "actor-model"] == actor_call[0][4:6]
    assert ["--model", "critic-model"] == critic_call[0][4:6]
    assert 'model_reasoning_effort="medium"' in actor_call[0]
    assert 'model_reasoning_effort="high"' in critic_call[0]
    assert not any("unrelated" in argument for argument in actor_call[0] + critic_call[0])
    assert actor_call[1] != critic_call[1]


def test_persistence_pins_and_resumes_distinct_role_sessions(
    task: Path, codex_home: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    session_ids = {
        "actor": "11111111-1111-4111-8111-111111111111",
        "critic": "22222222-2222-4222-8222-222222222222",
    }
    calls = []

    def run(command, **kwargs):
        role = Path(kwargs["env"]["CODEX_HOME"]).name
        calls.append((role, command))
        return subprocess.CompletedProcess(
            command, 0, stdout=codex_json(session_ids[role]), stderr=""
        )

    monkeypatch.setattr(actor_critic.subprocess, "run", run)

    actor_critic.run_codex("actor", task, None, [], "work", persist_sessions=True)
    actor_critic.run_codex("actor", task, None, [], "continue", persist_sessions=True)
    actor_critic.run_codex("critic", task, None, [], "review", persist_sessions=True)
    actor_critic.run_codex("critic", task, None, [], "continue", persist_sessions=True)

    assert "resume" not in calls[0][1]
    assert calls[0][1][3] == "--json"
    assert calls[1][1][3:6] == ["resume", "--json", "--config"]
    assert session_ids["actor"] in calls[1][1]
    assert session_ids["critic"] in calls[3][1]
    assert (codex_home / "actor-critic").exists()
    for role, session_id in session_ids.items():
        role_home = actor_critic.role_codex_home(role, task)
        assert (role_home / "session-id").read_text().strip() == session_id


def test_persistence_is_off_by_default_even_with_an_existing_session(
    task: Path, codex_home: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    role_home = actor_critic.role_codex_home("actor", task)
    session_file = role_home / "session-id"
    session_file.write_text("33333333-3333-4333-8333-333333333333\n")
    calls = []

    def run(command, **kwargs):
        calls.append(command)
        return subprocess.CompletedProcess(command, 0, stdout=codex_json(None), stderr="")

    monkeypatch.setattr(actor_critic.subprocess, "run", run)

    actor_critic.run_codex("actor", task, None, [], "work")

    assert "resume" not in calls[0]
    assert session_file.read_text() == "33333333-3333-4333-8333-333333333333\n"


def test_persistent_session_requires_thread_id(
    task: Path, codex_home: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    monkeypatch.setattr(
        actor_critic.subprocess,
        "run",
        lambda command, **kwargs: subprocess.CompletedProcess(
            command, 0, stdout=codex_json(None), stderr=""
        ),
    )

    with pytest.raises(RuntimeError, match="no thread ID"):
        actor_critic.run_codex("actor", task, None, [], "work", persist_sessions=True)

    role_home = actor_critic.role_codex_home("actor", task)
    assert not (role_home / "session-id").exists()
