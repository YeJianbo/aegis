/**
 * common functions
 */

import handleError from '../common/error-handler'
import Modal from '../components/common/modal'
import { debounce, some, get, pickBy } from 'lodash-es'
import {
  leftSidebarWidthKey,
  rightSidebarWidthKey,
  addPanelWidthLsKey,
  dismissDelKeyTipLsKey,
  connectionMap,
  paneMap
} from '../common/constants'
import * as ls from '../common/safe-local-storage'
import { refs, refsStatic } from '../components/common/ref'
import { action } from 'manate'
import uid from '../common/uid'
import deepCopy from 'json-deep-copy'
import { aiConfigsArr } from '../components/ai/ai-config-props'

const e = window.translate
const { assign } = Object

function sleep (ms) {
  return new Promise(resolve => setTimeout(resolve, ms))
}

function quotePosixSingle (value) {
  return `'${String(value).replace(/'/g, "'\\''")}'`
}

function escapeRegExp (value) {
  return String(value).replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
}

const aegisAnsi = {
  reset: '\\033[0m',
  bold: '\\033[1m',
  dim: '\\033[2m',
  fgCyan: '\\033[36m',
  fgGreen: '\\033[32m',
  fgRed: '\\033[31m',
  bgMagenta: '\\033[45;97m',
  bgGreen: '\\033[42;30m',
  bgRed: '\\033[41;97m',
  bgYellow: '\\033[43;30m',
  bgOrange: '\\033[48;5;208;30m'
}

function isUsableSshTab (tab) {
  return tab && tab.type === 'ssh' && tab.host
}

function isUsableAegisFileTab (tab) {
  return tab && (tab.type === 'ssh' || tab.type === 'ftp') && tab.host
}

function tabMatchesAegisHost (tab, hostId, host, isUsable = isUsableSshTab) {
  if (!isUsable(tab)) return false
  if (tab.srcId && tab.srcId === hostId) return true
  if (tab.id && tab.id === hostId) return true
  if (!host) return false
  const address = String(host.address || '').split(':')[0]
  return !!address && tab.host === address
}

function activateAegisTab (store, tab) {
  store.activeTabId = tab.id
  if (tab.batch !== undefined) {
    store[`activeTabId${tab.batch}`] = tab.id
    store.currentLayoutBatch = tab.batch
  }
}

function aegisRiskBadge (risk) {
  const label = String(risk || 'low').toUpperCase()
  if (risk === 'critical') return `${aegisAnsi.bgRed} ${label} ${aegisAnsi.reset}`
  if (risk === 'high') return `${aegisAnsi.bgOrange} ${label} ${aegisAnsi.reset}`
  if (risk === 'medium') return `${aegisAnsi.bgYellow} ${label} ${aegisAnsi.reset}`
  return `${aegisAnsi.bgGreen} ${label} ${aegisAnsi.reset}`
}

