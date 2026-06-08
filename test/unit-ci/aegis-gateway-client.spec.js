const test = require('node:test')
const assert = require('node:assert/strict')

const {
  DEFAULT_BASE_URL,
  normalizeBaseUrl,
  requestGateway
} = require('../../src/app/lib/aegis-gateway-client')

test('normalizes gateway base url', () => {
  assert.equal(normalizeBaseUrl('http://127.0.0.1:17321/'), DEFAULT_BASE_URL)
  assert.equal(normalizeBaseUrl('http://127.0.0.1:17321///'), DEFAULT_BASE_URL)
})

test('uses default gateway base url for empty input', () => {
  assert.equal(normalizeBaseUrl(''), DEFAULT_BASE_URL)
})

test('exports request helper for renderer ipc wrappers', () => {
  assert.equal(typeof requestGateway, 'function')
})
