# Idle-queue supervisor — creation spec (CREATED 2026-10-17: automation-a60b5841-7b60-43c2-9cc6-47a58c624c40; old worker automation-c6386d8d… deleted)

## Why this exists (platform constraints, verified 2026-10-17)

The user wants the migration to run CONTINUOUSLY on free idle-time
compute. Three platform rules shape the design:

1. An idle-time task's prompt must never ask its run to create further
   idle-time tasks or automations, and an idle-time run must never call
   OffPeakCreate — self-chaining is forbidden.
2. A session that belongs to a scheduled/idle task cannot create NEW
   automations either ("Cannot create a scheduled task inside a session
   that already belongs to a scheduled task").
3. Idle-time runs have no background agents (dispatch foreground only).

Therefore replenishment must be owned by a SCHEDULED AUTOMATION created
from a normal interactive chat: the supervisor fires on a cron, checks
the idle queue (OffPeakList), and pre-stages successors via OffPeakCreate
(cron runs are not idle-time runs, so creating there is legal). The cron
re-fires forever, so the chain cannot break the way a self-chain would.

## TO CREATE (one step, in a FRESH chat in this workspace)

Ask the agent: `按 migration/SUPERVISOR.md 创建监督者定时任务`
(or any equivalent). Parameters, verbatim:

- **CronCreate cron**: `0 */4 * * *` (user-chosen cadence; any
  `0 */N * * *` works — smaller N = tighter gap coverage, more tickets)
- **title**: `每4小时补给闲时迁移队列(监督者)`
- **prompt**: the SUPERVISOR PROMPT block below, verbatim.

Also optionally delete the older paused automation
`automation-c6386d8d…` ("每4小时推进 Glean→Rust 迁移", worker-mode,
runCount 0) to avoid confusion if ever re-enabled.

## SUPERVISOR PROMPT

```text
You are the idle-queue SUPERVISOR for the Glean→Rust migration (a
scheduled automation run — you are NOT an idle-time task run, so only
you may create idle-time tasks). Repo: the Glean/ checkout in this
workspace, branch migration/rust-port. Do NOT do ordinary migration
work except in the narrow heartbeat case of Step 3.

Step 1 — inspect: call OffPeakList. Count tasks whose status is queued
or running (IGNORE paused tasks — user-held spares; never attempt to
resume or delete them, there is no tool anyway). Also run
`git -C Glean log -1 --format='%h %cI %s'` and note the newest
commit's timestamp.

Step 2 — replenish (target state: never zero queued-or-running; at
most one queued AND at most one running):
- none queued and none running → create ONE idle-time task via
  OffPeakCreate;
- exactly one running and none queued → create ONE successor NOW
  (pre-staged so execution continues the moment the running task
  finishes — gap-free continuation);
- one already queued → create nothing.
The idle-time task to create — title: 推进 Glean→Rust 迁移(闲时) —
prompt, pass VERBATIM:

"Continue the Glean→Rust migration as an unattended idle-time session.
Repo: the Glean/ checkout in this workspace, branch migration/rust-port.
Follow Glean/migration/RUNBOOK.md exactly — first read
Glean/migration/DECISIONS.md, Glean/migration/HANDOFF.md (STATUS
block), Glean/migration/RUNBOOK.md, Glean/migration/RULEBOOK.md.
Orient per RUNBOOK Step 0 (verify branch; triage git-status leftovers:
complete-but-unreviewed drafts go to review, partial drafts get a
completion round; seconds-fresh files mean another live session — exit
without touching anything). Compute the queue (manifest rows with no
target file on disk; unit number = manifest file line; skip blocked
lines 23/24). Ship the next 1-2 units through the full pipeline: ONE
implementer agent, then reviewers A (semantics) + B (obedience)
dispatched together in the FOREGROUND (idle-time sessions have no
background agents), then verify every finding's evidence yourself
before applying (refute wrong ones — reviewers' cites have been wrong
before; purely mechanical finding sets may be fixed inline), then
commit each unit with the established message format (unit <line>/386,
review verdicts, fixes applied AND refuted, marker counts). Afterwards
refresh HANDOFF.md's STATUS block, update the persistent memory files
(glean-rust-migration-status.md + the MEMORY.md index line) per the
session's memory instructions, append one cost-log.tsv row, and commit
the record updates. Hard rails: no cargo build/check/test/run; no git
push; touch only rust/** and migration/**; never ask the user questions
(deferrals → DECISIONS.md as coordinator-default rulings); dispatch
agents foreground only; do NOT create or configure any further
idle-time tasks or automations (platform rule — the scheduled
supervisor owns replenishment); stop cleanly at 1-2 units or on quota
exhaustion. Final message: units shipped (or why none), commits made,
new queue head."

If OffPeakCreate fails (e.g. free-quota exhausted), state the exact
error in your report and do not retry this fire.

Step 3 — heartbeat (ONLY under persistent stall): if no idle task is
running AND the newest migration commit on the branch is older than 8
hours, ship ONE unit yourself per Glean/migration/RUNBOOK.md (full
pipeline, foreground agents, commit + records refresh). Otherwise do
no migration work.

Rails: touch only rust/** and migration/**; no cargo build/check/test/
run; no git push; never ask the user questions; never create or modify
automations; create idle-time tasks only as Step 2 prescribes. Final
message (concise): queue state seen, task created or why not, any
OffPeakCreate error verbatim, heartbeat action if any, newest commit +
migration queue head.
```

## Operating notes

- Two idle tasks may briefly coexist (running + pre-staged). If the
  server grants both concurrently, RUNBOOK Step 0's seconds-fresh
  triage makes the loser exit untouched — at worst one free ticket is
  spent, never the repo.
- Idle-time starts are NOT guaranteed (server grants off-peak compute
  when available). The heartbeat (Step 3) bounds silent stalls to one
  plan-quota unit per fire after 8h of no commits; every fire's report
  shows queue state, so persistent stalls are visible.
- The user also holds a paused idle task ("m2") as a manual spare;
  supervisors ignore paused tasks.
