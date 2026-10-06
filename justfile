set shell := ["zsh", "-cu"]

cfr_version := "0.152"
cfr_jar := "target/tools/cfr-" + cfr_version + ".jar"
cfr_sha256 := "f686e8f3ded377d7bc87d216a90e9e9512df4156e75b06c655a16648ae8765b2"
cfr_release_url := "https://github.com/leibnitz27/cfr/releases/download/" + cfr_version + "/cfr-" + cfr_version + ".jar"

download-cfr:
    mkdir -p target/tools && curl --fail --location --show-error --retry 3 --output "{{cfr_jar}}.part" "{{cfr_release_url}}" && printf '%s  %s\n' "{{cfr_sha256}}" "{{cfr_jar}}.part" | sha256sum --check && mv "{{cfr_jar}}.part" "{{cfr_jar}}"

decompile-cfr jar output_dir:
    java -jar "{{cfr_jar}}" "{{jar}}" --outputdir "{{output_dir}}"

actor-critic-loop:
    uv run -m agent_loops.actor_critic --task .codex/PLAN.md --max-iterations 100 \
    --actor-model gpt-5.6-luna --critic-model gpt-5.6-sol \
    --actor-reasoning-level medium --critic-reasoning-level high

ralph-loop:
    uv run -m agent_loops.ralph --task .codex/PLAN_ut.md --progress .codex/PROGRESS_ut.md \
    --model gpt-5.6-luna --reasoning-effort medium \
    --max-iterations 30
