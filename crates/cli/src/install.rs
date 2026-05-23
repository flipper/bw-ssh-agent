use std::env;
use std::io::Write;
use anyhow::anyhow;
use tokio::process::Command;
use zbus::Connection;
use crate::consts::{DEFAULT_SSH_AGENT_NAME, DEFAULT_SSH_AGENT_SOCKET, ENVIRONMENT_SSH_AGENT_NAME, ENVIRONMENT_SSH_AGENT_TEMPLATE, PLASMA_WORKSPACE_ENV_SCRIPT_PATH, PLASMA_WORKSPACE_ENV_SCRIPT_PATH_DISABLED, UNIT_SERVICE_TEMPLATE, UNIT_SYSTEMD_NAME};
use crate::systemd_manager::SystemdManagerProxy;

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

async fn disable_plasma_workspace_ssh_env() -> anyhow::Result<()> {
    let path = std::path::Path::new(PLASMA_WORKSPACE_ENV_SCRIPT_PATH);
    let new_path = std::path::Path::new(PLASMA_WORKSPACE_ENV_SCRIPT_PATH_DISABLED);

    if !path.exists() {
        return Ok(());
    }

    log::info!("Disabling plasma workspace ssh env");

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
        Err(anyhow!("Failed to disable ssh-agent env file"))
    }
}

async fn disable_default_ssh_agent() -> anyhow::Result<()> {
    disable_plasma_workspace_ssh_env().await?;

    let connection = Connection::session().await?;
    let proxy = SystemdManagerProxy::new(&connection).await?;

    log::info!("Stopping {}", DEFAULT_SSH_AGENT_SOCKET);
    let _ = proxy.stop_unit(DEFAULT_SSH_AGENT_SOCKET, "replace").await;
    log::info!("Stopping {}", DEFAULT_SSH_AGENT_NAME);
    let _ = proxy.stop_unit(DEFAULT_SSH_AGENT_NAME, "replace").await;

    log::info!("Disabling {:?}", vec![DEFAULT_SSH_AGENT_SOCKET, DEFAULT_SSH_AGENT_NAME]);
    proxy
        .disable_unit_files(vec![DEFAULT_SSH_AGENT_SOCKET, DEFAULT_SSH_AGENT_NAME], false)
        .await?;

    log::info!("Masking {:?}", vec![DEFAULT_SSH_AGENT_SOCKET, DEFAULT_SSH_AGENT_NAME]);
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