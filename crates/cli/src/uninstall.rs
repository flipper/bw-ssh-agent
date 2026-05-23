use anyhow::anyhow;
use tokio::process::Command;
use zbus::Connection;
use crate::consts::{DEFAULT_SSH_AGENT_NAME, DEFAULT_SSH_AGENT_SOCKET, ENVIRONMENT_SSH_AGENT_NAME, PLASMA_WORKSPACE_ENV_SCRIPT_PATH, PLASMA_WORKSPACE_ENV_SCRIPT_PATH_DISABLED, UNIT_SYSTEMD_NAME};
use crate::systemd_manager::SystemdManagerProxy;

async fn uninstall_systemd_service() -> anyhow::Result<()> {
    let config_dir = dirs_next::config_dir().expect("config dir not found");

    let systemd_user_dir = config_dir.join("systemd/user");

    if !systemd_user_dir.exists() {
        std::fs::create_dir_all(&systemd_user_dir)?;
    }

    let unit_file_path = systemd_user_dir.join(UNIT_SYSTEMD_NAME);

    if !unit_file_path.exists() {
        return Ok(())
    }

    let _ = std::fs::remove_file(&unit_file_path);

    log::info!("Removed systemd unit from {}", unit_file_path.display());

    let connection = Connection::session().await?;
    let proxy = SystemdManagerProxy::new(&connection).await?;
    let _ = proxy.reload().await?;
    log::info!("Reloaded systemd manager");

    Ok(())
}

async fn delete_systemd_env_file() -> anyhow::Result<()> {
    let config_dir = dirs_next::config_dir().expect("config dir not found");

    let environment_dir = config_dir.join("environment.d");

    if !environment_dir.exists() {
        std::fs::create_dir_all(&environment_dir)?;
    }

    let file_path = environment_dir.join(ENVIRONMENT_SSH_AGENT_NAME);

    if !file_path.exists() {
        return Ok(())
    }

    log::info!("Removed environment file {}", file_path.display());

    let _ = std::fs::remove_file(&file_path)?;

    Ok(())
}

async fn enable_plasma_workspace_ssh_env() -> anyhow::Result<()> {
    let path = std::path::Path::new(PLASMA_WORKSPACE_ENV_SCRIPT_PATH_DISABLED);
    let new_path = std::path::Path::new(PLASMA_WORKSPACE_ENV_SCRIPT_PATH);

    if !path.exists() {
        return Ok(());
    }

    log::info!("Enabling plasma workspace ssh env");

    if std::fs::rename(path, new_path).is_ok() {
        return Ok(());
    }

    let status = Command::new("pkexec")
        .arg("mv")
        .arg(path)
        .arg(new_path)
        .status()
        .await?;

    if status.success() {
        Ok(())
    } else {
        Err(anyhow!("Failed to enable ssh-agent env file"))
    }
}

async fn enable_default_ssh_agent() -> anyhow::Result<()> {
    enable_plasma_workspace_ssh_env().await?;

    let connection = Connection::session().await?;
    let proxy = SystemdManagerProxy::new(&connection).await?;

    log::info!("Unmasking {:?}", vec![DEFAULT_SSH_AGENT_SOCKET, DEFAULT_SSH_AGENT_NAME]);

    proxy
        .unmask_unit_files(
            vec![DEFAULT_SSH_AGENT_SOCKET, DEFAULT_SSH_AGENT_NAME],
            false,
        )
        .await?;

    log::info!("Enabling {:?}", vec![DEFAULT_SSH_AGENT_SOCKET, DEFAULT_SSH_AGENT_NAME]);

    proxy
        .enable_unit_files(vec![DEFAULT_SSH_AGENT_SOCKET, DEFAULT_SSH_AGENT_NAME], false, true)
        .await?;


    log::info!("Starting {}", DEFAULT_SSH_AGENT_SOCKET);
    let _ = proxy.restart_unit(DEFAULT_SSH_AGENT_SOCKET, "replace").await;
    log::info!("Starting {}", DEFAULT_SSH_AGENT_NAME);
    let _ = proxy.restart_unit(DEFAULT_SSH_AGENT_NAME, "replace").await;

    Ok(())
}

pub async fn uninstall(running_in_systemd: bool) -> anyhow::Result<()> {
    if running_in_systemd {
        delete_systemd_env_file().await?;
        uninstall_systemd_service().await?;
        enable_default_ssh_agent().await?;

    } else {
        log::error!("Could not find systemd. Unable to automatically uninstall service");
    }

    Ok(())
}