# Orchestration Metrics: Vale Village Rust Multi-Agent Build

Quantitative analysis of 13 agent waves across a 4-hour orchestration session (2026-02-24, 17:56–22:05 UTC). 72 Task dispatches, 38 commits, ~30 source files.

---

## 1. Collision Rate: How Often Did Two Agents Touch the Same File in the Same Wave?

### Raw Data: File Modification Frequency

| File | Total Modifications (across all commits) |
|------|:---:|
| `plugins/ui.rs` | 17 |
| `plugins/overworld.rs` | 14 |
| `battle/systems.rs` | 14 |
| `plugins/core_plugin.rs` | 13 |
| `plugins/battle_ui.rs` | 12 |
| `plugins/save.rs` | 9 |
| `data/enemies.rs` | 7 |
| `battle/djinn.rs` | 6 |
| `battle/damage.rs` | 6 |

### Collision Analysis by Wave

**Wave 1 (pre-wave, 17:56–18:08):** 5 agents dispatched to `abilities.rs`, `systems.rs`, `shop.rs`, `inventory.rs`, `djinn.rs`. **Zero file-level collisions.** Each agent owned exactly one file. However, `systems.rs` was a read-dependency for both the shop and inventory agents (they needed to understand `BattleAction` patterns).

**Wave 2 (18:42):** Integration commit touching 11 files — this was the *orchestrator* merging Wave 1 outputs, not parallel agents. No agent-agent collision because only the orchestrator wrote cross-cutting code.

**Wave 3 (18:59):** Files touched: `systems.rs`, `enemies.rs`, `battle_ui.rs`, `inventory.rs`, `overworld.rs`, `ui.rs`. Six files across multiple agents. **Potential collision on `systems.rs`** which was modified in Wave 2 (18:42) just 17 minutes prior — likely different agents working on the same file in rapid succession, not truly parallel.

**Wave 12 (21:10–21:34):** The densest wave — **15 commits in 24 minutes**. Both `ui.rs` and `core_plugin.rs` received multiple sequential writes:
- `ui.rs`: 7 commits between 21:10 and 21:34 (6 of them in 16 minutes)
- `core_plugin.rs`: 4 commits between 21:10 and 21:31
- Two commits landed 6 seconds apart (21:26:01 and 21:26:07), both touching `ui.rs`

This is the closest thing to a true collision, but it appears the orchestrator was doing sequential integration rather than two agents simultaneously writing to the same file.

### Collision Rate

| Metric | Value |
|--------|-------|
| **Waves with zero file-level agent-agent collision** | 11/13 (85%) |
| **Waves with potential same-file contention** | 2/13 (Wave 3 on `systems.rs`, Wave 12 on `ui.rs`) |
| **True simultaneous write conflicts** | 0/13 (0%) |

**Key finding:** The strict single-file scope enforcement worked — collisions were mechanically impossible because each agent was told "edit ONLY this file." The 2/13 "potential" collisions were actually sequential orchestrator integration writes to the same file, not parallel agent conflicts. The collision rate for actual parallel agent work is **0%**.

**But this masks the real cost:** The orchestrator absorbed all the collision risk by doing all cross-file work itself. This shifts collision from "two agents break each other" to "the orchestrator must manually reconcile every cross-file dependency." See Metric 3.

---

## 2. Integration Cost Per Wave

### Methodology

Each wave follows a pattern: **dispatch agents → agents return → orchestrator integrates → orchestrator gates (check/test) → commit**. We measure integration cost as the time and commits between "last agent returns" and "wave commit."

### Wave-by-Wave Timing

| Wave | Dispatch Time | Commit Time | Duration | Agent Commits | Integration Commits | Integration % |
|------|:---:|:---:|:---:|:---:|:---:|:---:|
| Pre-wave 1 | 17:42 | 17:56 | 14 min | 1 (recon) | 0 | 0% |
| Wave 1 | 17:56 | 18:08 | 12 min | 1 | 1 (clippy clean) | 50% |
| Wave 2 | 18:08 | 18:42 | 34 min | 1 | 1 (11-file integration) | 50% |
| Wave 3 | 18:42 | 18:59 | 17 min | 1 | 0 | 0% |
| Wave 4 | 18:59 | 19:11 | 12 min | 1 | 0 | 0% |
| Wave 5 | 19:11 | 19:27 | 16 min | 1 | 0 | 0% |
| Wave 6 | 19:27 | 19:49 | 22 min | 1 | 1 (story flag wiring) | ~30% |
| Wave 7 | 19:49 | 20:23 | 34 min | 1 | 0 | 0% |
| Wave 8 | 20:23 | 20:31 | 8 min | 1 | 0 | 0% |
| Wave 9 | 20:31 | 20:42 | 11 min | 1 | 0 | 0% |
| Wave 10 | 20:42 | 20:49 | 7 min | 1 | 0 | 0% |
| Wave 11 | 20:49 | 21:00 | 11 min | 1 | 0 | 0% |
| Wave 12 | 21:00 | 21:34 | 34 min | 4+ (WIP commits) | 7+ (sequential fixes) | ~65% |
| Wave 13 | 21:34 | 22:05 | 31 min | 1 | 0 | 0% |

