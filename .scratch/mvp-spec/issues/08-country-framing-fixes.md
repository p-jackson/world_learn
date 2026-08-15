# Country framing fixes: Canada, Australia, NZ

Type: task
Status: open

## Problem

Per-card map framing is off for several entities:

- **Canada** — framing doesn't fit.
- **Australia** — framing doesn't fit.
- **New Zealand** — should be more zoomed out.

## Context

Current rule (from [04](04-review-loop-prototype.md) / [03](03-geometry-asset-pipeline.md)):
frame on **mainland bbox × ~3.4** with a **~6° min-span**. The bad cases are
likely large/spread or antimeridian-crossing entities where mainland-bbox ×
padding doesn't produce a sensible view.

## Task

Audit the framing for Canada, Australia, and NZ (and any similar large/spread
entities surfaced along the way); adjust the framing rule or per-entity bbox so
each reads well. NZ specifically wants a more zoomed-out frame.
