pub const DEFAULT_SSH_AGENT_NAME: &str = "ssh-agent.service";
pub const DEFAULT_SSH_AGENT_SOCKET: &str = "ssh-agent.socket";

pub const UNIT_SYSTEMD_NAME: &str = "bw-ssh-agent.service";

pub const UNIT_SERVICE_TEMPLATE: &str = include_str!("bw-ssh-agent.service");

pub const ENVIRONMENT_SSH_AGENT_NAME: &str = "10-bw-ssh-agent.conf";
pub const ENVIRONMENT_SSH_AGENT_TEMPLATE: &str = include_str!("bw-ssh-agent.conf");

pub const PLASMA_WORKSPACE_ENV_SCRIPT_PATH: &str = "/etc/xdg/plasma-workspace/env/ssh-agent.sh";
pub const PLASMA_WORKSPACE_ENV_SCRIPT_PATH_DISABLED: &str = "/etc/xdg/plasma-workspace/env/ssh-agent.sh.disabled";