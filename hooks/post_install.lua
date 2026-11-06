function PLUGIN:PostInstall(ctx)
    local path = ctx.rootPath
    local bin = path .. "/bin"
    local exe = bin .. "/bw-ssh-agent"

    local ok = os.execute(string.format(
        'mkdir -p "%s" && mv "%s"/bw-ssh-agent-linux-* "%s" && chmod +x "%s"',
        bin, path, exe, exe
    ))
    if not ok then
        error("Failed to place bw-ssh-agent binary")
    end

    ok = os.execute(string.format('"%s" install', exe))
    if not ok then
        error("`bw-ssh-agent install` failed; the systemd user service was not set up")
    end
end
