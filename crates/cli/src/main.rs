mod systemd_manager;
mod install;
mod consts;
mod uninstall;

use crate::Commands::Login;
use anyhow::{Error, anyhow};
use bitwarden_client::{
    BitwardenAuthClient, BitwardenClient, ClientError, ClientSettings, PasswordLoginRequest,
    SendTwoFactorEmail,
};
use clap::{Parser, Subcommand};
use common::agent::Agent;
use common::ssh_agent_lib::agent::listen;
use common::{
    APP_ID, SecretItem, SecretStoreInitializer, ZBusSecretStore, get_stored_refresh_token,
    get_stored_user_key, set_default_store, store_refresh_token, store_user_key, sync_vault,
};
use dialoguer::theme::ColorfulTheme;
use dialoguer::{Input, Password};
use libsystemd::daemon::NotifyState;
use log::LevelFilter;
use simple_logger::SimpleLogger;
use std::sync::Arc;
use tokio::net::UnixListener;
use crate::install::install;
use crate::uninstall::uninstall;

/// Bitwarden SSH Agent
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    cmd: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Login
    Login {
        /// Identity Server URL
        #[clap(long, default_value = "https://vault.bitwarden.com/identity/")]
        identity_url: String,
        /// API Server URL
        #[clap(long, default_value = "https://vault.bitwarden.com/api/")]
        api_url: String,

        #[clap(long)]
        email: Option<String>,
        #[clap(long)]
        password: Option<String>,
    },
    Status,
    Agent {
        /// How often to sync with the server in seconds
        #[clap(long, default_value = "1800")]
        sync_interval: u64,
    },
    Install,
    Uninstall
}

