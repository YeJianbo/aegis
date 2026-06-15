import { useEffect, useState } from 'react'
import { auto } from 'manate/react'
import {
  Alert,
  Button,
  Collapse,
  Empty,
  Flex,
  Input,
  List,
  Modal,
  Popover,
  Segmented,
  Space,
  Statistic,
  Tag,
  Tooltip
} from 'antd'
import {
  ApiOutlined,
  CheckCircleOutlined,
  ClockCircleOutlined,
  HistoryOutlined,
  ReloadOutlined,
  SafetyCertificateOutlined,
  SettingOutlined
} from '@ant-design/icons'
import {
  connectionMap,
  statusMap
} from '../../common/constants'
import './aegis-agent.styl'

function riskColor (risk) {
  if (risk === 'critical') return 'red'
  if (risk === 'high') return 'orange'
  if (risk === 'medium') return 'gold'
  return 'green'
}

function fmtCommand (command) {
  if (!command) return ''
  return command.length > 90 ? command.slice(0, 87) + '...' : command
}

function policyToDraft (policy) {
  return {
    mode: policy?.mode || 'guarded',
    whitelist: (policy?.whitelist || []).join('\n'),
    blacklist: (policy?.blacklist || []).join('\n')
  }
}

function splitPatterns (value) {
  return String(value || '')
    .split(/[\n,]/)
    .map(item => item.trim())
    .filter(Boolean)
}

function tabAddress (tab) {
  return tab?.port ? `${tab.host}:${tab.port}` : tab?.host
}

function tabDisplayName (tab) {
  return tab?.title || [tab?.username, tabAddress(tab)].filter(Boolean).join('@') || tab?.host || tab?.id
}

function isConnectedAegisTab (tab) {
  return !!(
    tab?.host &&
    tab.status === statusMap.success &&
    (tab.type === connectionMap.ssh || tab.type === connectionMap.ftp)
  )
}

function findHostForTab (tab, hosts) {
  if (tab?.srcId) {
    const sourceHost = hosts.find(host => host.id === tab.srcId)
    if (sourceHost) return sourceHost
  }
  if (tab?.id) {
    const idHost = hosts.find(host => host.id === tab.id)
    if (idHost) return idHost
  }
  const address = tabAddress(tab)
  const hostOnly = String(tab?.host || '')
  return hosts.find(host => (
    host.address === address ||
    String(host.address || '').split(':')[0] === hostOnly
  ))
}

