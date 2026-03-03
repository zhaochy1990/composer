# Product Vision

One-liner: An AI agent orchestration platform that turns work items into parallelized, workflow-driven coding tasks — config once, run on any agent.

# Design Principles

1. **Do not build new coding agents, orchestration only.**
2. **Config once, use anywhere.**
   - One MCP config → adapts to Claude Code/Codex/Github Copilot...
   - One knowledge base → injected into any agent session
   - One workflow → executable by any agent type
3. **Enterprise-first, not enterprise-bolted-on.**
   - Auth, audit trail, cost tracking from day one.
   - Respect existing DevOps toolchain (Azure DevOps, Github Copilot, QuickBuild).
4. **Human-in-the-loop by default, autonomous by opt-in.**
   - Agents propose, humans approve. Escalation, not automation.
5. **Observable, not magical.**
   - Every agent action is logged, reviewable, and replayable.
6. **Distributed-first, local-friendly.**
   - Orchestrate agents across multiple DevBoxes for true parallelism.
   - At each devbox, use local worktrees
   - The control plane is lightweight — it coordinates, it doesn't execute.

# Feature Map

## F1. Project & Repository Management

> As a user, I can set up repositories for my project. A project may include multiple repositories as upstream/downstream services, so that AI agents can have the ability to get a complete dependency analysis.

| Feature | Name | Description |
|---------|------|-------------|
| F1.1 | Multi-repo project | A project links to N repos (e.g., Substrate + MgmtApi) |
| F1.2 | Branch strategy | Define branch conventions per project |
| F1.3 | Component hints | Define primary components for a project (e.g., `sources/dev/Management`, `sources/dev/common`). Agents start their search here but are NOT restricted from reaching outside. |

## F2. Agent Pool & Execution

> As a user, I want to create multiple agents and let them work in parallel.

| Feature | Name | Description |
|---------|------|-------------|
| F2.1 | Agent pool | Register available agents (Claude Code, Gemini CLI, Codex, etc.). Auto-discover installed agents on the machine. Show health/auth status. |
| F2.2 | Parallel execution | Launch N agents simultaneously, each working on an independent task. Visual dashboard shows real-time status (idle / working / waiting-for-approval / done / failed). |
| F2.3 | Agent assignment strategy | manual — user picks agent per task. round-robin — distribute evenly. best-fit — match task complexity to agent capability (e.g., Opus for architecture, Haiku for simple fixes). |
| F2.4 | Isolation | Each agent works in its own git worktree. No two agents touch the same files. Conflict detection if scopes overlap. |
| F2.5 | Session management | Persist agent sessions. Support resume, fork. |

## F3. Workflow Engine

> As a user, I want to define workflows for my project, so that all agents follow the same pattern & steps to collaborate together.

This is the most differentiating feature. Think of it as GitHub Actions / ADO build pipelines for AI agents.

| Feature | Name | Description |
|---------|------|-------------|
| F3.1 | Workflow definition | Define reusable workflow. Example below. |
| F3.2 | DAG execution | Steps can run in parallel or sequential. Define dependencies: step B waits for step A. Visualize DAG in UI. |
| F3.3 | Shared context | Steps can pass outputs to downstream steps (e.g., step 1 design the feature and generate the design doc, step 2 implement it and create a PR, step 3 review the PR and outputs a list of findings). |
| F3.4 | Human gates | Insert human review/approval points anywhere in the workflow. Agent pauses until human acts. |
| F3.5 | Workflow library | Pre-built workflows for common patterns (see examples below). |
| F3.6 | Error handling | Per-step `on-failure` policy. Options: `retry(N)` — retry up to N times, `escalate` — pause and notify human, `abort` — stop workflow, `skip` — skip and continue. When retries exhausted, `on-max-retries` defines the fallback. Default behavior is `escalate` (aligns with human-in-the-loop principle). |

### Example workflow definition

