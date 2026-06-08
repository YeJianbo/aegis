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
}

pub fn classify_command(command: impl AsRef<str>) -> CommandAssessment {
    let command = command.as_ref().trim();
    let normalized = command.to_ascii_lowercase();

    let (risk, reason) = if contains_any(
        &normalized,
        &[
            "rm -rf",
            "mkfs",
            " dd ",
            "iptables",
            "drop database",
            "kubectl delete",
        ],
    ) {
        (RiskLevel::Critical, "命中破坏性命令规则")
    } else if contains_any(
        &normalized,
        &[
            "systemctl restart",
            "docker restart",
            "chmod -r",
            "chown -r",
        ],
    ) {
        (RiskLevel::High, "命中服务重启或递归权限变更规则")
    } else if contains_any(
        &normalized,
        &["git pull", "npm install", "pip install", "docker ps"],
    ) {
        (RiskLevel::Medium, "命中会改变环境或依赖状态的命令规则")
    } else {
        (RiskLevel::Low, "未命中高危规则")
    };

    CommandAssessment {
        command: command.to_owned(),
        risk,
        approval_required: matches!(risk, RiskLevel::High | RiskLevel::Critical),
        reason: reason.to_owned(),
    }
}

fn contains_any(value: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| value.contains(needle))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_low_risk_command() {
        let assessment = classify_command("git status");
        assert_eq!(assessment.risk, RiskLevel::Low);
        assert!(!assessment.approval_required);
    }

    #[test]
    fn classifies_high_risk_command() {
        let assessment = classify_command("systemctl restart nginx");
        assert_eq!(assessment.risk, RiskLevel::High);
        assert!(assessment.approval_required);
    }

    #[test]
    fn classifies_critical_command() {
        let assessment = classify_command("rm -rf /var/www/app");
        assert_eq!(assessment.risk, RiskLevel::Critical);
        assert!(assessment.approval_required);
    }
}
