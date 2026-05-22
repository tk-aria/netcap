# Repository Guide

This repository hosts Claude Code plugins distributed via the
`research-plugin-registry` marketplace. Each plugin lives under
`plugins/<plugin-name>/` and follows the standard Claude Code plugin layout.

## Knowledge Accumulation Rule

When a discussion with the user produces durable insight — design decisions,
trade-off analyses, test results, rejected alternatives, or lessons learned —
that knowledge must be written into the relevant plugin's `docs/` directory
so it survives future sessions.

### When to write

Write a doc when any of the following apply:

- A non-obvious design decision was made and the reasoning matters
- Multiple alternatives were evaluated and rejected ones deserve a record
- Empirical tests were run (token counts, compliance tests, benchmarks)
- A recurring problem was diagnosed and a strategy was adopted
- A trade-off was consciously accepted (cost vs. compliance, speed vs. accuracy)

Do NOT write a doc for:

- Transient task state (use TodoWrite / plans)
- Code changes self-evident from git history
- Routine bug fixes with no broader implication
- User's personal preferences (those belong in `~/.claude/` memory)

### Where to write

Place docs under the affected plugin:

```
plugins/<plugin-name>/docs/<topic>.md
```

If the insight spans multiple plugins or the repository itself, place it
at the repository root:

```
docs/<topic>.md
```

### How to write

Each doc should include, at minimum:

1. **Motivation** — what problem triggered the investigation
2. **What was tested / considered** — alternatives evaluated
3. **Findings** — what actually happened, with numbers when available
4. **Adopted strategy** — the decision and why
5. **What was NOT adopted** — rejected alternatives with reasons
6. **Open items** — follow-ups, remaining risks, future work

Keep docs durable: favor claims that stay true over time. When something
changes, update or supersede the doc rather than leaving stale content.

## Plugins

- **research-tools** (`plugins/research-tools/`) — Multi-source web research
  plugin with skills for restaurant discovery, venue comparison,
  cross-site fact-checking, Wantedly developer recruitment research,
  X(Twitter) developer recruitment research, high-salary job search,
  and comprehensive 12-dimension company analysis
  (skills: restaurant-finder, wantedly-dev-research, x-dev-research, highclass-dev-recruitment, company-analysis)

## Plugin Version Management

When modifying any file under `plugins/<plugin-name>/` (including
hooks, skills, agents, commands, or the manifest itself), you **must**
bump the `version` field in `.claude-plugin/plugin.json` before
committing.

Claude Code caches plugins by version. Without a bump, changes will
not be picked up by users who already have the plugin installed.

- Use semver: patch for fixes, minor for new features, major for
  breaking changes.
- The canonical manifest is `.claude-plugin/plugin.json`.
