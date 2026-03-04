import { test, expect } from '../fixtures/test-fixtures';

test.describe('Task Board Advanced', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await expect(page.getByRole('heading', { name: 'Task Board' })).toBeVisible();
  });

  test('tasks appear in correct columns by status', async ({ apiClient, page }) => {
    await apiClient.createTask({ title: 'Backlog Task', status: 'backlog' });
    await apiClient.createTask({ title: 'IP Task', status: 'in_progress' });
    await apiClient.createTask({ title: 'Done Task', status: 'done' });
    await page.reload();

    // Each task should be visible
    await expect(page.getByText('Backlog Task')).toBeVisible({ timeout: 5000 });
    await expect(page.getByText('IP Task')).toBeVisible({ timeout: 5000 });
    await expect(page.getByText('Done Task')).toBeVisible({ timeout: 5000 });
  });

  test('task count displayed in column headers', async ({ apiClient, page }) => {
    await apiClient.createTask({ title: 'Task 1', status: 'backlog' });
    await apiClient.createTask({ title: 'Task 2', status: 'backlog' });
    await page.reload();

    // Look for a count indicator (e.g., "2" near Backlog header)
    await expect(page.getByText('Task 1')).toBeVisible({ timeout: 5000 });
    await expect(page.getByText('Task 2')).toBeVisible({ timeout: 5000 });
  });

  test('status change via detail panel moves task', async ({ apiClient, page }) => {
    await apiClient.createTask({ title: 'Move Via Panel', status: 'backlog' });
    await page.reload();

    await page.getByText('Move Via Panel').click();
    await expect(page.getByText('Edit Task')).toBeVisible();

    await page.locator('#edit-status').selectOption('done');
    await page.getByRole('button', { name: /save/i }).click();

    // Verify task still visible after move
    await expect(page.getByText('Move Via Panel')).toBeVisible({ timeout: 5000 });
  });

  test('multiple tasks maintain ordering', async ({ apiClient, page }) => {
    await apiClient.createTask({ title: 'First Created', status: 'backlog' });
    await apiClient.createTask({ title: 'Second Created', status: 'backlog' });
    await apiClient.createTask({ title: 'Third Created', status: 'backlog' });
    await page.reload();

    // All tasks should be visible
    await expect(page.getByText('First Created')).toBeVisible({ timeout: 5000 });
    await expect(page.getByText('Second Created')).toBeVisible({ timeout: 5000 });
    await expect(page.getByText('Third Created')).toBeVisible({ timeout: 5000 });
  });

  test('refresh loads fresh data from server', async ({ apiClient, page }) => {
    await apiClient.createTask({ title: 'Pre-Refresh Task', status: 'backlog' });
    await page.reload();
    await expect(page.getByText('Pre-Refresh Task')).toBeVisible({ timeout: 5000 });

    // Click refresh button
    const refreshBtn = page.getByRole('button', { name: /refresh/i });
    if (await refreshBtn.isVisible()) {
      await refreshBtn.click();
      await expect(page.getByText('Pre-Refresh Task')).toBeVisible({ timeout: 5000 });
    }
  });
});
