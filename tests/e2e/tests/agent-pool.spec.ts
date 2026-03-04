import { test, expect } from '../fixtures/test-fixtures';

test.describe('Agent Pool', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await page.getByText('Agents').click();
    await expect(page.getByRole('heading', { name: 'Agent Pool' })).toBeVisible();
  });

  test('empty state shows no agents message', async ({ page }) => {
    await expect(page.getByText(/no agents registered/i)).toBeVisible();
  });

  test('register a new agent', async ({ page }) => {
    await page.getByRole('button', { name: /add agent/i }).click();
    await expect(page.getByText('Register Agent')).toBeVisible();

    await page.locator('#agent-name').fill('Test Agent');
    await page.getByRole('button', { name: /register/i }).click();

    await expect(page.getByText('Test Agent')).toBeVisible({ timeout: 5000 });
  });

  test('delete an agent', async ({ apiClient, page }) => {
    await apiClient.createAgent({ name: 'Agent To Delete' });
    await page.reload();
    await page.getByText('Agents').click();

    await expect(page.getByText('Agent To Delete')).toBeVisible({ timeout: 5000 });

    // Hover the agent card to reveal the delete button, then click it
    const agentCard = page.locator('.group', { hasText: 'Agent To Delete' });
    await agentCard.hover();
    await agentCard.getByTitle('Delete agent').click();

    await expect(page.getByText('Agent To Delete')).not.toBeVisible({ timeout: 5000 });
  });
});
