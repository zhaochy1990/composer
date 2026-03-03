# Composer — Prototype Implementation Plan

## Context

Building "Composer", an AI agent orchestration platform. Rust backend + React frontend + single binary. This plan covers the Prototype phase: tasks + agents + worktree isolation + task board.

Full tech design: `Q:\src\composer\tech_design.md`
Product design: `Q:\src\composer\FHL project.md`

## Task Breakdown & Dependencies

```
Wave 1 (parallel — start 3 CC sessions):
  T1: api-types        ─┬─→ T4: db         ─┬─→ T6: services ──→ T7: server ─┬─→ T8:  Task Board UI  ─┐
  T2: git (standalone)  ─┘   T5: executors  ─┘                                ├─→ T9:  Agent Pool UI  ─┼─→ T11: Integration
  T3: web scaffold     ─────────────────────────────────────────────────────────┴─→ T10: Session UI     ─┘
```

### Wave 1: No dependencies (3 parallel sessions)

| Task | Description | Blocked by |
|------|-------------|------------|
| **T1** | Create `composer-api-types` crate — shared types, enums, request/response structs, ts-rs codegen. Also setup workspace root (Cargo.toml, package.json, pnpm-workspace.yaml). | None |
| **T2** | Create `composer-git` crate — git worktree create/remove/list via CLI commands. | None |
| **T3** | Scaffold `packages/web` — Vite + React + TanStack Router/Query + Zustand + Tailwind + shadcn. Placeholder pages, hooks, stores, WebSocket client. | None |

### Wave 2: Depends on T1 (2 parallel sessions)

| Task | Description | Blocked by |
|------|-------------|------------|
| **T4** | Create `composer-db` crate — SQLite schema (5 tables), migrations, CRUD repos for agents/tasks/sessions/worktrees/logs. | T1 |
| **T5** | Create `composer-executors` crate — Claude Code protocol types, process manager (spawn/interrupt/cleanup), agent discovery, auto-yes. **Highest risk component.** | T1 |

### Wave 3: Depends on T2 + T4 + T5

| Task | Description | Blocked by |
|------|-------------|------------|
| **T6** | Create `composer-services` crate — TaskService, AgentService, SessionService (orchestration), WorktreeService (with locking), EventBus. | T2, T4, T5 |

### Wave 4: Depends on T6

| Task | Description | Blocked by |
|------|-------------|------------|
| **T7** | Create `composer-server` crate — Axum routes, WebSocket hub, rust-embed frontend serving, server entry point. | T6 |

### Wave 5: Depends on T3 + T7 (3 parallel sessions)

| Task | Description | Blocked by |
|------|-------------|------------|
| **T8** | Build Task Board UI — Kanban board, task CRUD, drag-and-drop, assign agent. | T3, T7 |
| **T9** | Build Agent Pool UI — Agent cards, register, discover, health badges. | T3, T7 |
| **T10** | Build Session + Live Dashboard UI — Session list, live output streaming, interrupt/resume controls. | T3, T7 |

### Wave 6: Final integration

| Task | Description | Blocked by |
|------|-------------|------------|
| **T11** | Integration + E2E — Task↔Agent binding, auto-yes E2E, full flow test, production build test. | T8, T9, T10 |

## Max Parallelism Schedule

| Time | Session A | Session B | Session C |
|------|-----------|-----------|-----------|
| Wave 1 | T1: api-types | T2: git | T3: web scaffold |
| Wave 2 | T4: db | T5: executors | (idle or help T4/T5) |
| Wave 3 | T6: services | | |
| Wave 4 | T7: server | | |
| Wave 5 | T8: Task Board | T9: Agent Pool | T10: Session UI |
| Wave 6 | T11: Integration | | |

## Tech Stack Summary

- **Backend**: Rust — Axum, SQLx/SQLite, tokio, rust-embed
- **Frontend**: React 18, TypeScript, TanStack Router/Query, Zustand, Tailwind, shadcn/ui
- **Build**: pnpm workspace + Cargo workspace → single binary
- **Type safety**: ts-rs generates TypeScript from Rust structs
- **Agent comms**: Claude Code CLI subprocess, stdin/stdout stream-json protocol

## Verification

1. `pnpm run dev` → opens http://localhost:5173
2. Discover agents → Claude Code appears
3. Create task → appears in Backlog
4. Assign to agent → spawns in worktree, task moves to In Progress
5. Watch live output on Session Detail
6. Agent completes → task moves to Done
7. `pnpm run build` → single binary works standalone
