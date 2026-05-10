-- Migration: Handled by migrations/ folder automatically
function on_install(arg)
    print("Installing News module...")

    -- Register in Admin Menu via RPC
    rpc.call("admin_menu", "ensure_category", {
        code = "news",
        parent = "content",
        label = "admin_news",
        icon = "news.gif",
        weight = 20
    })

    rpc.call("admin_menu", "register_items", {
        module = "mod_news",
        items = {
            {
                code = "list",
                category = "news",
                label = "admin_list",
                link = "/admin/news",
                weight = 10
            },
            {
                code = "add",
                category = "news",
                label = "admin_add",
                link = "/admin/news/add",
                weight = 20
            }
        }
    })
end

function on_uninstall(arg)
    print("Uninstalling News module...")
    -- Table dropping handled by migrations/ rollback automatically
    
    -- Clean up Admin Menu via RPC
    rpc.call("admin_menu", "unregister_module", {
        module = "mod_news",
        mode = "remove"
    })
end

function admin_dispatch(arg)
    local path = arg.path

    if path == "list" then
        local news = db.select("news", { "id", "title", "created_at" })
        return {
            template = "news.admin_list.html",
            context = { news = news },
        }
    end

    if path == "add" then
        local categories = db.select("categories", { "id", "title" })
        return {
            template = "news.admin_edit.html",
            context = { categories = categories },
        }
    end

    if path == "save" then
        if arg.method == "POST" then
            local data = arg.form
            if data.cat_id ~= nil then
                data.cat_id = tonumber(data.cat_id)
            end
            db.insert("news", data)
            return { redirect = "/admin/news" }
        end
    end

    return "Unknown admin path: " .. tostring(path)
end

function frontend_dispatch(arg)
    local path = arg.path

    if path == "news" then
        local news = db.select("news", { "id", "title", "content", "created_at" })
        return {
            template = "news.standart.html",
            context = { news = news },
        }
    end

    if arg.params.id ~= nil then
        local id = arg.params.id
        local all_news = db.select("news", { "id", "title", "content", "created_at", "cat_id" })
        local found = nil

        for _, item in ipairs(all_news) do
            if tostring(item.id) == id then
                found = item
                break
            end
        end

        if found ~= nil then
            return {
                template = "news.read.html",
                context = { news = found },
            }
        end
    end

    return "Frontend path: " .. tostring(path)
end

function render_block(arg)
    local block_code = arg.block_code
    local settings = arg.settings or {}

    if block_code == "b-News" then
        local news = db.select("news", { "id", "title", "created_at" })
        -- Simple sort and limit (in a real system this would be in SQL)
        table.sort(news, function(a, b) return a.created_at > b.created_at end)
        
        local limit = tonumber(settings.limit or 5)
        local recent = {}
        for i = 1, math.min(#news, limit) do
            table.insert(recent, news[i])
        end

        return {
            template = "block.html",
            context = {
                news = recent,
                title = "Последние новости"
            }
        }
    end

    return "Unknown block: " .. tostring(block_code)
end
