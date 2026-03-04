import { test, expect } from '../fixtures/test-fixtures';

test.describe('Sessions via Task Detail', () => {
  test.beforeEach(async ({ apiClient, page }) => {
    await apiClient.createTask({ title: 'Session Test Task', status: 'backlog' });
    await page.goto('/');
    await page.getByText('Session Test Task').click();
    await expect(page.getByText('Edit Task')).toBeVisible();
  });

  test('Run Session dialog opens with all fields', async ({ page }) => {
    await page.getByRole('button', { name: /run session/i }).click();
    await expect(page.getByRole('heading', { name: 'New Session' })).toBeVisible();

    // Verify all form fields are present
    await expect(page.locator('#session-agent')).toBeVisible();
    await expect(page.locator('#session-prompt')).toBeVisible();
    await expect(page.locator('#session-repo')).toBeVisible();
    await expect(page.getByText(/auto-approve/i)).toBeVisible();
  });

  test('no-agents warning when no agents registered', async ({ page }) => {
    await page.getByRole('button', { name: /run session/i }).click();
    await expect(page.getByRole('heading', { name: 'New Session' })).toBeVisible();

    // The agent select should show a warning or have no options
    const agentSelect = page.locator('#session-agent');
    const options = agentSelect.locator('option');
    // Only the placeholder/empty option should exist (no real agents)
    const count = await options.count();
    expect(count).toBeLessThanOrEqual(1);
  });

  test('agent dropdown populates when agents exist', async ({ apiClient, page }) => {
    await apiClient.createAgent({ name: 'Session Test Agent' });
    await page.reload();

    // Re-open task detail panel
    await page.getByText('Session Test Task').click();
    await expect(page.getByText('Edit Task')).toBeVisible();

    await page.getByRole('button', { name: /run session/i }).click();
    await expect(page.getByRole('heading', { name: 'New Session' })).toBeVisible();

    // The agent dropdown should contain the created agent
    await expect(page.locator('#session-agent').locator('option', { hasText: 'Session Test Agent' })).toBeAttached();
  });

  test('empty sessions shows placeholder message', async ({ page }) => {
    await expect(page.getByText(/no sessions yet/i)).toBeVisible();
  });
});
