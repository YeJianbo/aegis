import { useEffect } from 'react'
import { auto } from 'manate/react'
import {
  Alert,
  Button,
  Empty,
  Flex,
  Input,
  List,
  Modal,
  Space,
  Statistic,
  Tag,
  Tooltip
} from 'antd'
import {
  ApiOutlined,
  CheckCircleOutlined,
  ClockCircleOutlined,
  ReloadOutlined,
  SafetyCertificateOutlined,
  SettingOutlined
} from '@ant-design/icons'
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

export default auto(function AegisAgentPanel ({ rightPanelTab, visible }) {
  const { store } = window
  const status = store.aegisGatewayStatus

  useEffect(() => {
    if (visible || rightPanelTab === 'agent') {
      store.refreshAegisGatewayStatus()
    }
  }, [rightPanelTab, visible])

  if (!visible && rightPanelTab !== 'agent') {
    return null
  }

  const handleRefresh = () => store.refreshAegisGatewayStatus()
  const handleOpenWidgets = () => store.openWidgetsModal()
  const handleApproval = (item, allow, modifiedCommand) => {
    store.decideAegisApproval(item.id, allow, modifiedCommand).catch(store.onError)
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
      onOk: () => handleApproval(item, true, command)
    })
  }
  const handleSessionPauseToggle = session => {
    const action = session.paused ? store.resumeAegisSession : store.pauseAegisSession
    action.call(store, session.id).catch(store.onError)
  }
  const handleSessionMode = (session, mode) => {
    store.setAegisSessionMode(session.id, mode).catch(store.onError)
  }
  const pendingApprovals = status.approvals.filter(item => item.status === 'pending')
  const latestAudit = status.auditEvents.slice(-8).reverse()

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
        <Statistic title='Hosts' value={status.hosts.length} prefix={<ApiOutlined />} />
        <Statistic title='Sessions' value={status.sessions.length} prefix={<SafetyCertificateOutlined />} />
        <Statistic title='Pending' value={pendingApprovals.length} />
      </div>

      <section className='aegis-agent-section'>
        <h3>Approvals</h3>
        {
          pendingApprovals.length
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
                          type='primary'
                          onClick={() => handleApproval(item, true)}
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
                          onClick={() => handleApproval(item, false)}
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
      </section>

      <section className='aegis-agent-section'>
        <h3>Sessions</h3>
        {
          status.sessions.length
            ? (
              <List
                size='small'
                dataSource={status.sessions}
                renderItem={item => (
                  <List.Item>
                    <div className='aegis-agent-list-item'>
                      <Flex justify='space-between' align='center'>
                        <Tag color={item.paused ? 'orange' : 'green'}>
                          {item.paused ? 'paused' : 'running'}
                        </Tag>
                        <span className='aegis-agent-muted'>{item.mode}</span>
                      </Flex>
                      <div className='aegis-agent-command' title={item.title}>
                        {item.title}
                      </div>
                      <div className='aegis-agent-muted'>
                        {item.host_id}
                      </div>
                      <Space className='aegis-agent-actions' wrap>
                        <Button
                          size='small'
                          onClick={() => handleSessionPauseToggle(item)}
                        >
                          {item.paused ? 'Resume' : 'Pause'}
                        </Button>
                        <Button
                          size='small'
                          type={item.mode === 'agent_writable' ? 'primary' : 'default'}
                          onClick={() => handleSessionMode(item, 'agent_writable')}
                        >
                          Agent
                        </Button>
                        <Button
                          size='small'
                          type={item.mode === 'agent_read_only' ? 'primary' : 'default'}
                          onClick={() => handleSessionMode(item, 'agent_read_only')}
                        >
                          Read-only
                        </Button>
                        <Button
                          size='small'
                          type={item.mode === 'human_only' ? 'primary' : 'default'}
                          onClick={() => handleSessionMode(item, 'human_only')}
                        >
                          Human
                        </Button>
                      </Space>
                    </div>
                  </List.Item>
                )}
              />
              )
            : <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description='No sessions yet' />
        }
      </section>

      <section className='aegis-agent-section'>
        <h3>Audit Timeline</h3>
        {
          latestAudit.length
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
      </section>
    </div>
  )
})
