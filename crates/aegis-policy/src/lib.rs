use std::{fs, path::Path};

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandRule {
    pub name: String,
    pub risk: RiskLevel,
    pub reason: String,
    #[serde(default, alias = "needles")]
    pub patterns: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandRuleSet {
    pub rules: Vec<CommandRule>,
}

impl CommandRule {
    fn matches(&self, value: &str) -> bool {
        self.patterns
            .iter()
            .map(|pattern| pattern.trim())
            .filter(|pattern| !pattern.is_empty())
            .any(|pattern| value.contains(&pattern.to_ascii_lowercase()))
    }
}

pub fn classify_command(command: impl AsRef<str>) -> CommandAssessment {
    classify_command_with_rules(command, &default_rules())
}

pub fn classify_command_with_rules(
    command: impl AsRef<str>,
    rules: &[CommandRule],
) -> CommandAssessment {
    let command = command.as_ref().trim();
    let normalized = command.to_ascii_lowercase();

    let matched = rules.iter().find(|rule| rule.matches(&normalized));
    let (risk, reason, matched_rule) = matched
        .map(|rule| (rule.risk, rule.reason.as_str(), Some(rule.name.clone())))
        .unwrap_or((RiskLevel::Low, "未命中高危规则", None));

    CommandAssessment {
        command: command.to_owned(),
        risk,
        approval_required: matches!(risk, RiskLevel::High | RiskLevel::Critical),
        reason: reason.to_owned(),
        matched_rule,
    }
}

pub fn load_rules_from_yaml(path: impl AsRef<Path>) -> anyhow::Result<Vec<CommandRule>> {
    let path = path.as_ref();
    let content = fs::read_to_string(path)?;
    let rules: CommandRuleSet = serde_yaml::from_str(&content)?;
    Ok(normalize_rules(rules.rules))
}

pub fn default_rules() -> Vec<CommandRule> {
    normalize_rules(vec![
        CommandRule {
            name: "destructive-filesystem".to_owned(),
            risk: RiskLevel::Critical,
            reason: "命中破坏性文件系统命令规则".to_owned(),
            patterns: patterns(&["rm -rf", "mkfs", " dd ", " dd if=", " dd of="]),
        },
        CommandRule {
            name: "destructive-network-or-cluster".to_owned(),
            risk: RiskLevel::Critical,
            reason: "命中网络、集群或数据库破坏性操作规则".to_owned(),
            patterns: patterns(&[
                "iptables",
                " nft ",
                "drop database",
                "truncate table",
                "kubectl delete",
                "helm uninstall",
            ]),
        },
        CommandRule {
            name: "service-restart".to_owned(),
            risk: RiskLevel::High,
            reason: "命中服务或容器重启规则".to_owned(),
            patterns: patterns(&[
                "systemctl restart",
                "service restart",
                "docker restart",
                "docker compose restart",
                "supervisorctl restart",
            ]),
        },
        CommandRule {
            name: "recursive-permission-change".to_owned(),
            risk: RiskLevel::High,
            reason: "命中递归权限或属主变更规则".to_owned(),
            patterns: patterns(&[
                "chmod -r",
                "chmod --recursive",
                "chown -r",
                "chown --recursive",
            ]),
        },
        CommandRule {
            name: "environment-changing".to_owned(),
            risk: RiskLevel::Medium,
            reason: "命中会改变环境、依赖或运行状态的命令规则".to_owned(),
            patterns: patterns(&[
                "git pull",
                "npm install",
                "pnpm install",
                "yarn install",
                "pip install",
                "cargo install",
                "docker ps",
                "docker compose ps",
            ]),
        },
    ])
}

fn patterns(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

fn normalize_rules(rules: Vec<CommandRule>) -> Vec<CommandRule> {
    rules
        .into_iter()
        .filter_map(|mut rule| {
            rule.name = rule.name.trim().to_owned();
            rule.reason = rule.reason.trim().to_owned();
            rule.patterns = rule
                .patterns
                .into_iter()
                .map(|pattern| pattern.trim().to_owned())
                .filter(|pattern| !pattern.is_empty())
                .collect();
            if rule.name.is_empty() || rule.patterns.is_empty() {
                return None;
            }
            if rule.reason.is_empty() {
                rule.reason = format!("命中自定义规则: {}", rule.name);
            }
            Some(rule)
        })
        .collect()
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

    #[test]
    fn classifies_with_custom_rules() {
        let rules = vec![CommandRule {
            name: "custom-restart".to_owned(),
            risk: RiskLevel::Critical,
            reason: "custom reason".to_owned(),
            patterns: vec!["pm2 restart".to_owned()],
        }];
        let assessment = classify_command_with_rules("pm2 restart api", &rules);
        assert_eq!(assessment.risk, RiskLevel::Critical);
        assert_eq!(assessment.reason, "custom reason");
        assert_eq!(assessment.matched_rule.as_deref(), Some("custom-restart"));
    }

    #[test]
    fn parses_yaml_rules() {
        let rules: CommandRuleSet = serde_yaml::from_str(
            r#"
rules:
  - name: custom-delete
    risk: critical
    reason: custom delete rule
    patterns:
      - "redis-cli flushall"
"#,
        )
        .unwrap();
        let normalized = normalize_rules(rules.rules);
        let assessment = classify_command_with_rules("redis-cli flushall", &normalized);
        assert_eq!(assessment.risk, RiskLevel::Critical);
        assert_eq!(assessment.matched_rule.as_deref(), Some("custom-delete"));
    }
}
