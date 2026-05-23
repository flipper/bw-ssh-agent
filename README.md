# bw-ssh-agent

`bw-ssh-agent` is a lightweight SSH agent that uses your Bitwarden SSH keys.

It supports 2FA during login and custom Bitwarden server endpoints.

This project is unofficial and is not affiliated with or endorsed by Bitwarden.

## Requirements

- Linux
- Secret Service (usually already available on most Linux desktop setups)

## Quick Start

1. Install `bw-ssh-agent` (choose one method):

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

   <details>
   <summary>Install with <a href="https://mise.jdx.dev/">mise</a></summary>

   ```bash
   mise use -g github:flipper/bw-ssh-agent
   ```

   </details>

2. Log in:

   ```bash
   bw-ssh-agent login
   ```

3. Set up background service (systemd, recommended):

   ```bash
   bw-ssh-agent install
   ```

   <details>
   <summary>Other init systems</summary>

   Configure your init/service manager to run this in the background:

   ```bash
   bw-ssh-agent agent
   ```

   </details>

4. Reboot to apply all changes.

## Commands

### `bw-ssh-agent login`

Log in to Bitwarden, including 2FA.

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

- Socket path: `$XDG_RUNTIME_DIR/bw-ssh-agent.sock`