```yaml
name: "Feature Development"
trigger: manual | workitem-assigned
steps:
  - id: plan
    type: agent-task
    agent: claude-code/opus
    prompt-template: feature-plan
    inputs:
      workitem: "{{trigger.workitem_id}}"
    gate: approval  # human must approve the plan

  - id: implement
    type: agent-task
    agent: claude-code/opus
    prompt-template: implement-from-plan
    inputs:
      plan: "{{steps.plan.output}}"
    depends-on: [plan]
    isolation: worktree
    on-failure: retry(3)
    on-max-retries: escalate

  - id: create-pr
    type: pr
    title: "{{steps.plan.output.title}}"
    target: master
    depends-on: [implement]

  - id: monitor-pr
    type: loop
    agent: claude-code/sonnet
    depends-on: [create-pr]
    timeout: 1d
    exit-when: pr-checks-pass AND pr-comments-resolved
    on-timeout: escalate
    steps:
      - id: wait-for-feedback
        type: agent-task
        prompt-template: get-pr-feedback
      - id: wait-for-checks
        type: agent-task
        prompt-template: get-pr-checks
      - id: fix-issues
        type: agent-task
        prompt-template: fix-pr-feedback
        on-failure: retry(3)
        on-max-retries: escalate
        gate: review   # human reviews the comment fix
```

### Workflow: project-level config, task-level selection

```
Project "Substrate"
├── Workflows (project-level config):
│   ├── "Bug Fix"            ← recipe: analyze → fix → build → test → PR
│   ├── "Feature Dev"        ← recipe: plan → approve → implement → PR → monitor
│   ├── "Refactoring"        ← recipe: analyze scope → migrate in stages → PR
│   └── "Bond Schema Change" ← recipe: edit bond → regen → update consumers → PR
│
├── Task 1: "Fix login bug"
│   └── workflow: "Bug Fix"              ← user picks at task creation
│
├── Task 2: "Add retry logic to transport"
│   └── workflow: "Feature Dev"          ← user picks at task creation
│
└── Task 3: "Rename FooService to BarService"
    └── workflow: "Refactoring"          ← user picks at task creation
```

Default workflow can be auto-selected based on work item type. User can override.

```
default_workflows:
  Bug:        "Bug Fix"
  UserStory:  "Feature Dev"
  Task:       "Feature Dev"
```

```
┌───────────────────────────────────────────────────────────┐
│ User creates a Task:                                      │
│   title: "Fix login failure after password reset"         │
│   description: "Users report 500 error when..."           │
│   workflow: "Bug Fix"  (pick from project's workflows)    │
└──────────────────────┬────────────────────────────────────┘
                       │
                       ▼
┌───────────────────────────────────────────────────────────┐
│ System applies "Bug Fix" workflow to this task:           │
│                                                           │
│   Step 1: analyze    → agent reads description, finds root│
│   Step 2: implement  → agent writes the fix               │
│   Step 3: build      → run build command                  │
│   Step 4: test       → run tests                          │
│   Step 5: create-pr  → push & create PR                   │
│   Step 6: monitor-pr → loop until merged                  │
└───────────────────────────────────────────────────────────┘
```

## F4. Knowledge Management

> As a user, I want to have the ability to config reusable knowledge for the agents.

| Feature | Name | Description |
|---------|------|-------------|
| F4.1 | Knowledge layers | 2 layers, merged top-down: global (org-wide) → project (per-project) |
| F4.2 | Knowledge types | See table below |
| F4.3 | Knowledge injection | Knowledge is auto injected into agent context |

### Knowledge Types

| Type | Example | Injection method |
|------|---------|------------------|
| Coding standards | Coding conventions and style rules | Prepend to the prompt |
| Build recipes | e.g., In Substrate repo, `.github\instructions\Building.instructions.md` | Reuse that |
| Prompt templates | When fixing a build error, first read the error log, then … | Reusable prompt library, Prepend to the prompt |
| Instruction files | Existing instruction files in the repo | Auto-discovered |
| Forbidden patterns | Never use `dotnet build`. Never add `AutoGenerateBindingRedirects`. | Prepend to the prompt |
| Architecture docs | Component dependency diagrams, data flow descriptions | RAG or context attachment |
| Review checklists | "Before PR: StyleCop clean, no CPM violations, binding redirects ok" | Injected at review step |

