function PLUGIN:PreUninstall(ctx)
    local exe = ctx.main.path .. "/bin/bw-ssh-agent"
    local ok = os.execute(string.format('"%s" uninstall', exe))
    if not ok then
        error("`bw-ssh-agent uninstall` failed; the systemd user service was not removed")
    end
end
