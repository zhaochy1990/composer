import { test, expect } from '../fixtures/test-fixtures';

// TODO: Replace with your local repo path when running E2E tests on a different machine.
const TEST_REPO_PATH = 'Q:/src/composer';

test.describe('Workflow API', () => {
  test('built-in Feat-Common workflow exists after server start', async ({ apiClient }) => {
    const workflows = await apiClient.listWorkflows();
    expect(workflows.length).toBeGreaterThanOrEqual(1);

    const featCommon = workflows.find(w => w.name === 'Feat-Common');
    expect(featCommon).toBeDefined();
    expect(featCommon!.definition.steps.length).toBe(7);

    const stepTypes = featCommon!.definition.steps.map(s => s.step_type);
    expect(stepTypes).toEqual([
      'plan', 'human_gate', 'implement', 'pr_review', 'implement', 'human_review', 'implement',
    ]);
  });

  test('built-in workflow is not duplicated on repeated list calls', async ({ apiClient }) => {
    await apiClient.listWorkflows();
    const workflows = await apiClient.listWorkflows();
    const featCommonCount = workflows.filter(w => w.name === 'Feat-Common').length;
    expect(featCommonCount).toBe(1);
  });

  test('create custom workflow', async ({ apiClient }) => {
    const workflow = await apiClient.createWorkflow({
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

    const workflows = await apiClient.listWorkflows();
    expect(workflows.some(w => w.name === 'Quick Fix')).toBe(true);
  });

  test('delete custom workflow', async ({ apiClient }) => {
    const workflow = await apiClient.createWorkflow({
      name: 'Temp',
      definition: { steps: [{ step_type: 'plan', name: 'Plan' }] },
    });

    await apiClient.deleteWorkflow(workflow.id);

    const workflows = await apiClient.listWorkflows();
    expect(workflows.some(w => w.id === workflow.id)).toBe(false);
  });
});

test.describe('Workflow Run Lifecycle', () => {
  let projectId: string;
  let agentId: string;

  test.beforeEach(async ({ apiClient }) => {
    const project = await apiClient.createProject({ name: 'Run Test Project' });
    projectId = project.id;
    await apiClient.addProjectRepository(projectId, {
      local_path: TEST_REPO_PATH,
      role: 'primary',
    });
    const agent = await apiClient.createAgent({ name: 'Workflow Test Agent' });
    agentId = agent.id;
  });

  test('start-workflow requires backlog task', async ({ apiClient }) => {
    const workflows = await apiClient.listWorkflows();
    const featCommon = workflows.find(w => w.name === 'Feat-Common')!;

    const task = await apiClient.createTask({
      title: 'Not Backlog',
      status: 'in_progress',
      project_id: projectId,
      assigned_agent_id: agentId,
    });

    await expect(
      apiClient.startWorkflow(task.id, featCommon.id),
    ).rejects.toThrow(/400|backlog/i);
  });

  test('start-workflow requires assigned agent', async ({ apiClient }) => {
    const workflows = await apiClient.listWorkflows();
    const featCommon = workflows.find(w => w.name === 'Feat-Common')!;

    const task = await apiClient.createTask({
      title: 'No Agent',
      project_id: projectId,
    });

    await expect(
      apiClient.startWorkflow(task.id, featCommon.id),
    ).rejects.toThrow(/400|agent/i);
  });

  test('start-workflow requires project', async ({ apiClient }) => {
    const workflows = await apiClient.listWorkflows();
    const featCommon = workflows.find(w => w.name === 'Feat-Common')!;

    const task = await apiClient.createTask({
      title: 'No Project',
      assigned_agent_id: agentId,
    });

    await expect(
      apiClient.startWorkflow(task.id, featCommon.id),
    ).rejects.toThrow(/400|project/i);
  });
});

test.describe('Workflow UI', () => {
  test.beforeEach(async ({ apiClient, page }) => {
    const project = await apiClient.createProject({ name: 'UI Workflow Project' });
    await apiClient.addProjectRepository(project.id, {
      local_path: TEST_REPO_PATH,
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

  test('workflows page is accessible from sidebar', async ({ page }) => {
    await page.getByRole('button', { name: 'Workflows' }).click();
    await expect(page.getByRole('heading', { name: 'Workflows' })).toBeVisible();
  });

  test('workflows page shows Feat-Common', async ({ page }) => {
    await page.getByRole('button', { name: 'Workflows' }).click();
    await expect(page.getByText('Feat-Common')).toBeVisible();
  });

  test('workflow dropdown appears in task detail', async ({ page }) => {
    await page.getByText('Workflow UI Test Task').click();
    const workflowSelect = page.locator('select').filter({ has: page.locator('option', { hasText: 'No workflow' }) });
    await expect(workflowSelect).toBeVisible();
  });

  test('workflow dropdown shows Feat-Common in task detail', async ({ page }) => {
    await page.getByText('Workflow UI Test Task').click();
    const featCommonOption = page.locator('option', { hasText: 'Feat-Common' });
    await expect(featCommonOption).toBeAttached();
  });

  test('start workflow button appears when workflow selected', async ({ page }) => {
    await page.getByText('Workflow UI Test Task').click();
    const workflowSelect = page.locator('select').filter({ has: page.locator('option', { hasText: 'No workflow' }) });
    await workflowSelect.selectOption({ label: 'Feat-Common' });
    await expect(page.getByRole('button', { name: /start workflow/i })).toBeVisible();
  });
});
