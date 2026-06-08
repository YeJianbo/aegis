use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandAssessment {
    pub command: String,
    pub risk: RiskLevel,
    pub approval_required: bool,
    pub reason: String,
    pub matched_rule: Option<String>,
}

pub fn classify_command(command: impl AsRef<str>) -> CommandAssessment {
    let command = command.as_ref().trim();
    let normalized = command.to_ascii_lowercase();

    let matched = default_rules()
        .into_iter()
        .find(|rule| rule.matches(&normalized));
    let (risk, reason, matched_rule) = matched
        .map(|rule| (rule.risk, rule.reason, Some(rule.name.to_owned())))
        .unwrap_or((RiskLevel::Low, "未命中高危规则", None));

    CommandAssessment {
        command: command.to_owned(),
        risk,
        approval_required: matches!(risk, RiskLevel::High | RiskLevel::Critical),
        reason: reason.to_owned(),
        matched_rule,
    }
}

#[derive(Debug, Clone, Copy)]
struct PolicyRule {
    name: &'static str,
    risk: RiskLevel,
    reason: &'static str,
    needles: &'static [&'static str],
}

impl PolicyRule {
    fn matches(&self, value: &str) -> bool {
        self.needles.iter().any(|needle| value.contains(needle))
    }
}

fn default_rules() -> Vec<PolicyRule> {
    vec![
        PolicyRule {
            name: "destructive-filesystem",
            risk: RiskLevel::Critical,
            reason: "命中破坏性文件系统命令规则",
            needles: &["rm -rf", "mkfs", " dd ", " dd if=", " dd of="],
        },
        PolicyRule {
            name: "destructive-network-or-cluster",
            risk: RiskLevel::Critical,
            reason: "命中网络、集群或数据库破坏性操作规则",
            needles: &[
                "iptables",
                " nft ",
                "drop database",
                "truncate table",
                "kubectl delete",
                "helm uninstall",
            ],
        },
        PolicyRule {
            name: "service-restart",
            risk: RiskLevel::High,
            reason: "命中服务或容器重启规则",
            needles: &[
                "systemctl restart",
                "service restart",
                "docker restart",
                "docker compose restart",
                "supervisorctl restart",
            ],
        },
        PolicyRule {
            name: "recursive-permission-change",
            risk: RiskLevel::High,
            reason: "命中递归权限或属主变更规则",
            needles: &[
                "chmod -r",
                "chmod --recursive",
                "chown -r",
                "chown --recursive",
            ],
        },
        PolicyRule {
            name: "environment-changing",
            risk: RiskLevel::Medium,
            reason: "命中会改变环境、依赖或运行状态的命令规则",
            needles: &[
                "git pull",
                "npm install",
                "pnpm install",
                "yarn install",
                "pip install",
                "cargo install",
                "docker ps",
                "docker compose ps",
            ],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_low_risk_command() {
        let assessment = classify_command("git status");
        assert_eq!(assessment.risk, RiskLevel::Low);
        assert!(!assessment.approval_required);
        assert_eq!(assessment.matched_rule, None);
    }

    #[test]
    fn classifies_high_risk_command() {
        let assessment = classify_command("systemctl restart nginx");
        assert_eq!(assessment.risk, RiskLevel::High);
        assert!(assessment.approval_required);
        assert_eq!(assessment.matched_rule.as_deref(), Some("service-restart"));
    }

    #[test]
    fn classifies_critical_command() {
        let assessment = classify_command("rm -rf /var/www/app");
        assert_eq!(assessment.risk, RiskLevel::Critical);
        assert!(assessment.approval_required);
        assert_eq!(
            assessment.matched_rule.as_deref(),
            Some("destructive-filesystem")
        );
    }

    #[test]
    fn classifies_database_drop_as_critical() {
        let assessment = classify_command("mysql -e 'DROP DATABASE prod'");
        assert_eq!(assessment.risk, RiskLevel::Critical);
        assert!(assessment.approval_required);
    }

    #[test]
    fn classifies_dependency_install_as_medium() {
        let assessment = classify_command("npm install");
        assert_eq!(assessment.risk, RiskLevel::Medium);
        assert!(!assessment.approval_required);
    }
}
