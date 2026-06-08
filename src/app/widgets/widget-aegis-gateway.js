const { spawn } = require('child_process')
const { randomUUID } = require('crypto')
const fs = require('fs')
const path = require('path')
const {
  DEFAULT_BASE_URL,
  getHealth,
  classifyCommand,
  listApprovals,
  listAuditEvents
} = require('../lib/aegis-gateway-client')

const repoRoot = path.resolve(__dirname, '..', '..', '..')
const gatewayBinary = path.resolve(
  repoRoot,
  'target',
  'debug',
  process.platform === 'win32' ? 'aegis-gateway.exe' : 'aegis-gateway'
)

const widgetInfo = {
  name: 'Aegis Gateway',
  description: 'Start and monitor the local Rust Agent Gateway used by AI coding agents.',
  version: '0.1.0',
  author: 'Aegis',
  type: 'instance',
  builtin: true,
  singleInstance: true,
  configs: [
    {
      name: 'baseUrl',
      type: 'string',
      default: DEFAULT_BASE_URL,
      description: 'Gateway HTTP base URL'
    },
    {
      name: 'command',
      type: 'string',
      default: 'cargo',
      description: 'Executable used to start the Gateway'
    },
    {
      name: 'args',
      type: 'string',
      default: 'run -p aegis-gateway',
      description: 'Arguments passed to the Gateway command'
    },
    {
      name: 'cwd',
      type: 'string',
      default: repoRoot,
      description: 'Working directory for starting the Gateway'
    },
    {
      name: 'startupTimeoutMs',
      type: 'number',
      default: 15000,
      description: 'How long to wait for /health after starting'
    },
    {
      name: 'autoRun',
      type: 'boolean',
      default: false,
      description: 'Automatically start this Gateway when the app launches'
    }
  ]
}

function splitArgs (args) {
  return String(args || '')
    .split(/\s+/)
    .map(s => s.trim())
    .filter(Boolean)
}

function delay (ms) {
  return new Promise(resolve => setTimeout(resolve, ms))
}

class AegisGatewayWidget {
  constructor (config) {
    this.config = {
      baseUrl: DEFAULT_BASE_URL,
      command: 'cargo',
      args: 'run -p aegis-gateway',
      cwd: repoRoot,
      startupTimeoutMs: 15000,
      ...config
    }
    this.instanceId = randomUUID()
    this.child = null
    this.logs = []
  }

  async start () {
    const health = await this.tryHealth()
    if (health) {
      return this.startResult('Aegis Gateway already running')
    }

    this.child = spawn(this.config.command, splitArgs(this.config.args), {
      cwd: this.config.cwd,
      windowsHide: true,
      shell: false,
      env: process.env
    })

    this.child.stdout.on('data', data => this.appendLog(data))
    this.child.stderr.on('data', data => this.appendLog(data))
    this.child.on('exit', (code, signal) => {
      this.appendLog(`gateway exited: code=${code} signal=${signal}`)
      this.child = null
    })

    await this.waitUntilReady()
    return this.startResult('Aegis Gateway started')
  }

  async stop () {
    if (!this.child) {
      return
    }
    const child = this.child
    this.child = null
    killProcessTree(child)
    await delay(500)
  }

  async status () {
    const health = await this.tryHealth()
    return {
      running: !!health,
      health,
      baseUrl: this.config.baseUrl,
      pid: this.child ? this.child.pid : null,
      logs: this.logs.slice(-20)
    }
  }

  async classify (command) {
    return classifyCommand(this.config.baseUrl, command)
  }

  async approvals () {
    return listApprovals(this.config.baseUrl)
  }

  async auditEvents () {
    return listAuditEvents(this.config.baseUrl)
  }

  async tryHealth () {
    try {
      return await getHealth(this.config.baseUrl)
    } catch (_) {
      return null
    }
  }

  async waitUntilReady () {
    const started = Date.now()
    while (Date.now() - started < Number(this.config.startupTimeoutMs || 15000)) {
      const health = await this.tryHealth()
      if (health) {
        return health
      }
      if (!this.child || this.child.exitCode !== null) {
        break
      }
      await delay(500)
    }
    throw new Error(`Aegis Gateway failed to start. ${this.logs.slice(-5).join('\n')}`)
  }

  appendLog (data) {
    this.logs.push(String(data).trim())
    if (this.logs.length > 100) {
      this.logs.splice(0, this.logs.length - 100)
    }
  }

  startResult (msg) {
    const baseUrl = this.config.baseUrl
    return {
      instanceId: this.instanceId,
      msg,
      serverInfo: {
        url: baseUrl,
        path: [
          '/health',
          '/api/v1/hosts',
          '/api/v1/sessions',
          '/api/v1/commands',
          '/api/v1/approvals',
          '/api/v1/audit/events'
        ].join('\n')
      }
    }
  }
}

function resolveDefaultConfig (config = {}) {
  if (config.command) {
    return config
  }
  if (fs.existsSync(gatewayBinary)) {
    return {
      command: gatewayBinary,
      args: '',
      ...config
    }
  }
  return {
    command: 'cargo',
    args: 'run -p aegis-gateway',
    ...config
  }
}

function killProcessTree (child) {
  if (!child || child.killed) {
    return
  }
  if (process.platform === 'win32') {
    spawn('taskkill', ['/pid', String(child.pid), '/T', '/F'], {
      windowsHide: true,
      stdio: 'ignore'
    })
    return
  }
  child.kill('SIGTERM')
}

function widgetRun (config) {
  return new AegisGatewayWidget(resolveDefaultConfig(config))
}

module.exports = {
  widgetInfo,
  widgetRun
}
