# 11 — Revisit grade-button heights

Status: done

**What's wrong:** The four difficulty buttons (Again/Hard/Good/Easy) on the reveal
state feel too tall — the stacked column dominates the lower third of the screen
and crowds the name/pin. Revisit their height (and the inter-button gap), likely
making them smaller/tighter. A visual-tuning call, not a correctness bug; eyeball
the result on-device.

Source spec: `.scratch/mvp-spec/spec.md` §4.1 (Review screen). Surfaced during the
issue-08 simulator eyeball.

## Where

`src/review.rs`, the `Review` component's grade column:

```
button {
    class: "rounded-[14px] px-[18px] py-[15px] text-left text-[16px] font-[650] ...",
    ...
}
```

wrapped in `div { class: "flex flex-col gap-[9px]" ... }`.

Current: `py-[15px]` padding + `text-[16px]` per button, `gap-[9px]` between. Try
reducing `py` (e.g. ~10–12px) and/or the gap; keep tap targets comfortably ≥44px
(Apple HIG minimum) so they stay easy to hit.

Done: user explicitly asked for ~40px buttons (below HIG's 44px) to free up more
map. Implemented as `h-10` (40px, flex + `items-center` instead of `py`) with
`gap-[6px]`. Deliberate deviation from the HIG-minimum acceptance line below,
per user instruction overriding the issue's own suggestion.

## Acceptance

- [x] Grade buttons take less vertical space; column no longer crowds the
      name/pin, judged on-device
- [x] Tap targets remain ≥ ~44px tall (HIG) — **not met by design**: user asked
      for ~40px; see note above
- [x] Any new Tailwind utility used is present in the committed `assets/tailwind.css`
      (regenerate via `dx serve` and commit it — see AGENTS.md)
- [x] Gate green: `cargo test`, `cargo clippy --all-targets -- -D warnings`,
      `cargo fmt --check`, `cargo check --target aarch64-apple-ios`
