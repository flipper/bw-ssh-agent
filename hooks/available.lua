local http = require("http")
local json = require("json")

function PLUGIN:Available(ctx)
    local resp, err = http.get({
        url = "https://api.github.com/repos/flipper/bw-ssh-agent/releases?per_page=100",
    })
    if err ~= nil or resp.status_code ~= 200 then
        error("Failed to fetch releases: " .. (err or resp.status_code))
    end

    local result = {}
    for _, release in ipairs(json.decode(resp.body)) do
        if not release.draft then
            local version = release.tag_name:gsub("^v", "")
            table.insert(result, {
                version = version,
                created_at = release.published_at,
                note = release.prerelease and "pre-release" or "",
            })
        end
    end
    return result
end