### Integration Cost Summary

| Metric | Value |
|--------|-------|
| **Average wave duration** | ~19 min |
| **Waves requiring integration commits** | 4/13 (31%) |
| **Heaviest integration wave** | Wave 2 (11 files) and Wave 12 (15 commits, 24 min) |
| **Total integration commits** | ~10 of 38 total (26%) |
| **Integration time as % of total session** | ~30% of the 4-hour session |

### Compiler Error Incidents (from Session Log)

| Session Line | Error Code | Description | Cause |
|:---:|:---:|---|---|
| 506 | E0599 (×4) | Method not found | Inventory agent used `EventWriter::write` instead of `send` (wrong Bevy API) |
| 1060 | E0061 (×2) | Wrong number of arguments | Cross-module function signature mismatch |
| 1079 | E0061 (×2) | Wrong number of arguments | Same class of error, different location |
| 1394 | E0063 (×8) | Missing struct fields | Agent-created struct missing fields the integration code expected |
| 3337 | E0425, E0282 | Unresolved name, type inference | Agent referenced a type not imported in its scope |
| 5323 | E0422/E0425/E0433 (×16) | Unresolved imports/names | Massive cluster — likely a late-wave agent that referenced types from multiple other modules |

**Key finding:** The E0599 at line 506 is the canonical example the orchestrator cited in the original session: "classic scope violation from the paper's findings." But notably, this was a *knowledge error* (wrong Bevy API), not a scope collision. Strict file scoping didn't prevent it — the agent simply didn't know Bevy 0.15's `EventWriter` uses `.send()` not `.write()`.

---

## 3. Does Strict File Ownership Reduce Throughput?

### The Tradeoff

Strict scope means zero file-level collisions (see Metric 1). But it creates a different cost: **missing cross-wiring**. When Agent A adds a type to `types.rs` and Agent B needs that type in `systems.rs`, Agent B either can't reference it (if dispatched in parallel) or the orchestrator must manually wire the import.

### Evidence of Cross-Wiring Gaps

**From the session log and git history:**

1. **`spawn_party_battle_units` (Wave 1 → Wave 2 integration):** The abilities agent, shop agent, and inventory agent each wrote to their scoped files. But nobody wired the actual system that spawns party members into battle. The orchestrator had to write `spawn_party_battle_units` in `systems.rs` and register it in `plugin.rs` — touching 2 files that were in different agents' scopes. This was the **single largest integration task** in the entire session.

2. **Plugin registration (every wave):** Each agent that created new systems couldn't add them to the plugin registry (`plugin.rs`, `mod.rs`, `main.rs`) because those files were outside their scope. The orchestrator manually added `.add_systems(...)` calls after every wave. This is visible in the git data: `main.rs` was modified 5 times, `plugins/mod.rs` 5 times — always by the orchestrator, never by agents.

3. **Story flag wiring (Wave 6):** The story-flags agent added `StoryProgress` to `core_plugin.rs`. But the recruitment system in `overworld.rs`, the tower progression in `tower.rs`, and the victory condition in `systems.rs` all needed to read those flags. The orchestrator committed a separate "wire story flags" commit (19:49) just 3 minutes after the wave commit (19:46).

4. **Clippy fixes in foreign files (Wave 1):** Running `cargo clippy -D warnings` surfaced 23 lint errors across the codebase. Agents were told to fix clippy in their file, but 15+ of these `collapsible_if` warnings were in files like `ai.rs`, `overworld.rs`, `ui.rs` — files that belonged to no agent in Wave 1. The orchestrator fixed all of them.

### Quantification

| Metric | Value |
|--------|-------|
| **Files only the orchestrator modified** | `main.rs`, `plugins/mod.rs`, `plugins/tower.rs` (registration/wiring only) |
| **Explicit "wiring" commits** | 3 (18:08 clippy clean, 19:49 story flag wiring, 21:18 audio wiring) |
| **Cross-module imports the orchestrator manually added** | ~15+ (use statements, re-exports, plugin registrations) |
| **% of integration that is pure wiring (not logic)** | ~70% — most integration commits add imports, register systems, or connect types across module boundaries |

### Verdict

Strict file ownership **eliminated collisions at the cost of creating a wiring bottleneck at the orchestrator.** This is the correct tradeoff for a single-orchestrator system: the orchestrator can hold the full dependency graph in context and wire efficiently, while agents writing to each other's files would create unpredictable merge conflicts.

But it means **the orchestrator is the throughput ceiling.** The agents can run in parallel, but integration is serial and scales linearly with the number of cross-module dependencies. For a 30-file Rust project, this worked. For a 300-file project, the wiring backlog would likely dominate.