## F5. Task Management

Short term solution, use a light weight database, e.g. SQLite, to build the prototype.

Long term goal, we do not manage the tasks ourselves, we integrate with ADO to meet developers where they already work.

| Feature | Name | Description |
|---------|------|-------------|
| F5.1 | Task management | Task CRUD |
| F5.2 | Task ↔ Agent binding | Assign a task to an agent. Agent receives task context (title, description, acceptance criteria) as part of its prompt. When agent completes or needs human input, task auto-transitions to "Waiting" or "Done". |
| F5.3 | Task ↔ PR linking | When an agent creates a PR, the task stores the PR URL. Show PR status (draft/active/completed) on the task card. |
| F5.4 | Task → Workflow | User creates a task with a description of what they want. System applies a selected workflow to break it down into agent execution steps. Task status reflects overall progress of the workflow run. |
| F5.5 | Task board UI | Kanban board with columns: Backlog → In Progress → Waiting → Done. Filter by project, agent, priority. |
| F5.6 | Task linking | Link tasks as "follow-up". Follow-up links auto-inherit agent type and inject context from the linked task. System attempts session resume for same-agent links. |
| F5.7 | Workflow Run Status | Each task follows workflow steps, we can see the status of the current workflow run. Each task has 4 statuses, and another property "Workflow Run Status" is used to indicate the workflow status of the task. |
| F5.8 | Auto-generated summary | When a task transitions to Done, the system auto-generates a structured summary from existing context (plan, files changed, PR description, PR URL). This summary is the data source for Level 1 context injection into follow-up tasks. |

### Task Status

| State | Meaning |
|-------|---------|
| **Backlog** | Not started yet |
| **In Progress** | Agent(s) actively working |
| **Waiting** | Needs human action (approve, review, input, resolve conflict — anything) |
| **Done** | Complete |

```
Backlog ──→ In Progress ──→ Done
                ↕
             Waiting
```

**Status derivation from workflow steps** (priority order, highest wins):

```
if   no workflow run started             → Backlog
if   any step needs human action         → Waiting
elif any step is running                 → In Progress
if   all steps done                      → Done
```

### Task Context Management

Use the example below to illustrate the problem:

```
Task #1: "Implement OAuth login"     (3 weeks ago, Done)
  └─ Agent session: 200+ messages, full design context, file knowledge

Task #2: "Fix: OAuth token not refreshing"    (today, New)
  └─ Link to: Task #1
  └─ Agent starts from scratch? Or picks up #1's context?
```

Without linking, the agent on Task #2 wastes 10-20 minutes rediscovering what was built, which files matter, what design decisions were made. With linking, it already knows.

#### Two Levels of Context Propagation

Session resumption sounds ideal but doesn't always work. You need two levels:

**Level 1: Context injection (always works)**
- Inject a structured summary from Task #1 into Task #2's prompt
- Works across different agent types (Task #1 was Claude, Task #2 is Copilot)
- Works even if the original session expired

**Level 2: Session resumption (best case)**
- Actually resume the Claude Code session from Task #1
- Agent has full memory of every file it read, every decision it made
- Only works if: same agent type AND session still exists

We will start from Level 2. When user creates Task #2:
- Links as follow-up to Task #1
- System auto:
  1. Inherits agent type from Task #1 (Claude Code/Opus)
  2. Injects Level 1 context (structured summary)
  3. Attempts Level 2 session resume (same agent, best effort)

The user doesn't need to pick an agent or worry about context — the link does both.

## F6. MCP Configuration Center

| Feature | Name | Description |
|---------|------|-------------|
| F6.1 | Centralized MCP config | Define MCP servers once, use anywhere |
| F6.2 | Agent-specific adapter | Auto-convert MCP config format per agent type |
| F6.3 | Per-project MCP scope | Different projects need different tools |

## F7. Observability

| Feature | Name | Description |
|---------|------|-------------|
| F7.1 | Live dashboard | Real-time view of all agents, status, current step in workflow. |

## F8. Approval & Safety

| Feature | Name | Description |
|---------|------|-------------|
| F8.1 | Gate: Review | Force review and approve for critical steps, e.g., the plan, the PR |
| F8.2 | Policy engine | Define rules: "auto-approve file reads", "require approval for file deletes", "block `rm -rf *`". Claude Code already has this; we only make the configuration reusable. |
| F8.3 | Auto-yes | When Claude Code asks for some permission, auto select yes. |

## F9. DevBox Management (Future)

Full repo clones — No worktree limitations. Each agent gets a full enlistment.
Use a new DevBox best for risky experiments, major refactors.

```
┌──────────────┐  ┌──────────────┐  ┌──────────────┐
│   DevBox A   │  │   DevBox B   │  │   DevBox C   │
│              │  │              │  │              │
│ Agent1       │  │ Agent3       │  │ Agent5       │
│ Agent2       │  │ Agent4       │  │ Agent6       │
│              │  │              │  │              │
│ full repo    │  │ full repo    │  │ full repo    │
│ build cache  │  │ build cache  │  │ build cache  │
└──────┬───────┘  └──────┬───────┘  └──────┬───────┘
       │                 │                 │
       └────────┬────────┘                 │
                │    ┌─────────────────────┘
       ┌────────▼────▼───────┐
       │  Control Plane (UI) │
       │  your local machine │
       └─────────────────────┘
```

| Feature | Name | Description |
|---------|------|-------------|
| F9.1 | DevBox registry | Auto-discover from Microsoft Dev Box service via API. |
| F9.2 | DevBox health probe | Ensure required agent tool, git, Azure CLI, GitHub CLI etc. are successfully installed and configured at DevBox. |
| F9.3 | Distributed Coordination | M agents run on N DevBoxes, merge the changes, sync files cross-DevBox, consolidate PR, etc. |

# Roadmap

**Prototype**: Core loop — tasks + agents + worktree isolation + task board.

**FHL**: Workflow engine + knowledge injection + task linking.

**Future**: ADO integration, MCP adapters, multi-DevBox, distributed execution.

| Phase | Scope | Features |
|---|---|---|
| **Prototype** | Core loop — tasks + agents + worktree isolation + task board | F2.1 Agent pool (Claude Code only) |
| | | F2.2 Parallel execution |
| | | F2.4 Worktree isolation |
| | | F2.5 Session management |
| | | F5.1 Task CRUD (SQLite) |
| | | F5.2 Task ↔ Agent binding |
| | | F5.5 Task board UI |
| | | F8.3 Auto-yes |
| **FHL** | Workflow engine + knowledge + project setup + task linking | F1.1 Multi-repo project |
| | | F1.2 Branch strategy |
| | | F1.3 Component hints |
| | | F3.1 Workflow definition |
| | | F3.2 DAG execution |
| | | F3.3 Shared context |
| | | F3.4 Human gates |
| | | F3.6 Error handling |
| | | F4.1 Knowledge layers |
| | | F4.2 Knowledge types |
| | | F4.3 Knowledge injection |
| | | F5.3 Task ↔ PR linking |
| | | F5.4 Task → Workflow binding |
| | | F5.6 Task linking + context |
| | | F5.7 Workflow Run Status |
| | | F5.8 Auto-generated summary |
| | | F7.1 Live dashboard |
| | | F8.1 Gate: Review |
| **Future** | ADO integration, MCP adapters, multi-DevBox | F2.3 Agent assignment strategy |
| | | F3.5 Workflow library |
| | | F5 Phase 2 (ADO sync) |
| | | F6.1 Centralized MCP config |
| | | F6.2 Agent-specific adapter |
| | | F6.3 Per-project MCP scope |
| | | F8.2 Policy engine |
| | | F9.1 DevBox registry |
| | | F9.2 DevBox health probe |
| | | F9.3 Distributed Coordination |
