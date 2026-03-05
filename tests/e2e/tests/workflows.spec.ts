import { test, expect } from '../fixtures/test-fixtures';

test.describe('Workflow API', () => {
  let projectId: string;

  test.beforeEach(async ({ apiClient }) => {
    const project = await apiClient.createProject({ name: 'Workflow Test Project' });
    projectId = project.id;
  });

  test('built-in Feat-Common workflow is seeded on first list', async ({ apiClient }) => {
    const workflows = await apiClient.listWorkflowsByProject(projectId);
    expect(workflows.length).toBeGreaterThanOrEqual(1);

    const featCommon = workflows.find(w => w.name === 'Feat-Common');
    expect(featCommon).toBeDefined();
    expect(featCommon!.project_id).toBe(projectId);
    expect(featCommon!.definition.steps.length).toBe(7);

    // Verify step types match the expected sequence
    const stepTypes = featCommon!.definition.steps.map(s => s.step_type);
    expect(stepTypes).toEqual([
      'plan', 'human_gate', 'implement', 'pr_review', 'implement', 'human_review', 'implement',
    ]);
  });

  test('built-in workflow is idempotent', async ({ apiClient }) => {
    // List twice — should not create duplicates
    await apiClient.listWorkflowsByProject(projectId);
    const workflows = await apiClient.listWorkflowsByProject(projectId);
    const featCommonCount = workflows.filter(w => w.name === 'Feat-Common').length;
    expect(featCommonCount).toBe(1);
  });

  test('create custom workflow', async ({ apiClient }) => {
    const workflow = await apiClient.createWorkflow({
      project_id: projectId,
      name: 'Quick Fix',
      definition: {
        steps: [
          { step_type: 'implement', name: 'Fix & PR' },
          { step_type: 'human_review', name: 'Review' },
        ],
      },
    });
    expect(workflow.name).toBe('Quick Fix');
    expect(workflow.definition.steps.length).toBe(2);

    // Appears in project workflow list
    const workflows = await apiClient.listWorkflowsByProject(projectId);
    expect(workflows.some(w => w.name === 'Quick Fix')).toBe(true);
  });

  test('delete custom workflow', async ({ apiClient }) => {
    const workflow = await apiClient.createWorkflow({
      project_id: projectId,
      name: 'Temp',
      definition: { steps: [{ step_type: 'plan', name: 'Plan' }] },
    });

    await apiClient.deleteWorkflow(workflow.id);

    const workflows = await apiClient.listWorkflowsByProject(projectId);
    expect(workflows.some(w => w.id === workflow.id)).toBe(false);
  });
});