---

## 4. Gate Ordering: Clippy as a Behavior-Shaping Gate

### Prescribed Gate Sequence

The orchestrator's dispatch prompts to every agent included this explicit gate ordering:

```
1. Run `cargo check` after changes
2. Run `cargo clippy --all-targets --all-features -- -D warnings` and fix warnings
3. Run `cargo fmt`
```

Notably: **`cargo test` was not in the standard agent gate sequence.** Tests were prescribed only for agents working on pure-logic modules (damage, AI, rewards), e.g.:
```
- Run `cargo test battle::damage` after changes
```

### Gate Invocation Frequency (from Session Log)

| Command | Occurrences in Session |
|---------|:---:|
| `cargo check` | ~200+ |
| `cargo test` | ~170 |
| `cargo clippy` | ~40 (mostly in dispatch prompts, not raw execution) |
| `cargo fmt` | ~15 |
| `cargo build` | 0 |

### Clippy as Behavior Shaping

**Key evidence:** In Wave 1 integration, `cargo clippy -D warnings` forced refactors in 6 files the agents didn't own:

| File | Clippy Issue | Fix Applied |
|------|-------------|-------------|
| `battle/ai.rs` | `collapsible_if` | Collapsed nested `if hp_pct < 0.30 { if let Some(action) = ... }` into let-chain |
| `battle/damage.rs` | `collapsible_if` | Collapsed nested if in `consume_shield_charge` |
| `data/djinn.rs` | manual `Default` impl | Changed to `#[derive(Default)]` |
| `plugins/battle_ui.rs` | `too_many_arguments` | Added `#[allow(clippy::too_many_arguments)]` |
| `plugins/overworld.rs` | `too_many_arguments` + `collapsible_if` | Allow attribute + collapsed nested conditionals |
| `plugins/ui.rs` | `too_many_arguments` | Allow attributes on 2 functions |

**This is the paper's finding applied to Rust, but with a twist.** In TypeScript (the paper's corpus), there's no equivalent to clippy — the closest is ESLint, which is rarely run with `--error` severity. In Rust:

1. **`cargo check` catches type errors** — this is the basic compilation gate. It found the E0599 (wrong Bevy API), E0061 (wrong arg count), and E0063 (missing fields) errors.

2. **`cargo clippy -D warnings` catches *style* errors that force structural refactors** — collapsible_if requires rewriting control flow, too_many_arguments forces either function decomposition or explicit suppression. These are code-quality decisions, not correctness decisions.

3. **`cargo test` catches semantic errors** — wrong damage calculations, incorrect turn ordering, etc.

### Gate Ordering Matters Because of Failure Mode

| Gate | Failure Mode | Fix Required | Blast Radius |
|------|-------------|-------------|:---:|
| `cargo check` | Won't compile | Fix types/imports | Local (1 file) |
| `cargo clippy` | Compiles but idiomatic violations | Refactor structure | Often cross-file (6 files in Wave 1) |
| `cargo test` | Compiles + passes lint but wrong behavior | Fix logic | Local (1 function) |

**Clippy is the middle gate that causes the most unexpected work.** It compiles fine — the agent thinks it's done — but clippy forces structural changes that can cascade. In Wave 1, a single `cargo clippy` run created work in 6 files across 4 modules.

### The Missed Optimization

The orchestrator **should have run clippy first, before dispatching agents.** The 23 pre-existing clippy warnings were technical debt from the Gemini WIP commit (4d6fd73d). If the orchestrator had cleaned those before Wave 1, the agents wouldn't have encountered them and the integration phase wouldn't have needed the "clippy clean" commit.

In general, for Rust multi-agent builds: **clippy → check → test** is the optimal gate order because:
- Clippy failures propagate furthest (cross-file style enforcement)
- Check failures are local (type errors in the file you changed)
- Test failures are semantic (logic errors in the function you changed)

Running clippy last means you discover the most disruptive issues after all the local work is done.

---

## Summary Table

| Metric | Value | Implication |
|--------|-------|-------------|
| Agent-agent file collision rate | **0%** (0/13 waves) | Strict scope works perfectly for preventing conflicts |
| Integration cost as % of session | **~30%** | Orchestrator spends 1/3 of its time on merge+fix+gate |
| Wiring commits as % of total | **26%** (10/38) | 1 in 4 commits is pure cross-module plumbing |
| Heaviest integration wave | **Wave 12** (15 commits, 24 min, 65% integration) | Complexity scales non-linearly with agent count |
| Cross-wiring gaps from strict scope | **~15+ manual import additions** | Strict scope trades collisions for wiring bottleneck |
| Most disruptive gate | **`cargo clippy`** (forced 6-file refactor in Wave 1) | Clippy should run *before* agent dispatch, not after |
| Gate most often run | **`cargo check`** (~200 invocations) | Type checking is the primary feedback loop |
| `cargo test` invocations | **~170** | Tests are secondary to compilation in practice |
