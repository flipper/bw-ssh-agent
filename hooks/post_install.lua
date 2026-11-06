function PLUGIN:PostInstall(ctx)
    local path = ctx.rootPath
    local bin = path .. "/bin"
    local ok = os.execute(string.format(
        'mkdir -p "%s" && mv "%s"/bw-ssh-agent-linux-* "%s/bw-ssh-agent" && chmod +x "%s/bw-ssh-agent"',
        bin, path, bin, bin
    ))
    if not ok then
        error("Failed to place bw-ssh-agent binary")
    end
end