export default Store => {
  Store.prototype.storeAssign = function (updates) {
    assign(window.store, updates)
  }

  Store.prototype.onError = function (e) {
    handleError(e)
  }

  Store.prototype.updateConfig = function (ext) {
    window.store.setConfig(ext)
  }

  Store.prototype.openInfoPanel = action(function () {
    const { store } = window
    store.rightPanelVisible = true
    store.rightPanelTab = 'info'
    store.openInfoPanelAction()
  })

  Store.prototype.openInfoPanelAction = function () {
    const { store } = window
    setTimeout(() => {
      const term = refs.get('term-' + store.activeTabId)
      term && term.handleShowInfo()
    }, 300)
  }

  Store.prototype.toggleAIConfig = function () {
    window.store.showAIConfigModal = true
  }

  Store.prototype.onResize = debounce(async function () {
    const { width, height } = await window.pre.runGlobalAsync('getScreenSize')
    const isMaximized = window.pre.runSync('isMaximized')
    const update = {
      height: window.innerHeight,
      innerWidth: window.innerWidth,
      screenWidth: width,
      screenHeight: height,
      isMaximized
    }
    window.store.storeAssign(update)
    window.pre.runGlobalAsync('setWindowSize', {
      ...update,
      height: window.outerHeight
    })
  }, 100, {
    leading: true
  })

  Store.prototype.toggleTerminalSearch = function () {
    const now = Date.now()
    if (window.lastToggleTerminalSearch && now - window.lastToggleTerminalSearch < 300) {
      return
    }
    window.lastToggleTerminalSearch = now
    window.store.termSearchOpen = !window.store.termSearchOpen
  }

  Store.prototype.setState = function (name, value) {
    window.store['_' + name] = JSON.stringify(value)
  }

  Store.prototype.setSettingItem = function (v) {
    window.store.settingItem = v
  }

  Store.prototype.setTermSearchOption = function (update) {
    Object.assign(window.store._termSearchOptions, update)
  }

  Store.prototype.setLeftSidePanelWidth = function (v) {
    ls.setItem(leftSidebarWidthKey, v)
    window.store.leftSidebarWidth = v
  }

  Store.prototype.setAddPanelWidth = function (v) {
    ls.setItem(addPanelWidthLsKey, v)
    window.store.addPanelWidth = v
  }

  Store.prototype.setRightSidePanelWidth = function (v) {
    ls.setItem(rightSidebarWidthKey, v)
    window.store.rightPanelWidth = v
  }
  Store.prototype.dismissDelKeyTip = function (v) {
    ls.setItem(dismissDelKeyTipLsKey, 'y')
    window.store.hideDelKeyTip = true
  }
  Store.prototype.beforeExit = function (evt) {
    const { confirmBeforeExit } = window.store.config
    if (
      (confirmBeforeExit &&
      !window.confirmExit) ||
      window.store.isTransporting
    ) {
      evt.returnValue = false
      let mod = null
      mod = Modal.confirm({
        onCancel: () => {
          window.confirmExit = false
          mod.destroy()
        },
        onOk: () => {
          window.confirmExit = true
          window.store[window.exitFunction]()
        },
        title: e('quit'),
        okText: e('ok'),
        cancelText: e('cancel'),
        content: ''
      })
    }
  }
  Store.prototype.beforeExitApp = function (evt, name) {
    let mod = null
    mod = Modal.confirm({
      onCancel: () => {
        window.pre.runGlobalAsync('setCloseAction', 'closeApp')
        mod.destroy()
      },
      onOk: () => {
        window.pre.runGlobalAsync(name)
      },
      title: e('quit'),
      okText: e('ok'),
      cancelText: e('cancel'),
      content: ''
    })
  }

  Store.prototype.toggleResolutionEdit = function () {
    window.store.openResolutionEdit = !window.store.openResolutionEdit
  }

  Store.prototype.setTerminalInfos = function (arr) {
    window.store.setConfig({
      terminalInfos: arr
    })
  }

  Store.prototype.applyProfile = function (tab) {
    const {
      profile,
      type,
      authType
    } = tab
    if (!profile || authType !== 'profiles') {
      return tab
    }
    let p = window.store.profiles.find(x => x.id === profile)
    if (!p) {
      return tab
    }
    p = deepCopy(p)
    // delete tab.password
    // delete tab.privateKey
    // delete tab.passphrase
    delete p.name
    delete p.id
    if (type === connectionMap.rdp) {
      const filtered = pickBy(p.rdp, (value) => value !== undefined && value !== '')
      return {
        ...tab,
        ...filtered
      }
    } else if (type === connectionMap.vnc) {
      const filtered = pickBy(p.vnc, (value) => value !== undefined && value !== '')
      return {
        ...tab,
        ...filtered
      }
    } else if (type === connectionMap.telnet) {
      const filtered = pickBy(p.telnet, (value) => value !== undefined && value !== '')
      return {
        ...tab,
        ...filtered
      }
    }
    delete p.rdp
    delete p.vnc
    delete p.telnet
    const filtered = pickBy(p, (value) => value !== undefined && value !== '')
    return {
      ...tab,
      ...filtered
    }
  }
  Store.prototype.applyProfileToTabs = function (tab) {
    if (
      tab.connectionHoppings &&
      tab.connectionHoppings.length &&
      some(tab.connectionHoppings, s => s.profile)
    ) {
      tab.connectionHoppings = tab.connectionHoppings.map(s => {
        return window.store.applyProfile(s)
      })
    }
    return window.store.applyProfile(tab)
  }

  Store.prototype.handleOpenAIPanel = function () {
    const { store } = window
    store.rightPanelVisible = true
    store.rightPanelTab = 'ai'
  }

  Store.prototype.handleOpenAegisPanel = function () {
    const { store } = window
    store.rightPanelVisible = true
    store.rightPanelTab = 'agent'
    store.refreshAegisGatewayStatus()
  }

  Store.prototype.refreshAegisGatewayStatus = async function () {
    const { store } = window
    store.aegisGatewayStatus.loading = true
    store.aegisGatewayStatus.error = ''
    try {
      const health = await window.pre.runGlobalAsync('aegisGatewayHealth')
      const syncedHosts = await window.pre.runGlobalAsync(
        'aegisGatewaySyncHosts',
        store.buildAegisHostsFromBookmarks()
      )
      const [
        sessions,
        approvals,
        auditEvents,
        policy
      ] = await Promise.all([
        window.pre.runGlobalAsync('aegisGatewaySessions'),
        window.pre.runGlobalAsync('aegisGatewayApprovals'),
        window.pre.runGlobalAsync('aegisGatewayAuditEvents'),
        window.pre.runGlobalAsync('aegisGatewayPolicy')
      ])
      store.aegisGatewayStatus = {
        loading: false,
        online: true,
        error: '',
        health,
        hosts: syncedHosts,
        sessions,
        approvals,
        auditEvents,
        policy
      }
      store.startAegisTerminalExecutor()
      store.startAegisFileExecutor()
    } catch (err) {
      store.aegisGatewayStatus = {
        ...store.aegisGatewayStatus,
        loading: false,
        online: false,
        error: err.message || String(err)
      }
    }
  }

  Store.prototype.bootstrapAegisGatewayIntegration = function () {
    const { store } = window
    if (store._aegisGatewayBootstrapRunning) {
      return
    }
    store._aegisGatewayBootstrapRunning = true

    const loop = async () => {
      while (store._aegisGatewayBootstrapRunning) {
        try {
          const health = await window.pre.runGlobalAsync('aegisGatewayHealth')
          const syncedHosts = await window.pre.runGlobalAsync(
            'aegisGatewaySyncHosts',
            store.buildAegisHostsFromBookmarks()
          )
          const policy = await window.pre.runGlobalAsync('aegisGatewayPolicy')
          store.aegisGatewayStatus = {
            ...store.aegisGatewayStatus,
            online: true,
            error: '',
            health,
            hosts: syncedHosts,
            policy
          }
          store.startAegisTerminalExecutor()
          store.startAegisFileExecutor()
        } catch (_) {
          store.aegisGatewayStatus = {
            ...store.aegisGatewayStatus,
            online: false
          }
        }
        await sleep(5000)
      }
    }

    loop()
  }

  Store.prototype.buildAegisHostsFromBookmarks = function () {
    const { bookmarks } = window.store
    return bookmarks
      .filter(bookmark => (bookmark.type === 'ssh' || bookmark.type === 'ftp') && bookmark.host)
      .map(bookmark => {
        const tags = [
          bookmark.type,
          bookmark.username ? `user:${bookmark.username}` : '',
          bookmark.port ? `port:${bookmark.port}` : ''
        ].filter(Boolean)
        return {
          id: bookmark.id,
          name: bookmark.title || bookmark.host,
          address: bookmark.port ? `${bookmark.host}:${bookmark.port}` : bookmark.host,
          tags
        }
      })
  }

  Store.prototype.startAegisTerminalExecutor = function () {
    const { store } = window
    if (store._aegisTerminalExecutorRunning) {
      return
    }
    store._aegisTerminalExecutorRunning = true

    const loop = async () => {
      while (store._aegisTerminalExecutorRunning) {
        try {
          const task = await window.pre.runGlobalAsync('aegisGatewayClaimTerminalCommand')
          if (!task) {
            await sleep(1000)
            continue
          }
          await store.executeAegisTerminalCommand(task)
          if (store.rightPanelTab === 'agent') {
            store.refreshAegisGatewayStatus().catch(() => {})
          }
        } catch (err) {
          store.aegisGatewayStatus = {
            ...store.aegisGatewayStatus,
            executorError: err.message || String(err)
          }
          await sleep(2000)
        }
      }
    }

    loop()
  }

  Store.prototype.startAegisFileExecutor = function () {
    const { store } = window
    if (store._aegisFileExecutorRunning) {
      return
    }
    store._aegisFileExecutorRunning = true

    const loop = async () => {
      while (store._aegisFileExecutorRunning) {
        try {
          const task = await window.pre.runGlobalAsync('aegisGatewayClaimFileTask')
          if (!task) {
            await sleep(1000)
            continue
          }
          await store.executeAegisFileTask(task)
          if (store.rightPanelTab === 'agent') {
            store.refreshAegisGatewayStatus().catch(() => {})
          }
        } catch (err) {
          store.aegisGatewayStatus = {
            ...store.aegisGatewayStatus,
            fileExecutorError: err.message || String(err)
          }
          await sleep(2000)
        }
      }
    }

    loop()
  }

  Store.prototype.executeAegisFileTask = async function (task) {
    const { store } = window
    let tabId = ''
    try {
      tabId = await store.resolveAegisFileTab(task.host_id)
      await store.ensureAegisSftpReady(tabId)

      const args = {
        tabId,
        remotePath: task.remote_path,
        localPath: task.local_path,
        content: task.content
      }
      let result
      switch (task.operation) {
        case 'list':
          result = await store.mcpSftpList(args)
          break
        case 'stat':
          result = await store.mcpSftpStat(args)
          break
        case 'read_file':
          result = await store.mcpSftpReadFile(args)
          break
        case 'write_file':
          result = await store.mcpSftpWriteFile(args)
          break
        case 'delete':
          result = await store.mcpSftpDel(args)
          break
        case 'upload':
          result = await store.mcpSftpUpload(args)
          break
        case 'download':
          result = await store.mcpSftpDownload(args)
          break
        default:
          throw new Error(`Unsupported Aegis file operation: ${task.operation}`)
      }

      await store.completeAegisFileTask(task.id, {
        result,
        tab_id: tabId
      })
    } catch (err) {
      await store.completeAegisFileTask(task.id, {
        error: err.message || String(err),
        tab_id: tabId || null
      })
    }
  }

  Store.prototype.resolveAegisFileTab = async function (hostId) {
    const { store } = window
    const host = store.aegisGatewayStatus.hosts.find(item => item.id === hostId)
    const current = store.currentTab
    if (tabMatchesAegisHost(current, hostId, host, isUsableAegisFileTab)) {
      return store.activeTabId
    }

    const existing = store.tabs.find(tab => (
      tabMatchesAegisHost(tab, hostId, host, isUsableAegisFileTab)
    ))
    if (existing) {
      activateAegisTab(store, existing)
      return existing.id
    }

    const bookmark = store.bookmarks.find(item => item.id === hostId)
    if (!bookmark) {
      throw new Error(`No connected SFTP/FTP tab or bookmark for host: ${hostId}`)
    }
    if (bookmark.type !== 'ssh' && bookmark.type !== 'ftp') {
      throw new Error(`Host ${hostId} is not an SSH/FTP bookmark`)
    }
    store.onSelectBookmark(hostId)
    await sleep(2500)
    const opened = store.tabs.find(tab => tabMatchesAegisHost(tab, hostId, host, isUsableAegisFileTab))
    if (!opened && !tabMatchesAegisHost(store.currentTab, hostId, host, isUsableAegisFileTab)) {
      throw new Error(`Failed to open SFTP/FTP bookmark for host: ${hostId}`)
    }
    return opened ? opened.id : store.activeTabId
  }

  Store.prototype.ensureAegisSftpReady = async function (tabId) {
    const { store } = window
    const tab = store.tabs.find(item => item.id === tabId)
    if (!tab) {
      throw new Error(`Tab not found: ${tabId}`)
    }

    activateAegisTab(store, tab)
    if (tab.type === 'ssh' && tab.pane !== paneMap.fileManager) {
      store.updateTab(tabId, { pane: paneMap.fileManager })
      store.triggerResize?.()
    }

    const started = Date.now()
    while (Date.now() - started < 12000) {
      const sftpEntry = refs.get('sftp-' + tabId)
      if (sftpEntry?.sftp) {
        return sftpEntry
      }
      await sleep(400)
    }
    throw new Error(`SFTP not initialized for tab "${tabId}". Open the SFTP panel first.`)
  }

  Store.prototype.completeAegisFileTask = async function (taskId, payload) {
    await window.pre.runGlobalAsync('aegisGatewayCompleteFileTask', taskId, payload)
  }

  Store.prototype.executeAegisTerminalCommand = async function (task) {
    const { store } = window
    let tabId = ''
    try {
      tabId = await store.resolveAegisTerminalTab(task.host_id)
      const sentinel = `__AEGIS_EXIT_${String(task.id).replace(/-/g, '_')}__`
      const markerFormat = [
        `${aegisAnsi.bgMagenta} Aegis Agent ${aegisAnsi.reset}`,
        `${aegisRiskBadge(task.risk)} actor=%s`,
        `${aegisAnsi.fgCyan}${aegisAnsi.bold}$ %s${aegisAnsi.reset}\\n`
      ].join(' ')
      const successFormat = `${aegisAnsi.bgGreen} Aegis Agent completed ${aegisAnsi.reset} ${aegisAnsi.fgGreen}exit=%s${aegisAnsi.reset}\\n`
      const failedFormat = `${aegisAnsi.bgRed} Aegis Agent failed ${aegisAnsi.reset} ${aegisAnsi.fgRed}exit=%s${aegisAnsi.reset}\\n`
      const sentinelFormat = `${aegisAnsi.dim}%s=%s${aegisAnsi.reset}\\n`
      const wrapped = [
        'stty -echo 2>/dev/null || true',
        `printf ${quotePosixSingle(markerFormat)} ${quotePosixSingle(task.actor_name || 'Aegis')} ${quotePosixSingle(task.command)}`,
        task.command,
        '__aegis_exit=$?',
        `if [ "$__aegis_exit" -eq 0 ]; then printf ${quotePosixSingle(successFormat)} "$__aegis_exit"; else printf ${quotePosixSingle(failedFormat)} "$__aegis_exit"; fi`,
        `printf ${quotePosixSingle(sentinelFormat)} ${quotePosixSingle(sentinel)} "$__aegis_exit"`,
        'stty echo 2>/dev/null || true'
      ].join('\n')

      store.mcpSendTerminalCommand({
        tabId,
        command: wrapped,
        inputOnly: false
      })

      const idle = await store.mcpWaitForTerminalIdle({
        tabId,
        timeout: 45000,
        lines: 160,
        minWait: 1200
      })
      const parsed = store.parseAegisCommandOutput(idle.output || '', sentinel)
      const payload = {
        output: parsed.output,
        exit_code: parsed.exitCode,
        tab_id: tabId
      }
      if (idle.timedOut && parsed.exitCode === null) {
        payload.error = idle.message || 'terminal command timed out'
      }
      await window.pre.runGlobalAsync('aegisGatewayCompleteTerminalCommand', task.id, payload)
    } catch (err) {
      await window.pre.runGlobalAsync('aegisGatewayCompleteTerminalCommand', task.id, {
        error: err.message || String(err),
        tab_id: tabId || null
      })
    }
  }

  Store.prototype.resolveAegisTerminalTab = async function (hostId) {
    const { store } = window
    const host = store.aegisGatewayStatus.hosts.find(item => item.id === hostId)
    const current = store.currentTab
    if (tabMatchesAegisHost(current, hostId, host)) {
      return store.activeTabId
    }

    const existing = store.tabs.find(tab => tabMatchesAegisHost(tab, hostId, host))
    if (existing) {
      activateAegisTab(store, existing)
      return existing.id
    }

    if (isUsableSshTab(current)) {
      return store.activeTabId
    }

    const bookmark = store.bookmarks.find(item => item.id === hostId)
    if (!bookmark) {
      throw new Error(`No connected SSH terminal or bookmark for host: ${hostId}`)
    }
    if (bookmark.type !== 'ssh') {
      throw new Error(`Host ${hostId} is not an SSH bookmark and cannot run terminal commands`)
    }
    store.onSelectBookmark(hostId)
    await sleep(2500)
    if (!store.activeTabId) {
      throw new Error(`Failed to open SSH bookmark for host: ${hostId}`)
    }
    return store.activeTabId
  }

  Store.prototype.parseAegisCommandOutput = function (output, sentinel) {
    const exitPattern = new RegExp(`${escapeRegExp(sentinel)}=(\\d+)`)
    const match = output.match(exitPattern)
    const exitCode = match ? Number(match[1]) : null
    const cleaned = output
      .split('\n')
      .filter(line => !line.includes(`${sentinel}=`))
      .filter(line => !line.includes('stty -echo 2>/dev/null || true'))
      .filter(line => !line.includes('stty echo 2>/dev/null || true'))
      .join('\n')
      .trimEnd()
    return {
      output: cleaned,
      exitCode: Number.isFinite(exitCode) ? exitCode : null
    }
  }

  Store.prototype.decideAegisApproval = async function (approvalId, allow, modifiedCommand) {
    const payload = {
      allow,
      modified_command: modifiedCommand || null
    }
    await window.pre.runGlobalAsync('aegisGatewayDecideApproval', approvalId, payload)
    await window.store.refreshAegisGatewayStatus()
  }

  Store.prototype.pauseAegisSession = async function (sessionId) {
    await window.pre.runGlobalAsync('aegisGatewayPauseSession', sessionId)
    await window.store.refreshAegisGatewayStatus()
  }

  Store.prototype.resumeAegisSession = async function (sessionId) {
    await window.pre.runGlobalAsync('aegisGatewayResumeSession', sessionId)
    await window.store.refreshAegisGatewayStatus()
  }

  Store.prototype.setAegisSessionMode = async function (sessionId, mode) {
    await window.pre.runGlobalAsync('aegisGatewaySetSessionMode', sessionId, mode)
    await window.store.refreshAegisGatewayStatus()
  }

  Store.prototype.closeAegisSession = async function (sessionId) {
    await window.pre.runGlobalAsync('aegisGatewayCloseSession', sessionId)
    await window.store.refreshAegisGatewayStatus()
  }

  Store.prototype.setAegisPolicy = async function (payload) {
    await window.pre.runGlobalAsync('aegisGatewaySetPolicy', payload)
    await window.store.refreshAegisGatewayStatus()
  }

  Store.prototype.explainWithAi = function (txt) {
    const { store } = window
    store.handleOpenAIPanel()
    setTimeout(() => {
      refsStatic.get('AIChat')?.setPrompt(`explain terminal output: ${txt}`)
    }, 500)
    setTimeout(() => {
      refsStatic.get('AIChat')?.handleSubmit()
    }, 1200)
  }

  Store.prototype.runCommandInTerminal = function (cmd) {
    window.store.batchInputSelectedTabIds.forEach(id => {
      refs.get('term-' + id)?.runQuickCommand(cmd)
    })
  }

  Store.prototype.removeAiHistory = function (id) {
    const { store } = window
    const index = store.aiChatHistory.findIndex(d => d.id === id)
    if (index === -1) {
      return
    }
    window.store.aiChatHistory.splice(index, 1)
  }

  Store.prototype.getLangName = function (
    lang = window.store?.config.language || 'en_us'
  ) {
    return get(window.langMap, `[${lang}].name`)
  }

  Store.prototype.getLangNames = function () {
    return window.et.langs.map(d => d.name)
  }

  Store.prototype.fixProfiles = function () {
    const { profiles } = window.store
    const len = profiles.length
    let i = len - 1
    for (;i >= 0; i--) {
      const f = profiles[i]
      if (f.name) {
        continue
      }
      let count = 0
      let id = 'PROFILE' + i
      while (profiles.find(d => d.id === id)) {
        count = count + 1
        id = 'PROFILE' + count
      }
      const np = deepCopy(f)
      np.id = id
      np.name = id
      profiles.splice(i, 1, np)
    }
  }

  Store.prototype.makeSureProfileDefault = function (defaultId) {
    const { profiles } = window.store
    for (const p of profiles) {
      if (p.id !== defaultId) {
        delete p.isDefault
      }
    }
  }

  Store.prototype.aiConfigMissing = function () {
    return aiConfigsArr.filter(k => k !== 'apiKeyAI' && k !== 'proxyAI' && k !== 'nameAI').some(k => !window.store.config[k])
  }

  Store.prototype.clearHistory = function () {
    window.store.history = []
  }

  Store.prototype.addCmdHistory = action(function (cmd) {
    if (!cmd || !cmd.trim()) {
      return
    }
    const { terminalCommandHistory } = window.store
    const existing = terminalCommandHistory.find(item => item.cmd === cmd)
    if (existing) {
      existing.count = existing.count + 1
      existing.lastUseTime = new Date().toISOString()
    } else {
      terminalCommandHistory.push({
        id: uid(),
        cmd,
        count: 1,
        lastUseTime: new Date().toISOString()
      })
    }
    if (terminalCommandHistory.length > 200) {
      // Delete oldest 20 items when history exceeds 100
      terminalCommandHistory.sort((a, b) => new Date(a.lastUseTime).getTime() - new Date(b.lastUseTime).getTime())
      terminalCommandHistory.splice(0, 20)
    }
  })

  Store.prototype.deleteCmdHistory = function (cmd) {
    const { terminalCommandHistory } = window.store
    const idx = terminalCommandHistory.findIndex(item => item.cmd === cmd)
    if (idx !== -1) {
      terminalCommandHistory.splice(idx, 1)
    }
  }

  Store.prototype.clearAllCmdHistory = function () {
    window.store.terminalCommandHistory = []
  }

  Store.prototype.runCmdFromHistory = function (cmd) {
    window.store.runQuickCommand(cmd)
    window.store.addCmdHistory(cmd)
  }
}
