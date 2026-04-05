use std::env;
use std::io::Write;
use anyhow::anyhow;
use tokio::process::Command;
use zbus::Connection;
use crate::systemd_manager::SystemdManagerProxy;

const DEFAULT_SSH_AGENT_NAME: &str = "ssh-agent.service";
const DEFAULT_SSH_AGENT_SOCKET: &str = "ssh-agent.socket";

const UNIT_SYSTEMD_NAME: &str = "bw-ssh-agent.service";

const UNIT_SERVICE_TEMPLATE: &str = include_str!("bw-ssh-agent.service");

const ENVIRONMENT_SSH_AGENT_NAME: &str = "10-bw-ssh-agent.conf";
const ENVIRONMENT_SSH_AGENT_TEMPLATE: &str = include_str!("bw-ssh-agent.conf");

async fn install_systemd_service() -> anyhow::Result<()> {
    let config_dir = dirs_next::config_dir().expect("config dir not found");

    let systemd_user_dir = config_dir.join("systemd/user");

    if !systemd_user_dir.exists() {
        std::fs::create_dir_all(&systemd_user_dir)?;
    }

    let unit_file_path = systemd_user_dir.join(UNIT_SYSTEMD_NAME);
    let mut unit_file = std::fs::File::create(&unit_file_path)?;

    let executable_path = env::current_exe()?;

    let unit = UNIT_SERVICE_TEMPLATE.replace("$BIN_PATH$", executable_path.to_str().unwrap());

    unit_file.write_all(unit.as_bytes())?;

    log::info!("Wrote systemd unit to {}", unit_file_path.display());

    Ok(())
}

async fn write_systemd_env_file() -> anyhow::Result<()> {
    let config_dir = dirs_next::config_dir().expect("config dir not found");

    let environment_dir = config_dir.join("environment.d");

    if !environment_dir.exists() {
        std::fs::create_dir_all(&environment_dir)?;
    }

    let file_path = environment_dir.join(ENVIRONMENT_SSH_AGENT_NAME);
    let mut file = std::fs::File::create(&file_path)?;

    file.write_all(ENVIRONMENT_SSH_AGENT_TEMPLATE.as_bytes())?;

    log::info!("Wrote environment file to {}", file_path.display());

    Ok(())
}

async fn enable_and_restart_systemd_service() -> anyhow::Result<()> {
    let connection = Connection::session().await?;
    let proxy = SystemdManagerProxy::new(&connection).await?;
    let _ = proxy.reload().await?;
    log::info!("Reloaded systemd manager");

    let r = proxy.enable_unit_files(Vec::from(&[UNIT_SYSTEMD_NAME]), false, true).await?;
    if !r.0 {
        log::error!("Failed to enable service");
        return Ok(());
    }

    let r = proxy.restart_unit(UNIT_SYSTEMD_NAME, "replace").await?;
    log::info!("Started bw-ssh-agent.service: {}", r);

    Ok(())
}

async fn remove_plasma_workspace_ssh_env() -> anyhow::Result<()> {
    let path = std::path::Path::new("/etc/xdg/plasma-workspace/env/ssh-agent.sh");

    if !path.exists() {
        return Ok(());
    }

    log::info!("Removing plasma workspace ssh env");

    if std::fs::remove_file(path).is_ok() {
        return Ok(());
    }

    let status = Command::new("pkexec")
        .arg("rm")
        .arg("-f")
        .arg(path)
        .status()
        .await?;

    if status.success() {
        Ok(())
    } else {
        Err(anyhow!("Failed to delete ssh-agent env file"))
    }
}

async fn disable_default_ssh_agent() -> anyhow::Result<()> {
    remove_plasma_workspace_ssh_env().await?;

    let connection = Connection::session().await?;
    let proxy = SystemdManagerProxy::new(&connection).await?;

    let _ = proxy.stop_unit(DEFAULT_SSH_AGENT_SOCKET, "replace").await;
    let _ = proxy.stop_unit(DEFAULT_SSH_AGENT_NAME, "replace").await;

    proxy
        .disable_unit_files(vec![DEFAULT_SSH_AGENT_SOCKET, DEFAULT_SSH_AGENT_NAME], false)
        .await?;

    proxy
        .mask_unit_files(
            vec![DEFAULT_SSH_AGENT_SOCKET, DEFAULT_SSH_AGENT_NAME],
            false,
            true,
        )
        .await?;

    Ok(())
}

pub async fn install(running_in_systemd: bool) -> anyhow::Result<()> {
    if running_in_systemd {
        disable_default_ssh_agent().await?;
        install_systemd_service().await?;
        enable_and_restart_systemd_service().await?;

        write_systemd_env_file().await?;
    } else {
        log::error!("Could not find systemd. Unable to automatically install service");
    }

    Ok(())
}