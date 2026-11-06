function PLUGIN:PreUninstall(ctx)
    local path = ctx.main.path
    local exe = path .. "/bin/bw-ssh-agent"

    local config = os.getenv("XDG_CONFIG_HOME")
    if config == nil or config == "" then
        config = os.getenv("HOME") .. "/.config"
    end

    local unit = io.open(config .. "/systemd/user/bw-ssh-agent.service", "r")
    if unit == nil then
        return
    end
    local contents = unit:read("*a")
    unit:close()

    -- On upgrade the new version is installed first and the unit already points
    -- at it; only tear the service down when it still points at this version.
    if not contents:find(exe, 1, true) then
        return
    end

    local ok = os.execute(string.format('"%s" uninstall', exe))
    if not ok then
        error("`bw-ssh-agent uninstall` failed; the systemd user service was not removed")
    end
end
