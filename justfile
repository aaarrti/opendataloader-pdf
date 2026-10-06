set shell := ["zsh", "-cu"]



actor-critic-loop:
    uv run -m agent_loops.actor_critic --task .codex/PLAN.md --max-iterations 100 \
    --actor-model gpt-5.6-luna --critic-model gpt-5.6-sol \
    --actor-reasoning-level medium --critic-reasoning-level high

ralph-loop:
    uv run -m agent_loops.ralph --task .codex/PLAN_ut.md --progress .codex/PROGRESS_ut.md \
    --model gpt-5.6-luna --reasoning-effort medium \
    --max-iterations 30