export default auto(function AegisAgentPanel ({ rightPanelTab, visible }) {
  const { store } = window
  const status = store.aegisGatewayStatus
  const [policyDraft, setPolicyDraft] = useState(policyToDraft(status.policy))
  const [reviewApproval, setReviewApproval] = useState(null)

  useEffect(() => {
    if (visible || rightPanelTab === 'agent') {
      store.refreshAegisGatewayStatus()
    }
  }, [rightPanelTab, visible])

  useEffect(() => {
    setPolicyDraft(policyToDraft(status.policy))
  }, [status.policy?.mode, status.policy?.whitelist?.join('\n'), status.policy?.blacklist?.join('\n')])

  if (!visible && rightPanelTab !== 'agent') {
    return null
  }

  const handleRefresh = () => store.refreshAegisGatewayStatus()
  const handleOpenWidgets = () => store.openWidgetsModal('aegis-gateway')
  const handleApproval = (item, allow, modifiedCommand) => {
    store.decideAegisApproval(item.id, allow, modifiedCommand).catch(store.onError)
  }
  const closeReviewApproval = () => setReviewApproval(null)
  const handleReviewApproval = item => setReviewApproval(item)
  const handleReviewDecision = (item, allow, modifiedCommand) => {
    closeReviewApproval()
    handleApproval(item, allow, modifiedCommand)
  }
  const handleModifyApproval = item => {
    let command = item.command
    Modal.confirm({
      title: 'Modify Command',
      content: (
        <Input.TextArea
          defaultValue={item.command}
          autoSize={{ minRows: 3, maxRows: 8 }}
          onChange={event => {
            command = event.target.value
          }}
        />
      ),
      okText: 'Allow Modified',
      cancelText: 'Cancel',
      onOk: () => handleReviewDecision(item, true, command)
    })
  }
  const handleWhitelistApproval = item => {
    const command = String(item.command || '').trim()
    if (!command) return
    const policy = status.policy || {}
    const whitelist = Array.from(new Set([
      ...(policy.whitelist || []),
      command
    ]))
    closeReviewApproval()
    store.setAegisPolicy({
      mode: policy.mode || 'guarded',
      whitelist,
      blacklist: policy.blacklist || []
    })
      .then(() => store.decideAegisApproval(item.id, true))
      .catch(store.onError)
  }
  const handleSessionPauseToggle = session => {
    const action = session.paused ? store.resumeAegisSession : store.pauseAegisSession
    action.call(store, session.id).catch(store.onError)
  }
  const handleSessionMode = (session, mode) => {
    store.setAegisSessionMode(session.id, mode).catch(store.onError)
  }
  const handleCloseSession = session => {
    Modal.confirm({
      title: 'Close Session',
      content: session.title || session.id,
      okText: 'Close',
      okButtonProps: { danger: true },
      onOk: () => store.closeAegisSession(session.id).catch(store.onError)
    })
  }
  const handlePolicySave = () => {
    store.setAegisPolicy({
      mode: policyDraft.mode,
      whitelist: splitPatterns(policyDraft.whitelist),
      blacklist: splitPatterns(policyDraft.blacklist)
    }).catch(store.onError)
  }
  const pendingApprovals = status.approvals.filter(item => item.status === 'pending')
  const latestAudit = status.auditEvents.slice(-8).reverse()
  const usedSessionIds = new Set()
  const connectedRows = store.tabs
    .filter(isConnectedAegisTab)
    .map(tab => {
      const host = findHostForTab(tab, status.hosts)
      const hostId = host?.id || tab.srcId || tab.id
      const session = status.sessions.find(item => (
        !usedSessionIds.has(item.id) && item.host_id === hostId
      ))
      if (session) {
        usedSessionIds.add(session.id)
      }
      return {
        tab,
        host,
        hostId,
        session
      }
    })
  const historicalSessions = status.sessions.filter(item => !usedSessionIds.has(item.id))

  function findSessionById (sessionId) {
    return status.sessions.find(item => item.id === sessionId)
  }

  function findHostById (hostId) {
    return status.hosts.find(item => item.id === hostId)
  }

  function renderSessionControls (session) {
    return (
      <Space className='aegis-agent-actions' wrap>
        <Button
          size='small'
          onClick={() => handleSessionPauseToggle(session)}
        >
          {session.paused ? 'Resume' : 'Pause'}
        </Button>
        <Segmented
          size='small'
          value={session.mode}
          options={[
            { label: 'Agent', value: 'agent_writable' },
            { label: 'Read-only', value: 'agent_read_only' },
            { label: 'Human', value: 'human_only' }
          ]}
          onChange={mode => handleSessionMode(session, mode)}
        />
        <Button
          size='small'
          danger
          onClick={() => handleCloseSession(session)}
        >
          Close
        </Button>
      </Space>
    )
  }

  function renderHistoryPopover () {
    return (
      <div className='aegis-agent-history-popover'>
        {
          historicalSessions.length
            ? (
              <List
                size='small'
                dataSource={historicalSessions}
                renderItem={item => (
                  <List.Item className='aegis-agent-history-item'>
                    <div className='aegis-agent-list-item'>
                      <Flex justify='space-between' align='center'>
                        <Tag color={item.paused ? 'orange' : 'default'}>
                          {item.paused ? 'paused' : 'history'}
                        </Tag>
                        <span className='aegis-agent-muted'>{item.mode}</span>
                      </Flex>
                      <div className='aegis-agent-command' title={item.title}>
                        {item.title}
                      </div>
                      <Flex justify='space-between' align='center'>
                        <span className='aegis-agent-muted'>{item.host_id}</span>
                        <Button
                          size='small'
                          danger
                          onClick={() => handleCloseSession(item)}
                        >
                          Remove
                        </Button>
                      </Flex>
                    </div>
                  </List.Item>
                )}
              />
              )
            : <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description='No historical sessions' />
        }
      </div>
    )
  }

  function renderPolicyPopover () {
    return (
      <div className='aegis-agent-policy-popover'>
        <div className='aegis-agent-policy'>
          <Segmented
            block
            size='small'
            value={policyDraft.mode}
            options={[
              { label: 'Guarded', value: 'guarded' },
              { label: 'Full allow', value: 'full_allow' }
            ]}
            onChange={mode => setPolicyDraft({ ...policyDraft, mode })}
          />
          <Input.TextArea
            className='aegis-agent-policy-input'
            value={policyDraft.whitelist}
            placeholder='Whitelist patterns, one per line'
            autoSize={{ minRows: 2, maxRows: 5 }}
            onChange={event => setPolicyDraft({
              ...policyDraft,
              whitelist: event.target.value
            })}
          />
          <Input.TextArea
            className='aegis-agent-policy-input'
            value={policyDraft.blacklist}
            placeholder='Blacklist patterns, one per line'
            autoSize={{ minRows: 2, maxRows: 5 }}
            onChange={event => setPolicyDraft({
              ...policyDraft,
              blacklist: event.target.value
            })}
          />
          <Button size='small' type='primary' block onClick={handlePolicySave}>
            Save Policy
          </Button>
        </div>
      </div>
    )
  }

  function renderApprovalsContent () {
    return pendingApprovals.length
      ? (
        <List
          size='small'
          dataSource={pendingApprovals}
          renderItem={item => (
            <List.Item>
              <div className='aegis-agent-list-item'>
                <Flex justify='space-between' align='center'>
                  <Tag color={riskColor(item.assessment.risk)}>
                    {item.assessment.risk}
                  </Tag>
                  <span className='aegis-agent-muted'>{item.actor_name}</span>
                </Flex>
                <div className='aegis-agent-command' title={item.command}>
                  {fmtCommand(item.command)}
                </div>
                <div className='aegis-agent-muted'>
                  {item.assessment.reason}
                </div>
                <Space className='aegis-agent-actions'>
                  <Button
                    size='small'
                    onClick={() => handleReviewApproval(item)}
                  >
                    Review
                  </Button>
                  <Button
                    size='small'
                    type='primary'
                    onClick={() => handleReviewDecision(item, true)}
                  >
                    Allow
                  </Button>
                  <Button
                    size='small'
                    onClick={() => handleModifyApproval(item)}
                  >
                    Modify
                  </Button>
                  <Button
                    size='small'
                    danger
                    onClick={() => handleReviewDecision(item, false)}
                  >
                    Deny
                  </Button>
                </Space>
              </div>
            </List.Item>
          )}
        />
        )
      : <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description='No pending approvals' />
  }

  function renderAuditContent () {
    return latestAudit.length
      ? (
        <List
          size='small'
          dataSource={latestAudit}
          renderItem={item => (
            <List.Item>
              <div className='aegis-agent-list-item'>
                <Flex justify='space-between' align='center'>
                  <Tag color={riskColor(item.risk)}>{item.risk}</Tag>
                  <span className='aegis-agent-muted'>{item.approval_status}</span>
                </Flex>
                <div className='aegis-agent-command' title={item.command}>
                  {fmtCommand(item.command)}
                </div>
                <div className='aegis-agent-muted'>
                  {item.output_summary || 'No output summary'}
                </div>
              </div>
            </List.Item>
          )}
        />
        )
      : <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description='No audit events yet' />
  }

  function renderCollapseLabel (title, count) {
    return (
      <span className='aegis-agent-collapse-label'>
        <span>{title}</span>
        <Tag>{count}</Tag>
      </span>
    )
  }

  function renderApprovalReviewModal () {
    if (!reviewApproval) {
      return null
    }
    const session = findSessionById(reviewApproval.session_id)
    const host = findHostById(session?.host_id)
    const assessment = reviewApproval.assessment || {}
    return (
      <Modal
        open
        title='Review Approval'
        onCancel={closeReviewApproval}
        footer={[
          <Button key='deny' danger onClick={() => handleReviewDecision(reviewApproval, false)}>
            Deny
          </Button>,
          <Button key='modify' onClick={() => handleModifyApproval(reviewApproval)}>
            Modify
          </Button>,
          <Button key='whitelist' onClick={() => handleWhitelistApproval(reviewApproval)}>
            Whitelist & Allow
          </Button>,
          <Button key='allow' type='primary' onClick={() => handleReviewDecision(reviewApproval, true)}>
            Allow once
          </Button>
        ]}
      >
        <div className='aegis-agent-approval-review'>
          <Flex justify='space-between' align='center'>
            <Tag color={riskColor(assessment.risk)}>{assessment.risk}</Tag>
            <span className='aegis-agent-muted'>{reviewApproval.actor_name}</span>
          </Flex>
          <div className='aegis-agent-review-command'>
            {reviewApproval.command}
          </div>
          <div className='aegis-agent-review-grid'>
            <span>Host</span>
            <strong>{host?.name || session?.host_id || 'Unknown'}</strong>
            <span>Address</span>
            <strong>{host?.address || 'Unknown'}</strong>
            <span>Session</span>
            <strong>{session?.title || reviewApproval.session_id}</strong>
            <span>Rule</span>
            <strong>{assessment.matched_rule || 'policy'}</strong>
            <span>Reason</span>
            <strong>{assessment.reason || 'No reason provided'}</strong>
          </div>
        </div>
      </Modal>
    )
  }

  return (
    <div className='aegis-agent-panel'>
      <Flex justify='space-between' align='center' className='aegis-agent-toolbar'>
        <Space>
          <Tag
            color={status.online ? 'green' : 'red'}
            icon={status.online ? <CheckCircleOutlined /> : <ClockCircleOutlined />}
          >
            {status.online ? 'Online' : 'Offline'}
          </Tag>
          <span className='aegis-agent-url'>127.0.0.1:17321</span>
        </Space>
        <Space>
          <Popover
            trigger='click'
            placement='bottomRight'
            title='Policy'
            content={renderPolicyPopover()}
          >
            <Tooltip title='Policy'>
              <Button
                icon={<SafetyCertificateOutlined />}
                size='small'
              />
            </Tooltip>
          </Popover>
          <Popover
            trigger='click'
            placement='bottomRight'
            title={(
              <Flex justify='space-between' align='center' className='aegis-agent-popover-title'>
                <span>History</span>
                <Tag>{historicalSessions.length}</Tag>
              </Flex>
            )}
            content={renderHistoryPopover()}
          >
            <Tooltip title='History'>
              <Button
                icon={<HistoryOutlined />}
                size='small'
              />
            </Tooltip>
          </Popover>
          <Tooltip title='Refresh'>
            <Button
              icon={<ReloadOutlined />}
              size='small'
              loading={status.loading}
              onClick={handleRefresh}
            />
          </Tooltip>
          <Tooltip title='Open Widgets'>
            <Button
              icon={<SettingOutlined />}
              size='small'
              onClick={handleOpenWidgets}
            />
          </Tooltip>
        </Space>
      </Flex>

      {
        status.error
          ? (
            <Alert
              className='aegis-agent-alert'
              type='warning'
              showIcon
              message='Gateway is not reachable'
              description='Start the Aegis Gateway widget, then refresh this panel.'
            />
            )
          : null
      }
      {
        !status.error && status.hosts.length === 0
          ? (
            <Alert
              className='aegis-agent-alert'
              type='info'
              showIcon
              message='No SSH bookmarks synced'
              description='Create or import SSH bookmarks in electerm, then refresh this panel.'
            />
            )
          : null
      }

      <div className='aegis-agent-metrics'>
        <Statistic title='Connected' value={connectedRows.length} prefix={<ApiOutlined />} />
        <Statistic title='History' value={historicalSessions.length} />
        <Statistic title='Pending' value={pendingApprovals.length} />
      </div>

      <section className='aegis-agent-section'>
        <h3>Current Connections</h3>
        {
          connectedRows.length
            ? (
              <List
                size='small'
                dataSource={connectedRows}
                renderItem={row => (
                  <List.Item className='aegis-agent-compact-item'>
                    <div className='aegis-agent-list-item'>
                      <Flex justify='space-between' align='center'>
                        <Tag color='green'>
                          connected
                        </Tag>
                        <span className='aegis-agent-muted'>
                          {row.session ? row.session.mode : 'no agent session'}
                        </span>
                      </Flex>
                      <div className='aegis-agent-command' title={tabDisplayName(row.tab)}>
                        {tabDisplayName(row.tab)}
                      </div>
                      <div className='aegis-agent-muted'>
                        {[row.hostId, tabAddress(row.tab)].filter(Boolean).join(' · ')}
                      </div>
                      <Space className='aegis-agent-actions' wrap>
                        <Button
                          size='small'
                          onClick={() => store.changeActiveTabId(row.tab.id)}
                        >
                          Focus
                        </Button>
                        {row.session ? renderSessionControls(row.session) : null}
                      </Space>
                    </div>
                  </List.Item>
                )}
              />
              )
            : <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description='No connected Aegis tabs' />
        }
      </section>

      <Collapse
        ghost
        size='small'
        expandIconPosition='end'
        className='aegis-agent-compact-collapse'
        defaultActiveKey={pendingApprovals.length ? ['approvals'] : []}
        items={[
          {
            key: 'approvals',
            label: renderCollapseLabel('Approvals', pendingApprovals.length),
            children: renderApprovalsContent()
          },
          {
            key: 'audit',
            label: renderCollapseLabel('Audit Timeline', latestAudit.length),
            children: renderAuditContent()
          }
        ]}
      />
      {renderApprovalReviewModal()}
    </div>
  )
})
