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

async function listSessions (baseUrl) {
  return requestGateway(baseUrl, '/api/v1/sessions', { method: 'GET' })
}

async function listAuditEvents (baseUrl) {
  return requestGateway(baseUrl, '/api/v1/audit/events', { method: 'GET' })
}

module.exports = {
  DEFAULT_BASE_URL,
  normalizeBaseUrl,
  requestGateway,
  getHealth,
  classifyCommand,
  listHosts,
  listSessions,
  listApprovals,
  listAuditEvents
}
