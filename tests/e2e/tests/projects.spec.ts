import { test, expect } from '../fixtures/test-fixtures';

test.describe('Projects', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await page.getByText('Projects').click();
    await expect(page.getByRole('heading', { name: 'Projects' })).toBeVisible();
  });

  test('empty state shows no projects message', async ({ page }) => {
    await expect(page.getByText(/no projects yet/i)).toBeVisible();
  });

  test('create a new project', async ({ page }) => {
    await page.getByRole('button', { name: /new project/i }).click();
    await expect(page.getByRole('heading', { name: 'New Project' })).toBeVisible();

    await page.locator('#project-name').fill('My Test Project');
    await page.locator('#project-description').fill('A test project description');
    await page.getByRole('button', { name: /create project/i }).click();

    await expect(page.getByText('My Test Project')).toBeVisible({ timeout: 5000 });
  });

  test('create project without description', async ({ page }) => {
    await page.getByRole('button', { name: /new project/i }).click();
    await page.locator('#project-name').fill('Minimal Project');
    await page.getByRole('button', { name: /create project/i }).click();

    await expect(page.getByText('Minimal Project')).toBeVisible({ timeout: 5000 });
  });

  test('project card shows repo count', async ({ apiClient, page }) => {
    await apiClient.createProject({ name: 'Repo Count Project' });
    await page.reload();
    await page.getByText('Projects').click();

    await expect(page.getByText('Repo Count Project')).toBeVisible({ timeout: 5000 });
    await expect(page.getByText('0 repos')).toBeVisible();
  });

  test('open project detail panel by clicking card', async ({ apiClient, page }) => {
    await apiClient.createProject({ name: 'Detail Panel Project', description: 'Test description' });
    await page.reload();
    await page.getByText('Projects').click();

    // Click the project card (inside the grid, not the detail panel)
    await page.locator('button', { hasText: 'Detail Panel Project' }).click();

    // Detail panel should show edit/delete buttons
    await expect(page.getByRole('button', { name: 'Edit' })).toBeVisible({ timeout: 5000 });
    await expect(page.getByRole('button', { name: 'Delete', exact: true })).toBeVisible();
    // Description appears in the detail panel
    await expect(page.locator('.border-l').getByText('Test description')).toBeVisible();
  });

  test('edit project name and description', async ({ apiClient, page }) => {
    await apiClient.createProject({ name: 'Old Name', description: 'Old desc' });
    await page.reload();
    await page.getByText('Projects').click();

    await page.locator('button', { hasText: 'Old Name' }).click();
    await page.getByRole('button', { name: 'Edit' }).click();

    // Edit form should appear - fill in the name input in the detail panel
    const nameInput = page.locator('.border-l input').first();
    await nameInput.clear();
    await nameInput.fill('Updated Name');

    await page.getByRole('button', { name: 'Save' }).click();

    await expect(page.getByText('Updated Name')).toBeVisible({ timeout: 5000 });
  });

  test('cancel edit keeps original values', async ({ apiClient, page }) => {
    await apiClient.createProject({ name: 'Keep Me', description: 'Original desc' });
    await page.reload();
    await page.getByText('Projects').click();

    await page.locator('button', { hasText: 'Keep Me' }).click();
    await page.getByRole('button', { name: 'Edit' }).click();
    await page.getByRole('button', { name: 'Cancel' }).click();

    // Original name should still be visible in the panel header
    await expect(page.locator('.border-l').getByText('Keep Me')).toBeVisible();
  });

  test('delete a project', async ({ apiClient, page }) => {
    await apiClient.createProject({ name: 'Project To Delete' });
    await page.reload();
    await page.getByText('Projects').click();

    await page.locator('button', { hasText: 'Project To Delete' }).click();
    await page.getByRole('button', { name: 'Delete', exact: true }).click();

    // After delete, should return to the empty state
    await expect(page.getByText(/no projects yet/i)).toBeVisible({ timeout: 5000 });
  });

  test('close detail panel', async ({ apiClient, page }) => {
    await apiClient.createProject({ name: 'Close Panel Project' });
    await page.reload();
    await page.getByText('Projects').click();

    await page.locator('button', { hasText: 'Close Panel Project' }).click();
    // Panel should be open - verify Repositories heading is visible
    await expect(page.getByRole('heading', { name: 'Repositories' })).toBeVisible({ timeout: 5000 });

    // Close the panel using the X button in the panel header
    const closeButton = page.locator('.border-l button').first();
    await closeButton.click();

    // Repositories section should no longer be visible (panel closed)
    await expect(page.getByRole('heading', { name: 'Repositories' })).not.toBeVisible({ timeout: 3000 });
  });

  test('repositories section shows empty state', async ({ apiClient, page }) => {
    await apiClient.createProject({ name: 'Empty Repos Project' });
    await page.reload();
    await page.getByText('Projects').click();

    await page.locator('button', { hasText: 'Empty Repos Project' }).click();
    await expect(page.getByRole('heading', { name: 'Repositories' })).toBeVisible({ timeout: 5000 });
    await expect(page.getByText(/no repositories added/i)).toBeVisible();
  });

  test('create dialog requires name', async ({ page }) => {
    await page.getByRole('button', { name: /new project/i }).click();

    // Create button should be disabled when name is empty
    const createBtn = page.getByRole('button', { name: /create project/i });
    await expect(createBtn).toBeDisabled();

    // Type a name
    await page.locator('#project-name').fill('Valid Name');
    await expect(createBtn).toBeEnabled();

    // Clear the name
    await page.locator('#project-name').clear();
    await expect(createBtn).toBeDisabled();
  });

  test('cancel create dialog does not create project', async ({ page }) => {
    await page.getByRole('button', { name: /new project/i }).click();
    await page.locator('#project-name').fill('Should Not Exist');
    await page.getByRole('button', { name: 'Cancel' }).click();

    await expect(page.getByText('Should Not Exist')).not.toBeVisible();
    await expect(page.getByText(/no projects yet/i)).toBeVisible();
  });
});

test.describe('Projects API', () => {
  test('CRUD operations via API client', async ({ apiClient }) => {
    // Create
    const project = await apiClient.createProject({
      name: 'API Project',
      description: 'Created via API',
    });
    expect(project.id).toBeTruthy();
    expect(project.name).toBe('API Project');
    expect(project.description).toBe('Created via API');

    // Read
    const fetched = await apiClient.getProject(project.id);
    expect(fetched.name).toBe('API Project');

    // List
    const projects = await apiClient.listProjects();
    expect(projects.some((p) => p.id === project.id)).toBe(true);

    // Update
    const updated = await apiClient.updateProject(project.id, { name: 'Updated API Project' });
    expect(updated.name).toBe('Updated API Project');

    // Delete
    await apiClient.deleteProject(project.id);
    const afterDelete = await apiClient.listProjects();
    expect(afterDelete.some((p) => p.id === project.id)).toBe(false);
  });

  test('project tasks endpoint returns tasks linked to project', async ({ apiClient }) => {
    const project = await apiClient.createProject({ name: 'Task Link Project' });

    // Create a task without project_id
    await apiClient.createTask({ title: 'Unlinked Task' });

    // Project tasks should be empty (task not linked)
    const projectTasks = await apiClient.listProjectTasks(project.id);
    expect(projectTasks).toHaveLength(0);
  });
});