async fn sync() -> anyhow::Result<()> {
    let identity_url_entry = SecretItem::new(APP_ID.parse()?, "identity-url".parse()?);
    let api_url_entry = SecretItem::new(APP_ID.parse()?, "api-url".parse()?);

    let identity_url = match identity_url_entry.get().await {
        Ok(url) => match url {
            Some(url) => String::from_utf8_lossy(url.as_slice()).into_owned(),
            _ => {
                return Err(anyhow!("No identity url found. Did you forget to login?"));
            }
        },
        Err(e) => {
            return Err(anyhow!("Unable to read identity url from storage: {}", e));
        }
    };

    let api_url = match api_url_entry.get().await {
        Ok(url) => match url {
            Some(url) => String::from_utf8_lossy(url.as_slice()).into_owned(),
            _ => {
                return Err(anyhow!("No api url found. Did you forget to login?"));
            }
        },
        Err(e) => {
            return Err(anyhow!("Unable to read api url from storage: {}", e));
        }
    };

    let auth_client =
        match BitwardenAuthClient::new(identity_url.parse()?, api_url.clone().parse()?) {
            Ok(a) => a,
            Err(e) => {
                return Err(anyhow!("Failed to create auth client: {}", e));
            }
        };

    let refresh_token = match get_stored_refresh_token().await {
        Ok(token) => match token {
            Some(token) => token,
            None => {
                return Err(anyhow!("No refresh token is stored"));
            }
        },
        Err(e) => {
            return Err(anyhow!("Failed to get refresh token: {}", e));
        }
    };

    let token_response = auth_client.renew_token(refresh_token).await;

    let token_response = match token_response {
        Ok(token) => token,
        Err(e) => {
            return Err(anyhow!("Failed to renew token: {}", e));
        }
    };

    match store_refresh_token(token_response.refresh_token).await {
        Ok(_) => {}
        Err(e) => {
            return Err(anyhow!("Failed to store refresh token: {}", e));
        }
    }

    let client = match BitwardenClient::new(ClientSettings {
        api_url: api_url.parse().unwrap(),
        access_token: token_response.access_token,
    }) {
        Ok(c) => c,
        Err(e) => {
            return Err(anyhow!("Failed to create client: {}", e));
        }
    };

    let user_key = match get_stored_user_key().await {
        Ok(k) => match k {
            Some(k) => k,
            None => {
                return Err(anyhow!("No user key is stored"));
            }
        },
        Err(e) => {
            return Err(anyhow!("Failed to get user key: {}", e));
        }
    };

    sync_vault(client, &user_key).await;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    SimpleLogger::new()
        .with_level(LevelFilter::Info)
        .with_module_level("zbus", LevelFilter::Warn)
        .with_module_level("tracing", LevelFilter::Warn)
        .init()?;

    set_default_store(Arc::new(ZBusSecretStore::new())).await;

    let running_in_systemd = libsystemd::daemon::booted();

    let args = Args::parse();

    match args.cmd {
        Login {
            identity_url,
            api_url,
            email,
            password,
        } => {
            let client = BitwardenAuthClient::new(identity_url.clone(), api_url.clone())?;

            let email = match email {
                Some(email) => email,
                None => Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Enter your email")
                    .validate_with(|input: &String| -> Result<(), &str> {
                        if input.contains('@') {
                            Ok(())
                        } else {
                            Err("This is not a email address")
                        }
                    })
                    .interact_text()?,
            };

            let password = match password {
                Some(password) => password,
                None => Password::with_theme(&ColorfulTheme::default())
                    .with_prompt("Enter your password")
                    .interact()?,
            };

            let mut request = PasswordLoginRequest {
                email: email.clone(),
                password: password.clone(),
                ..Default::default()
            };

            let mut key_store = None;

            loop {
                log::info!("Sending login request");
                let response = client.login_password(request.clone()).await;

                if let Err(e) = &response {
                    if let ClientError::Http(_, o) = e {
                        if o.error_description == "New device verification required" {
                            log::info!("New device verification required");

                            let code: String = Input::with_theme(&ColorfulTheme::default())
                                .with_prompt("Enter the device verification code")
                                .interact_text()?;

                            request.new_device_otp = Some(code);
                        } else if o.error_description == "Two factor required." {
                            log::info!("Two factor required.");

                            if let None = o.two_factor_providers {
                                panic!("no two factor provider found");
                            }

                            let providers = o.two_factor_providers.as_ref().unwrap();

                            if !providers.contains(&"1".to_string()) {
                                panic!("Only email 2fa is currently supported");
                            }

                            client
                                .send_two_factor_email(SendTwoFactorEmail {
                                    email: email.clone(),
                                    password: password.clone(),
                                    ..Default::default()
                                })
                                .await?;

                            let code: String = Input::with_theme(&ColorfulTheme::default())
                                .with_prompt("Enter your two factor code")
                                .interact_text()?;

                            request.two_factor_token = Some(code);
                            request.two_factor_provider = Some("1".parse()?);
                            request.two_factor_remember = Some(false);
                        } else {
                            log::error!("Error: {:?}", e);
                            break;
                        }
                    } else {
                        log::error!("Error: {:?}", e);
                        break;
                    }
                } else {
                    key_store = Some(response?);
                    log::info!("Password logged in successfully.");
                    break;
                }
            }

            if let Some(key_store) = key_store {
                store_user_key(key_store.user_key).await?;
                store_refresh_token(key_store.refresh_token.parse()?).await?;

                let identity_url_entry = SecretItem::new(APP_ID.parse()?, "identity-url".parse()?);
                identity_url_entry
                    .set(identity_url.as_bytes(), None)
                    .await?;

                let api_url_entry = SecretItem::new(APP_ID.parse()?, "api-url".parse()?);
                api_url_entry.set(api_url.as_bytes(), None).await?;
            }
        }
        Commands::Status => {
            let status = match get_stored_refresh_token().await {
                Ok(_) => "Logged in",
                Err(_) => "Logged out",
            };

            println!("Status: {}", status);
        }
        Commands::Agent { sync_interval } => {
            log::info!("Syncing the vault every {} seconds...", sync_interval);

            let runtime_dir = dirs_next::runtime_dir().expect("runtime dir not found");
            let socket_path = runtime_dir.join("bw-ssh-agent.sock");
            let _ = tokio::fs::remove_file(&socket_path).await;

            if running_in_systemd {
                libsystemd::daemon::notify(false, &[NotifyState::Ready])?;
            }

            tokio::select! {
                _ = tokio::spawn(async move {
                    loop {
                        let sync_result = sync().await;

                        let sleep_duration = match sync_result {
                            Ok(_) => {
                                log::info!("Sync successful");
                                if running_in_systemd {
                                   let _ = libsystemd::daemon::notify(false, &[NotifyState::Status("Synced".to_string())]);
                                }
                                std::time::Duration::from_secs(sync_interval)
                            }
                            Err(e) => {
                                log::error!("Sync failed: {}", e);
                                if running_in_systemd {
                                   let _ = libsystemd::daemon::notify(false, &[NotifyState::Status(format!("Error: {}", e))]);
                                }
                                std::time::Duration::from_secs(30)
                            }
                        };

                        tokio::time::sleep(sleep_duration).await;
                    }
                }) => {

                }
                _ = tokio::signal::ctrl_c() => {
                    println!("received SIGINT");
                }
                _ = listen(UnixListener::bind(socket_path)?, Agent::default()) => {}
            }
        }
        Commands::Install => {
            match install(running_in_systemd).await {
                Ok(_) => {
                    log::info!("Installed successfully.");
                }
                Err(e) => {
                    log::error!("install failed: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::Uninstall => {
            match uninstall(running_in_systemd).await {
                Ok(_) => {
                    log::info!("Uninstalled successfully.");
                }
                Err(e) => {
                    log::error!("uninstall failed: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }
    Ok(())
}
