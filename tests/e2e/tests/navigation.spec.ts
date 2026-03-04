import { test, expect } from '../fixtures/test-fixtures';

test.describe('Sidebar Navigation', () => {
  test('default page shows Task Board heading', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByRole('heading', { name: 'Task Board' })).toBeVisible();
  });

  test('click Agents sidebar shows Agent Pool heading', async ({ page }) => {
    await page.goto('/');
    await page.getByText('Agents').click();
    await expect(page.getByRole('heading', { name: 'Agent Pool' })).toBeVisible();
  });

  test('click Projects sidebar shows Projects heading', async ({ page }) => {
    await page.goto('/');
    await page.getByText('Projects').click();
    await expect(page.getByRole('heading', { name: 'Projects' })).toBeVisible();
  });

  test('navigate back to Task Board from another page', async ({ page }) => {
    await page.goto('/');
    await page.getByText('Agents').click();
    await expect(page.getByRole('heading', { name: 'Agent Pool' })).toBeVisible();

    await page.getByText('Task Board').click();
    await expect(page.getByRole('heading', { name: 'Task Board' })).toBeVisible();
  });

  test('navigate between all pages', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByRole('heading', { name: 'Task Board' })).toBeVisible();

    await page.getByText('Agents').click();
    await expect(page.getByRole('heading', { name: 'Agent Pool' })).toBeVisible();

    await page.getByText('Projects').click();
    await expect(page.getByRole('heading', { name: 'Projects' })).toBeVisible();

    await page.getByText('Task Board').click();
    await expect(page.getByRole('heading', { name: 'Task Board' })).toBeVisible();
  });
});
