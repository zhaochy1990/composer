import { test, expect } from '../fixtures/test-fixtures';

test.describe('Error Handling', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await expect(page.getByRole('heading', { name: 'Task Board' })).toBeVisible();
  });

  test('empty title validation prevents creation', async ({ page }) => {
    await page.getByRole('button', { name: /new task/i }).click();
    // Try to submit without filling title
    const createBtn = page.getByRole('button', { name: /create task/i });
    // The title field should be required - either HTML validation or disabled button
    const titleInput = page.locator('#create-title');
    await expect(titleInput).toBeVisible();
    // Leave title empty and check that we cannot create
    await createBtn.click();
    // Task should not appear - we're still in the dialog
    await expect(page.locator('[role="dialog"]')).toBeVisible();
  });

  test('network failure shows error gracefully', async ({ page }) => {
    // Intercept API calls and force failure
    await page.route('**/api/tasks', route => {
      if (route.request().method() === 'POST') {
        route.fulfill({
          status: 500,
          contentType: 'application/json',
          body: JSON.stringify({ error: 'Internal Server Error' }),
        });
      } else {
        route.continue();
      }
    });

    await page.getByRole('button', { name: /new task/i }).click();
    await page.locator('#create-title').fill('Should Fail');
    await page.getByRole('button', { name: /create task/i }).click();

    // The task should NOT appear on the board
    await page.waitForTimeout(1000);
    await expect(page.getByText('Should Fail')).not.toBeVisible();
  });

  test('long title renders without overflow', async ({ apiClient, page }) => {
    const longTitle = 'A'.repeat(200);
    await apiClient.createTask({ title: longTitle });
    await page.reload();

    // The task card should be visible and contained
    const card = page.getByText(longTitle);
    await expect(card).toBeVisible({ timeout: 5000 });
  });

  test('special characters in title', async ({ apiClient, page }) => {
    const specialTitle = '<script>alert("xss")</script>';
    await apiClient.createTask({ title: specialTitle });
    await page.reload();

    // Should render as text, not execute
    await expect(page.getByText(specialTitle)).toBeVisible({ timeout: 5000 });
  });

  test('refresh after creating task persists data', async ({ page }) => {
    await page.getByRole('button', { name: /new task/i }).click();
    await page.locator('#create-title').fill('Persistent Task');
    await page.getByRole('button', { name: /create task/i }).click();
    await expect(page.getByText('Persistent Task')).toBeVisible({ timeout: 5000 });

    await page.reload();
    await expect(page.getByText('Persistent Task')).toBeVisible({ timeout: 5000 });
  });

  test('retry after fixing invalid input', async ({ page }) => {
    await page.getByRole('button', { name: /new task/i }).click();

    // First try with empty title
    await page.getByRole('button', { name: /create task/i }).click();

    // Now fill in the title and retry
    await page.locator('#create-title').fill('Retry Task');
    await page.getByRole('button', { name: /create task/i }).click();

    await expect(page.getByText('Retry Task')).toBeVisible({ timeout: 5000 });
  });
});
