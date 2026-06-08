const DEFAULT_BASE_URL = 'http://127.0.0.1:17321'

function normalizeBaseUrl (baseUrl = DEFAULT_BASE_URL) {
  return String(baseUrl || DEFAULT_BASE_URL).replace(/\/+$/, '')
}

async function requestGateway (baseUrl, path, options = {}) {
  const url = normalizeBaseUrl(baseUrl) + path
  const res = await fetch(url, {
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...(options.headers || {})
    }
  })
  const text = await res.text()
  const body = text ? JSON.parse(text) : null
  if (!res.ok) {
    const msg = body && body.error ? body.error : `${res.status} ${res.statusText}`
    throw new Error(msg)
  }
  return body
}

async function getHealth (baseUrl) {
  return requestGateway(baseUrl, '/health', { method: 'GET' })
}

async function classifyCommand (baseUrl, command) {
  return requestGateway(baseUrl, '/api/v1/policy/classify', {
    method: 'POST',
    body: JSON.stringify({ command })
  })
}

async function listApprovals (baseUrl) {
  return requestGateway(baseUrl, '/api/v1/approvals', { method: 'GET' })
}

async function listHosts (baseUrl) {
  return requestGateway(baseUrl, '/api/v1/hosts', { method: 'GET' })
}

async function syncHosts (baseUrl, hosts) {
  return requestGateway(baseUrl, '/api/v1/hosts/sync', {
    method: 'POST',
    body: JSON.stringify({ hosts })
  })
}

async function listSessions (baseUrl) {
  return requestGateway(baseUrl, '/api/v1/sessions', { method: 'GET' })
}

async function pauseSession (baseUrl, sessionId) {
  return requestGateway(baseUrl, `/api/v1/sessions/${sessionId}/pause`, { method: 'POST' })
}

async function resumeSession (baseUrl, sessionId) {
  return requestGateway(baseUrl, `/api/v1/sessions/${sessionId}/resume`, { method: 'POST' })
}

async function setSessionMode (baseUrl, sessionId, mode) {
  return requestGateway(baseUrl, `/api/v1/sessions/${sessionId}/mode`, {
    method: 'POST',
    body: JSON.stringify({ mode })
  })
}

async function closeSession (baseUrl, sessionId) {
  return requestGateway(baseUrl, `/api/v1/sessions/${sessionId}`, { method: 'DELETE' })
}

async function listAuditEvents (baseUrl) {
  return requestGateway(baseUrl, '/api/v1/audit/events', { method: 'GET' })
}

async function getPolicyConfig (baseUrl) {
  return requestGateway(baseUrl, '/api/v1/policy', { method: 'GET' })
}

async function setPolicyConfig (baseUrl, payload) {
  return requestGateway(baseUrl, '/api/v1/policy', {
    method: 'PUT',
    body: JSON.stringify(payload)
  })
}

async function decideApproval (baseUrl, approvalId, payload) {
  return requestGateway(baseUrl, `/api/v1/approvals/${approvalId}/decision`, {
    method: 'POST',
    body: JSON.stringify(payload)
  })
}

async function claimTerminalCommand (baseUrl) {
  return requestGateway(baseUrl, '/api/v1/terminal/commands/next', {
    method: 'POST',
    body: JSON.stringify({})
  })
}

async function completeTerminalCommand (baseUrl, commandId, payload) {
  return requestGateway(baseUrl, `/api/v1/terminal/commands/${commandId}/complete`, {
    method: 'POST',
    body: JSON.stringify(payload)
  })
}

async function claimFileTask (baseUrl) {
  return requestGateway(baseUrl, '/api/v1/files/tasks/next', {
    method: 'POST',
    body: JSON.stringify({})
  })
}

async function completeFileTask (baseUrl, taskId, payload) {
  return requestGateway(baseUrl, `/api/v1/files/tasks/${taskId}/complete`, {
    method: 'POST',
    body: JSON.stringify(payload)
  })
}

module.exports = {
  DEFAULT_BASE_URL,
  normalizeBaseUrl,
  requestGateway,
  getHealth,
  classifyCommand,
  listHosts,
  syncHosts,
  listSessions,
  pauseSession,
  resumeSession,
  setSessionMode,
  closeSession,
  listApprovals,
  decideApproval,
  listAuditEvents,
  getPolicyConfig,
  setPolicyConfig,
  claimTerminalCommand,
  completeTerminalCommand,
  claimFileTask,
  completeFileTask
}
