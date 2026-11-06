# bw-ssh-agent

`bw-ssh-agent` is a lightweight SSH agent that uses your Bitwarden SSH keys.

It supports email-based 2FA during login and custom Bitwarden server endpoints.

This project is unofficial and is not affiliated with or endorsed by Bitwarden.

## Requirements

- Linux with systemd
- Secret Service (usually already available on most Linux desktop setups)

## Quick Start

1. Install `bw-ssh-agent` with [mise](https://mise.jdx.dev/) (vfox backend):

   ```bash
   mise plugins install vfox:bw-ssh-agent 'packslip:flipper/bw-ssh-agent'
   mise use -g vfox:bw-ssh-agent
   ```

   This installs the plugin from a signed [packslip](https://packslip.dev) release; mise verifies the signature before installing. To pin a plugin version, append `#<version>` to the packslip source, or in `mise.toml`:

   ```toml
   [plugins]
   "vfox:bw-ssh-agent" = "packslip:flipper/bw-ssh-agent#1.0.0"
   ```

   <details>
   <summary>Install with <a href="https://vfox.dev/">vfox</a></summary>

   ```bash
   vfox add --source https://github.com/flipper/bw-ssh-agent/releases/latest/download/bw-ssh-agent.zip bw-ssh-agent
   vfox install bw-ssh-agent@latest
   vfox use -g bw-ssh-agent
   ```

   </details>

   <details>
   <summary>Install manually (download binary)</summary>

   1. Download the release binary for your system (`bw-ssh-agent-(os)-(arch)`).
   2. Move it to `$HOME/.local/bin` and rename it to `bw-ssh-agent`:

      ```bash
      mkdir -p "$HOME/.local/bin"
      mv /path/to/downloaded/binary "$HOME/.local/bin/bw-ssh-agent"
      chmod +x "$HOME/.local/bin/bw-ssh-agent"
      ```

   Make sure `$HOME/.local/bin` is in your `PATH`. If `bw-ssh-agent` is not found, restart your shell or run:

      ```bash
      export PATH="$HOME/.local/bin:$PATH"
      ```

   </details>

2. Log in:

   ```bash
   bw-ssh-agent login
   ```

3. Set up the background systemd user service:

   ```bash
   bw-ssh-agent install
   ```

4. Reboot to apply all changes.

## Commands

### `bw-ssh-agent login`

Log in to Bitwarden, including email-based 2FA.

Optional flags:

- `--email <email>`
- `--password <password>`
- `--identity-url <url>`
- `--api-url <url>`

### `bw-ssh-agent status`

Show login status.

### `bw-ssh-agent agent`

Run the SSH agent (this is what your service manager should run in the background).

Optional flag:

- `--sync-interval <seconds>` (default: `1800`)

### `bw-ssh-agent install`

Automatic setup for systemd: installs and starts `bw-ssh-agent` as a user service.

After running install, reboot your system to apply all changes.

### `bw-ssh-agent uninstall`

Remove the installed user service setup.

## Custom Bitwarden Endpoints

Use custom servers with:

```bash
bw-ssh-agent login --identity-url <identity-url> --api-url <api-url>
```

<details>
<summary>Bitwarden EU example</summary>

```bash
bw-ssh-agent login \
  --identity-url https://vault.bitwarden.eu/identity/ \
  --api-url https://vault.bitwarden.eu/api/
```

</details>

## Notes

- When installed with mise or vfox, the plugin runs `bw-ssh-agent install` for you on every install or upgrade (so step 3 is not needed), and `bw-ssh-agent uninstall` whenever an installed version is removed.
- `SSH_AUTH_SOCK` is configured through systemd `environment.d` by `bw-ssh-agent install`; no shell profile changes are needed.
- Socket path: `$XDG_RUNTIME_DIR/bw-ssh-agent.sock`
