function on_install(arg)
    print("Installing Menu module...")
    
    db.create_table({
        table_name = "core_menu_groups",
        fields = {
            { name = "id", field_type = "integer", primary_key = true, auto_increment = true },
            { name = "code", field_type = "string", unique = true },
            { name = "title", field_type = "string", nullable = false }
        }
    })

    db.create_table({
        table_name = "core_menu_items",
        fields = {
            { name = "id", field_type = "integer", primary_key = true, auto_increment = true },
            { name = "group_id", field_type = "integer", nullable = false },
            { name = "parent_id", field_type = "integer", default = 0 },
            { name = "title", field_type = "string", nullable = false },
            { name = "link", field_type = "string", nullable = false },
            { name = "target", field_type = "string", default = "_self" },
            { name = "posit", field_type = "integer", default = 0 },
            { name = "acc", field_type = "string", default = "all" },
            { name = "css", field_type = "string", nullable = true }
        }
    })

    -- Seed defaults
    db.insert("core_menu_groups", { code = "top", title = "Верхнее меню" })
    db.insert("core_menu_groups", { code = "bottom", title = "Нижнее меню" })
    
    db.insert("core_menu_items", { group_id = 1, title = "Главная", link = "/", posit = 10 })
    db.insert("core_menu_items", { group_id = 1, title = "Новости", link = "/news", posit = 20 })
    
    -- Register in Admin Menu
    rpc.call("admin_menu", "register_items", {
        module = "mod_menu",
        items = {
            {
                code = "manage",
                category = "settings",
                label = "Меню сайта (Lua)",
                link = "/admin/menu",
                weight = 50
            }
        }
    })
end

function on_uninstall(arg)
    db.drop_table("core_menu_items")
    db.drop_table("core_menu_groups")
    rpc.call("admin_menu", "unregister_module", { module = "mod_menu" })
end

function render_block(arg)
    local block_code = arg.block_code
    local group_code = "top"
    if arg.settings ~= nil and arg.settings.group ~= nil then
        group_code = arg.settings.group
    end

    local groups = db.select("core_menu_groups", {"id"}, { code = group_code })
    if #groups == 0 then return "Group not found" end
    local group_id = groups[1].id

    local items = db.select("core_menu_items", {"title", "link", "parent_id", "target", "css"}, { group_id = group_id })
    
    return {
        template = "blocks/b-Menu/block.html",
        context = {
            items = items
        }
    }
end

function rpc_get_menu(payload, ctx)
    local code = payload.code or "top"
    local groups = db.select("core_menu_groups", {"id"}, { code = code })
    if #groups == 0 then return {} end
    
    return db.select("core_menu_items", {"title", "link", "target", "css"}, { group_id = groups[1].id })
end

function register_admin_routes()
    local r = danneo.Router.new()
    r:get("/", "admin_dispatch")
    r:get("/items", "admin_dispatch")
    r:post("/group/save", "admin_dispatch")
    r:get("/group/delete", "admin_dispatch")
    r:post("/item/save", "admin_dispatch")
    r:get("/item/delete", "admin_dispatch")
    return r
end

function admin_dispatch(arg)
    -- Main list of groups
    if arg.uri == "/" or arg.uri == "" then
        local groups = db.select("core_menu_groups", {"id", "code", "title"})
        return {
            template = "apanel/menu_list.html",
            context = { groups = groups }
        }
    end

    -- List items in a group
    if arg.uri == "/items" then
        local group_id = tonumber(arg.params.group_id or arg.query.group_id)
        local groups = db.select("core_menu_groups", {"id", "title"}, { id = group_id })
        if #groups == 0 then return "Group not found" end
        
        local items = db.select("core_menu_items", {"id", "parent_id", "title", "link", "target", "posit", "acc", "css"}, { group_id = group_id })
        return {
            template = "apanel/menu_items.html",
            context = { 
                group = groups[1],
                items = items
            }
        }
    end

    -- Save group
    if arg.uri == "/group/save" then
        local id = tonumber(arg.form.id)
        local data = { code = arg.form.code, title = arg.form.title }
        if id and id > 0 then
            db.update("core_menu_groups", { id = id }, data)
        else
            db.insert("core_menu_groups", data)
        end
        return { redirect = "/admin/menu" }
    end

    -- Delete group
    if arg.uri == "/group/delete" then
        local id = tonumber(arg.query.id)
        db.delete("core_menu_items", { group_id = id })
        db.delete("core_menu_groups", { id = id })
        return { redirect = "/admin/menu" }
    end

    -- Save item
    if arg.uri == "/item/save" then
        local id = tonumber(arg.form.id)
        local group_id = tonumber(arg.form.group_id)
        local data = {
            group_id = group_id,
            parent_id = tonumber(arg.form.parent_id) or 0,
            title = arg.form.title,
            link = arg.form.link,
            target = arg.form.target or "_self",
            posit = tonumber(arg.form.posit) or 0,
            acc = arg.form.acc or "all",
            css = arg.form.css
        }
        if id and id > 0 then
            db.update("core_menu_items", { id = id }, data)
        else
            db.insert("core_menu_items", data)
        end
        return { redirect = "/admin/menu/items?group_id=" .. tostring(group_id) }
    end

    -- Delete item
    if arg.uri == "/item/delete" then
        local id = tonumber(arg.query.id)
        local group_id = tonumber(arg.query.group_id)
        db.delete("core_menu_items", { id = id })
        return { redirect = "/admin/menu/items?group_id=" .. tostring(group_id) }
    end

    return "Unknown path: " .. tostring(arg.uri)
end
