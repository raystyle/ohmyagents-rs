import assert from 'node:assert/strict';
import test from 'node:test';

import { enforceCloudflareProjectPolicy } from '../scripts/enforce-cloudflare-project-policy.mjs';

const PROJECT_URL = 'https://cloudflare.invalid/accounts/account/pages/projects/rmux-web-share';

test('disables only automatic production deployments and preserves previews', async () => {
  const project = fixtureProject({ productionDeploymentsEnabled: true });
  const calls = [];
  const fetchImpl = async (url, options) => {
    calls.push({ url, options });
    if (options.method === 'PATCH') {
      const payload = JSON.parse(options.body);
      assert.equal(payload.source.type, 'github');
      assert.equal(payload.source.config.production_deployments_enabled, false);
      assert.equal(payload.source.config.preview_deployment_setting, 'all');
      assert.equal(payload.source.config.repo_name, 'rmux-web-share');
      project.source.config = structuredClone(payload.source.config);
    }
    return apiResponse(project);
  };

  const result = await enforceCloudflareProjectPolicy({
    accountId: 'account',
    apiToken: 'secret',
    apiBase: 'https://cloudflare.invalid',
    fetchImpl,
  });

  assert.deepEqual(result, { changed: true, previewDeploymentSetting: 'all' });
  assert.deepEqual(
    calls.map(({ url, options }) => [url, options.method]),
    [
      [PROJECT_URL, 'GET'],
      [PROJECT_URL, 'PATCH'],
      [PROJECT_URL, 'GET'],
    ],
  );
});

test('does not mutate an already compliant project', async () => {
  const project = fixtureProject({ productionDeploymentsEnabled: false });
  const methods = [];
  const fetchImpl = async (_url, options) => {
    methods.push(options.method);
    return apiResponse(project);
  };

  const result = await enforceCloudflareProjectPolicy({
    accountId: 'account',
    apiToken: 'secret',
    apiBase: 'https://cloudflare.invalid',
    fetchImpl,
  });

  assert.deepEqual(result, { changed: false, previewDeploymentSetting: 'all' });
  assert.deepEqual(methods, ['GET', 'GET']);
});

test('rejects a mismatched repository before mutation', async () => {
  const project = fixtureProject({ productionDeploymentsEnabled: true });
  project.source.config.repo_name = 'unexpected';
  const methods = [];
  const fetchImpl = async (_url, options) => {
    methods.push(options.method);
    return apiResponse(project);
  };

  await assert.rejects(
    enforceCloudflareProjectPolicy({
      accountId: 'account',
      apiToken: 'secret',
      apiBase: 'https://cloudflare.invalid',
      fetchImpl,
    }),
    /unexpected Cloudflare source repository/,
  );
  assert.deepEqual(methods, ['GET']);
});

function fixtureProject({ productionDeploymentsEnabled }) {
  return {
    name: 'rmux-web-share',
    source: {
      type: 'github',
      config: {
        owner: 'Helvesec',
        owner_id: 'owner-id',
        path_excludes: [],
        path_includes: ['*'],
        pr_comments_enabled: true,
        preview_branch_excludes: [],
        preview_branch_includes: ['*'],
        preview_deployment_setting: 'all',
        production_branch: 'main',
        production_deployments_enabled: productionDeploymentsEnabled,
        repo_id: 'repository-id',
        repo_name: 'rmux-web-share',
      },
    },
  };
}

function apiResponse(result) {
  return new Response(JSON.stringify({ success: true, errors: [], messages: [], result }), {
    status: 200,
    headers: { 'Content-Type': 'application/json' },
  });
}
