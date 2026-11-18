"""Self-play training of saito's learned opponent (docs/design/training-ai.md §7–§11).

The environment is the Rust pool `saito_gym.Pool` (§6.6); `saito_ai.fake.FakePool` stands in for it in
tests and smoke runs (`--fake`). The network trained here is run in the app by a hand-written Rust forward
pass; MODEL.md describes its computation exactly.
"""
