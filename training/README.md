# training/

The learned opponent's trainer (docs/design/training-ai.md §7–§11): recurrent PPO by self-play in a league,
written in PyTorch, never built with the app. The app runs the exported network with a Rust forward pass
(MODEL.md describes it exactly).

```
saito_ai/fake.py        FakePool: §6.6's pool in numpy, with a learnable toy game (tests, --fake)
saito_ai/model.py       the network, its MAC count (python -m saito_ai.model [--size large])
saito_ai/rollout.py     per-seat decision streams, compact rollout storage, the collector
saito_ai/ppo.py         GAE, the PPO update, input normalization
saito_ai/league.py      self-play, PFSP snapshots, scripted players, Elo
saito_ai/curriculum.py  the HP curriculum
saito_ai/device.py      the device, each thread's CUDA stream, copies that don't wait
saito_ai/train.py       the training loop (collector and learner threads, overlapped) and its CLI
saito_ai/evaluate.py    checkpoints against bots and each other, per game; Elo
saito_ai/export.py      safetensors for the Rust side, and a parity fixture
saito_ai/bench.py       the simulation, inference and update timed apart; update variants; the loop (--loop)
bridge/                  the pyo3 module saito_gym (its own Cargo workspace)
cloud/                   setup, run and sync scripts for the cloud machine
configs/                 smoke.toml (a short run), mac.toml (20 minutes on a Mac), cloud.toml (a full run)
```

## Locally

Everything goes through [uv](https://docs.astral.sh/uv/); nothing is installed globally.

```sh
cd training
uv sync                                   # Python 3.13, torch, numpy, safetensors, tensorboard, pytest
uv run pytest -q                          # about two minutes on a laptop's CPU
uv run python -m saito_ai.model          # MACs a decision and parameters of the default size
```

On FakePool (no Rust needed):

```sh
uv run python -m saito_ai.train --run-dir runs/try --config configs/smoke.toml --fake
uv run python -m saito_ai.evaluate runs/try --fake --envs 20 --rounds 2
uv run python -m saito_ai.export runs/try --out runs/try/saito-ai.safetensors
uv run tensorboard --logdir runs
```

On the real engine, build the bridge into this environment once (and after engine changes), then drop
`--fake`. `uv sync` removes it again (it isn't in the lock file), so run with `uv run --no-sync` after building:

```sh
uv run maturin develop --release --uv --manifest-path bridge/Cargo.toml
uv run --no-sync python -m saito_ai.train --run-dir runs/mac --config configs/mac.toml
uv run --no-sync python -m saito_ai.bench --profile     # where the time goes
uv run --no-sync python -m saito_ai.bench --variants base --loop 8   # and the loop's
```

Training overlaps collection with the update: a collector thread plays the next rollout with a copy of the
learner's weights (one update behind) while the learner updates on the last one (docs/design/training-ai.md
§8). There is no other way it runs.

Training writes `RUN_DIR/tb` (TensorBoard), `RUN_DIR/checkpoints` (each with the config, the git commit and the
observation spec; `--resume` continues the latest) and `RUN_DIR/snapshots` (the league's frozen versions).
`--init-from CHECKPOINT` (a file, or a run directory's latest) starts a new run from another run's network:
update 0, a fresh optimizer, league and curriculum, the checkpoint's model widths and input normalization, and the
GRU's previous-action columns zero where its network had none, so it plays as that checkpoint did (§8).
Settings: a TOML file (`--config`), a named size (`--size large`), and overrides (`--set ppo.lr=1e-4`).
`saito_ai/config.py` has every setting and its default.

Held-out tests (§9): `--train-games exe5,exe6 --eval-games exe4` trains on two games and evaluates on the third
every `eval.every` updates; `--set env.chip_holdout=0.1` keeps a tenth of each game's chips out of training
builds and favors them in evaluation builds.

## In the cloud

1. Put the repo on the machine, with its content submodules (the battles are the content's alone: no asset pack,
   picture or sound goes to the machine):
   ```sh
   ssh HOST git clone --recurse-submodules git@github.com:masuexe/saito.git saito     # or rsync the checkout
   ```
2. On the machine: `saito/training/cloud/setup.sh` (Rust, uv, the bridge, tests and a smoke run).
3. Start a run: `saito/training/cloud/run.sh first --config configs/cloud.toml` (tmux session `train-first`,
   TensorBoard on the machine's localhost:6006; `ssh -L 6006:localhost:6006 HOST` to see it).
4. Locally, fetch checkpoints and logs: `training/cloud/sync.sh HOST saito first`.
5. Publish the best checkpoint (by evaluate's Elo) for the app as a release of github.com/masuexe/saito-ai
   (docs/design/training-ai.md §10.5): `uv run python -m saito_ai.publish runs/first/checkpoints/ckpt_NNNNNNN.pt
   --release` (the GitHub CLI signed in); the repository's Pages workflow puts it on the site the app reads.
   (`SAITO_AI_WEIGHTS=<file>` plays an exported file in the app without publishing it.)
