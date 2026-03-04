import { test as base } from '@playwright/test';
import { ApiClient } from '../helpers/api-client';

type Fixtures = {
  apiClient: ApiClient;
  resetData: void;
};

export const test = base.extend<Fixtures>({
  apiClient: async ({}, use) => {
    const client = new ApiClient();
    await use(client);
  },

  // Auto-fixture: resets all data before each test
  resetData: [
    async ({}, use) => {
      const client = new ApiClient();
      await client.resetAllData();
      await use();
    },
    { auto: true },
  ],
});

export { expect } from '@playwright/test';
