# bw-ssh-agent

`bw-ssh-agent` is a lightweight SSH agent that uses your Bitwarden SSH keys.

It supports 2FA during login and supports custom Bitwarden server endpoints.

This project is an unofficial SSH agent and is not affiliated with or endorsed by Bitwarden.

## Requirements

- Linux
- Secret Service (usually already available on most Linux desktop setups)

`systemd` is not required to use `bw-ssh-agent`, but this README uses it for the easiest setup.

## Quick Start (systemd)

1. Download the release binary for your system (`bw-ssh-agent-(os)-(arch)`).
2. Move it to `$HOME/.local/bin` and rename it to `bw-ssh-agent`:

```bash
mkdir -p "$HOME/.local/bin"
mv /path/to/downloaded/binary "$HOME/.local/bin/bw-ssh-agent"
chmod +x "$HOME/.local/bin/bw-ssh-agent"
```

3. Log in:

```bash
bw-ssh-agent login
```

4. Install and start the service:

```bash
bw-ssh-agent install
```

5. Reboot to apply all changes.

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

Run the SSH agent.

Optional flag:

- `--sync-interval <seconds>` (default: `1800`)

### `bw-ssh-agent install`

Install and start `bw-ssh-agent` as a user service (systemd workflow).

After running install, reboot your system to apply all changes.

## Custom Bitwarden Endpoints

Use custom servers with:

```bash
bw-ssh-agent login --identity-url <identity-url> --api-url <api-url>
```

<details>
<summary>Bitwarden EU endpoints</summary>

```bash
bw-ssh-agent login \
  --identity-url https://vault.bitwarden.eu/identity/ \
  --api-url https://vault.bitwarden.eu/api/
```

</details>

## Notes

- Socket path: `$XDG_RUNTIME_DIR/bw-ssh-agent.sock`
- Make sure `$HOME/.local/bin` is in your `PATH`.
- If command not found, restart your shell or run:

```bash
export PATH="$HOME/.local/bin:$PATH"
```
