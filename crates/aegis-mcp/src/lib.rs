use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolDescriptor {
    pub name: String,
    pub description: String,
}

pub fn default_tools() -> Vec<ToolDescriptor> {
    vec![
        tool("list_hosts", "列出当前用户可见的远程主机"),
        tool("open_session", "打开或复用一个受控远程终端会话"),
        tool("run_command", "在受控会话中执行命令并返回输出"),
        tool("tail_log", "读取远程日志尾部内容"),
        tool("get_terminal_snapshot", "获取终端当前可见快照"),
    ]
}

fn tool(name: &str, description: &str) -> ToolDescriptor {
    ToolDescriptor {
        name: name.to_owned(),
        description: description.to_owned(),
    }
}