test.describe('Workflow Run Lifecycle', () => {
  let projectId: string;
  let agentId: string;
  let repoPath: string;

  test.beforeEach(async ({ apiClient }) => {
    // Set up project with a repo
    const project = await apiClient.createProject({ name: 'Run Test Project' });
    projectId = project.id;

    // Use the composer repo itself as the test repo
    repoPath = 'Q:/src/composer';
    await apiClient.addProjectRepository(projectId, {
      local_path: repoPath,
      role: 'primary',
    });

    // Create agent
    const agent = await apiClient.createAgent({ name: 'Workflow Test Agent' });
    agentId = agent.id;
  });

  test('start-workflow requires backlog task', async ({ apiClient }) => {
    // Seed the built-in workflow
    const workflows = await apiClient.listWorkflowsByProject(projectId);
    const featCommon = workflows.find(w => w.name === 'Feat-Common')!;

    // Create task in in_progress
    const task = await apiClient.createTask({
      title: 'Not Backlog',
      status: 'in_progress',
      project_id: projectId,
      assigned_agent_id: agentId,
    });

    // Should fail
    await expect(
      apiClient.startWorkflow(task.id, featCommon.id),
    ).rejects.toThrow(/400|backlog/i);
  });

  test('start-workflow requires assigned agent', async ({ apiClient }) => {
    const workflows = await apiClient.listWorkflowsByProject(projectId);
    const featCommon = workflows.find(w => w.name === 'Feat-Common')!;

    const task = await apiClient.createTask({
      title: 'No Agent',
      project_id: projectId,
      // No assigned_agent_id
    });

    await expect(
      apiClient.startWorkflow(task.id, featCommon.id),
    ).rejects.toThrow(/400|agent/i);
  });

  test('start-workflow requires project', async ({ apiClient }) => {
    const workflows = await apiClient.listWorkflowsByProject(projectId);
    const featCommon = workflows.find(w => w.name === 'Feat-Common')!;

    const task = await apiClient.createTask({
      title: 'No Project',
      assigned_agent_id: agentId,
      // No project_id
    });

    await expect(
      apiClient.startWorkflow(task.id, featCommon.id),
    ).rejects.toThrow(/400|project/i);
  });

  test('submit decision on non-paused run fails', async ({ apiClient }) => {
    const workflows = await apiClient.listWorkflowsByProject(projectId);
    const featCommon = workflows.find(w => w.name === 'Feat-Common')!;

    const task = await apiClient.createTask({
      title: 'Decision Test',
      project_id: projectId,
      assigned_agent_id: agentId,
    });

    // Start workflow — this will try to spawn a real agent which may fail,
    // but the workflow run should be created
    let runId: string;
    try {
      const run = await apiClient.startWorkflow(task.id, featCommon.id);
      runId = run.id;
    } catch {
      // If agent spawn fails, the workflow run may still exist
      // Check the task for a workflow_run_id
      const updatedTask = await apiClient.getTask(task.id);
      if (!updatedTask.workflow_run_id) {
        // Agent spawn failed before workflow run was created — skip test
        test.skip();
        return;
      }
      runId = updatedTask.workflow_run_id;
    }

    // The run is in 'running' or 'failed' — either way, not 'paused'
    const run = await apiClient.getWorkflowRun(runId);
    if (run.status !== 'paused') {
      await expect(
        apiClient.submitWorkflowDecision(runId, true),
      ).rejects.toThrow(/400|500|not paused/i);
    }
  });
});

test.describe('Workflow UI', () => {
  test.beforeEach(async ({ apiClient, page }) => {
    const project = await apiClient.createProject({ name: 'UI Workflow Project' });
    await apiClient.addProjectRepository(project.id, {
      local_path: 'Q:/src/composer',
      role: 'primary',
    });
    const agent = await apiClient.createAgent({ name: 'UI Agent' });

    await apiClient.createTask({
      title: 'Workflow UI Test Task',
      project_id: project.id,
      assigned_agent_id: agent.id,
    });

    await page.goto('/');
  });

  test('workflow dropdown appears for tasks with project', async ({ page }) => {
    // Click on the task to open detail panel
    await page.getByText('Workflow UI Test Task').click();

    // The workflow dropdown should be visible in the sessions section
    // (only when task is in backlog and has a project with workflows)
    const workflowSelect = page.locator('select').filter({ has: page.locator('option', { hasText: 'No workflow' }) });
    await expect(workflowSelect).toBeVisible();
  });

  test('workflow dropdown shows Feat-Common', async ({ page }) => {
    await page.getByText('Workflow UI Test Task').click();

    // Look for Feat-Common in any select dropdown
    const featCommonOption = page.locator('option', { hasText: 'Feat-Common' });
    await expect(featCommonOption).toBeAttached();
  });

  test('start workflow button appears when workflow selected', async ({ page }) => {
    await page.getByText('Workflow UI Test Task').click();

    // Select the Feat-Common workflow
    const workflowSelect = page.locator('select').filter({ has: page.locator('option', { hasText: 'No workflow' }) });
    await workflowSelect.selectOption({ label: 'Feat-Common' });

    // Start Workflow button should appear
    await expect(page.getByRole('button', { name: /start workflow/i })).toBeVisible();

    // Regular Start button should be hidden when workflow is selected
    await expect(page.getByRole('button', { name: /^start$/i })).not.toBeVisible();
  });
});
