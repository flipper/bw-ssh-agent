local http = require("http")
local json = require("json")

local ARCHES = {
    amd64 = "x86_64",
    arm64 = "aarch64",
}

function PLUGIN:PreInstall(ctx)
    if RUNTIME.osType ~= "linux" then
        error("bw-ssh-agent requires Linux with systemd")
    end
    local arch = ARCHES[RUNTIME.archType]
    if arch == nil then
        error("Unsupported architecture: " .. RUNTIME.archType)
    end

    local version = ctx.version
    local asset_name = "bw-ssh-agent-linux-" .. arch

    local resp, err = http.get({
        url = "https://api.github.com/repos/flipper/bw-ssh-agent/releases/tags/v" .. version,
    })
    if err ~= nil or resp.status_code ~= 200 then
        error("Failed to fetch release v" .. version .. ": " .. (err or resp.status_code))
    end

    for _, asset in ipairs(json.decode(resp.body).assets) do
        if asset.name == asset_name then
            local sha256 = asset.digest and asset.digest:match("^sha256:(%x+)$")
            if sha256 == nil then
                error("No SHA-256 digest published for " .. asset_name)
            end
            return {
                version = version,
                url = asset.browser_download_url,
                sha256 = sha256,
            }
        end
    end
    error("Release v" .. version .. " has no asset " .. asset_name)
end
