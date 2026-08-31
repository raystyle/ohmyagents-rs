import { pathToFileURL } from 'node:url';

const API_BASE = 'https://api.cloudflare.com/client/v4';
const EXPECTED_SOURCE = Object.freeze({
  type: 'github',
  owner: 'Helvesec',
  repository: 'rmux-web-share',
  productionBranch: 'main',
});
const WRITABLE_CONFIG_KEYS = Object.freeze([
  'owner',
  'owner_id',
  'path_excludes',
  'path_includes',
  'pr_comments_enabled',
  'preview_branch_excludes',
  'preview_branch_includes',
  'preview_deployment_setting',
  'production_branch',
  'production_deployments_enabled',
  'repo_id',
  'repo_name',
]);

export async function enforceCloudflareProjectPolicy({
  accountId,
  apiToken,
  projectName = 'rmux-web-share',
  apiBase = API_BASE,
  fetchImpl = fetch,
} = {}) {
  requireValue(accountId, 'Cloudflare account ID');
  requireValue(apiToken, 'Cloudflare API token');
  requireValue(projectName, 'Cloudflare Pages project name');

  const projectUrl = `${apiBase}/accounts/${encodeURIComponent(accountId)}/pages/projects/${encodeURIComponent(projectName)}`;
  const before = await cloudflareRequest(fetchImpl, projectUrl, apiToken);
  const beforeConfig = validateProjectIdentity(before, projectName);
  const previewSetting = beforeConfig.preview_deployment_setting;

  if (beforeConfig.production_deployments_enabled !== false) {
    const payload = buildPolicyUpdate(before);
    await cloudflareRequest(fetchImpl, projectUrl, apiToken, {
      method: 'PATCH',
      body: JSON.stringify(payload),
    });
  }

  const after = await cloudflareRequest(fetchImpl, projectUrl, apiToken);
  const afterConfig = validateProjectIdentity(after, projectName);
  if (afterConfig.production_deployments_enabled !== false) {
    throw new Error('Cloudflare automatic production deployments remain enabled');
  }
  if (afterConfig.preview_deployment_setting !== previewSetting) {
    throw new Error('Cloudflare preview deployment policy changed unexpectedly');
  }

  return {
    changed: beforeConfig.production_deployments_enabled !== false,
    previewDeploymentSetting: previewSetting,
  };
}

export function buildPolicyUpdate(project) {
  const source = requireRecord(project.source, 'Cloudflare project source');
  const config = requireRecord(source.config, 'Cloudflare project source config');
  const updateConfig = {};

  for (const key of WRITABLE_CONFIG_KEYS) {
    if (Object.hasOwn(config, key) && config[key] !== null) {
      updateConfig[key] = config[key];
    }
  }
  updateConfig.production_deployments_enabled = false;

  return {
    source: {
      type: source.type,
      config: updateConfig,
    },
  };
}

function validateProjectIdentity(project, projectName) {
  const value = requireRecord(project, 'Cloudflare project');
  if (value.name !== projectName) {
    throw new Error(`unexpected Cloudflare project: ${String(value.name)}`);
  }

  const source = requireRecord(value.source, 'Cloudflare project source');
  const config = requireRecord(source.config, 'Cloudflare project source config');
  const expectations = [
    ['source type', source.type, EXPECTED_SOURCE.type],
    ['source owner', config.owner, EXPECTED_SOURCE.owner],
    ['source repository', config.repo_name, EXPECTED_SOURCE.repository],
    ['production branch', config.production_branch, EXPECTED_SOURCE.productionBranch],
  ];
  for (const [label, actual, expected] of expectations) {
    if (actual !== expected) {
      throw new Error(`unexpected Cloudflare ${label}: ${String(actual)}`);
    }
  }

  if (!['all', 'custom', 'none'].includes(config.preview_deployment_setting)) {
    throw new Error('Cloudflare preview deployment policy is missing or unsupported');
  }
  if (typeof config.production_deployments_enabled !== 'boolean') {
    throw new Error('Cloudflare production deployment policy is missing');
  }
  return config;
}

async function cloudflareRequest(fetchImpl, url, apiToken, options = {}) {
  const method = options.method ?? 'GET';
  const response = await fetchImpl(url, {
    ...options,
    method,
    headers: {
      Authorization: `Bearer ${apiToken}`,
      'Content-Type': 'application/json',
      ...(options.headers ?? {}),
    },
  });
  const envelope = await response.json().catch(() => null);
  if (!response.ok || envelope?.success !== true) {
    const errors = Array.isArray(envelope?.errors)
      ? envelope.errors.map((error) => error?.message).filter(Boolean).join('; ')
      : '';
    throw new Error(`Cloudflare API ${method} failed (${response.status})${errors ? `: ${errors}` : ''}`);
  }
  return envelope.result;
}

function requireRecord(value, label) {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    throw new Error(`${label} is missing or malformed`);
  }
  return value;
}

function requireValue(value, label) {
  if (typeof value !== 'string' || value.length === 0) {
    throw new Error(`${label} is required`);
  }
}

async function main() {
  const result = await enforceCloudflareProjectPolicy({
    accountId: process.env.CLOUDFLARE_ACCOUNT_ID,
    apiToken: process.env.CLOUDFLARE_API_TOKEN,
    projectName: process.env.CLOUDFLARE_PAGES_PROJECT ?? 'rmux-web-share',
  });
  const action = result.changed ? 'disabled' : 'already disabled';
  console.log(`Cloudflare automatic production deployments: ${action}`);
  console.log(`Cloudflare preview deployment policy preserved: ${result.previewDeploymentSetting}`);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main().catch((error) => {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  });
}
