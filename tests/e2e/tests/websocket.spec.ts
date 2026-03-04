import { test, expect } from '../fixtures/test-fixtures';

test.describe('WebSocket Real-time Updates', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await expect(page.getByRole('heading', { name: 'Task Board' })).toBeVisible();
  });

  test('task created via API appears without manual refresh', async ({ apiClient, page }) => {
    // Wait for WebSocket to connect
    await page.waitForTimeout(1000);

    // Create task via API (bypassing UI)
    await apiClient.createTask({ title: 'WebSocket Task', status: 'backlog' });

    // Should appear on the board via WebSocket push
    await expect(page.getByText('WebSocket Task')).toBeVisible({ timeout: 10000 });
  });

  test('task deleted via API disappears without refresh', async ({ apiClient, page }) => {
    const task = await apiClient.createTask({ title: 'Delete WS Task', status: 'backlog' });
    await page.reload();
    await expect(page.getByText('Delete WS Task')).toBeVisible({ timeout: 5000 });

    // Wait for WebSocket to reconnect after reload
    await page.waitForTimeout(1000);

    // Delete via API
    await apiClient.deleteTask(task.id);

    // Should disappear via WebSocket push
    await expect(page.getByText('Delete WS Task')).not.toBeVisible({ timeout: 10000 });
  });

  test('multiple rapid creates all appear', async ({ apiClient, page }) => {
    // Wait for WebSocket
    await page.waitForTimeout(1000);

    // Create 3 tasks rapidly
    await Promise.all([
      apiClient.createTask({ title: 'Rapid Task 1', status: 'backlog' }),
      apiClient.createTask({ title: 'Rapid Task 2', status: 'backlog' }),
      apiClient.createTask({ title: 'Rapid Task 3', status: 'backlog' }),
    ]);

    // All should appear
    await expect(page.getByText('Rapid Task 1')).toBeVisible({ timeout: 10000 });
    await expect(page.getByText('Rapid Task 2')).toBeVisible({ timeout: 10000 });
    await expect(page.getByText('Rapid Task 3')).toBeVisible({ timeout: 10000 });
  });
});
