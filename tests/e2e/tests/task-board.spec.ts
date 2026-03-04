import { test, expect } from '../fixtures/test-fixtures';

test.describe('Task Board', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await expect(page.getByRole('heading', { name: 'Task Board' })).toBeVisible();
  });

  test('empty board shows 4 columns', async ({ page }) => {
    for (const column of ['Backlog', 'In Progress', 'Waiting', 'Done']) {
      await expect(page.getByText(column).first()).toBeVisible();
    }
  });

  test('create task via New Task button', async ({ page }) => {
    await page.getByRole('button', { name: /new task/i }).click();

    await page.locator('#create-title').fill('My E2E Task');
    await page.locator('#create-description').fill('Created by Playwright');
    await page.locator('#create-priority').selectOption('2'); // Medium

    await page.getByRole('button', { name: /create task/i }).click();

    // Task should appear on the board
    await expect(page.getByText('My E2E Task')).toBeVisible({ timeout: 5000 });
  });

  test('edit task title and status via detail panel', async ({ apiClient, page }) => {
    await apiClient.createTask({ title: 'Task To Edit', status: 'backlog' });
    await page.reload();

    await page.getByText('Task To Edit').click();
    await expect(page.getByText('Edit Task')).toBeVisible();

    await page.locator('#edit-title').clear();
    await page.locator('#edit-title').fill('Edited Task Title');
    await page.locator('#edit-status').selectOption('in_progress');

    await page.getByRole('button', { name: /save/i }).click();

    await expect(page.getByText('Edited Task Title')).toBeVisible({ timeout: 5000 });
  });

  test('delete task with confirmation', async ({ apiClient, page }) => {
    await apiClient.createTask({ title: 'Task To Delete', status: 'backlog' });
    await page.reload();

    await page.getByText('Task To Delete').click();
    await expect(page.getByText('Edit Task')).toBeVisible();

    await page.getByRole('button', { name: 'Delete', exact: true }).click();
    // Confirmation appears
    await expect(page.getByText('Delete this task?')).toBeVisible();
    await page.getByRole('button', { name: /yes/i }).click();

    // Task should be removed
    await expect(page.getByText('Task To Delete')).not.toBeVisible({ timeout: 5000 });
  });

  test('cancel delete keeps the task', async ({ apiClient, page }) => {
    await apiClient.createTask({ title: 'Task To Keep', status: 'backlog' });
    await page.reload();

    await page.getByText('Task To Keep').click();
    await expect(page.getByText('Edit Task')).toBeVisible();

    await page.getByRole('button', { name: 'Delete', exact: true }).click();
    await expect(page.getByText('Delete this task?')).toBeVisible();
    await page.getByRole('button', { name: 'No', exact: true }).click();

    // Close panel
    await page.getByRole('button', { name: /cancel/i }).click();

    // Task should still be visible
    await expect(page.getByText('Task To Keep')).toBeVisible();
  });

  test('task detail panel shows sessions section', async ({ apiClient, page }) => {
    await apiClient.createTask({ title: 'Panel Test Task', status: 'backlog' });
    await page.reload();

    await page.getByText('Panel Test Task').click();
    await expect(page.getByText('Edit Task')).toBeVisible();

    // Sessions section should be visible
    await expect(page.getByRole('heading', { name: 'Sessions' })).toBeVisible();
    await expect(page.getByRole('button', { name: /run session/i })).toBeVisible();
    await expect(page.getByText(/no sessions yet/i)).toBeVisible();
  });
});
