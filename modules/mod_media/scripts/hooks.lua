function on_install(arg)
    print("Installing Media module...")
    -- Register in Admin Menu
    rpc.call("admin_menu", "register_items", {
        module = "mod_media",
        items = {
            {
                code = "manage",
                category = "tools",
                label = "Медиа-менеджер",
                link = "/admin/media",
                weight = 10
            }
        }
    })
end

function on_uninstall(arg)
    rpc.call("admin_menu", "unregister_module", { module = "mod_media" })
end

function admin_dispatch(arg)
    local path = arg.path

    if path == "upload_page" or path == "" then
        return {
            template = "apanel/upload.html",
            context = {}
        }
    end

    if path == "do_upload" then
        -- We expect files metadata in arg.files
        if #arg.files == 0 then
             return {
                template = "apanel/upload.html",
                context = { message = "Ошибка: Файл не выбран." }
             }
        end

        local file = arg.files[1]
        local destination = "uploads/" .. file.name

        -- Call Storage RPC using FILE PATH, not Base64 content!
        local upload_res = rpc.call("storage", "upload", {
            path = destination,
            file_path = file.temp_path
        })

        if upload_res.status == "success" then
            -- Get public URL
            local url_res = rpc.call("storage", "get_url", { path = destination })
            
            return {
                template = "apanel/upload.html",
                context = {
                    message = "Загрузка завершена!",
                    file_path = destination,
                    file_url = url_res.url
                }
            }
        else
            return {
                template = "apanel/upload.html",
                context = { message = "Ошибка загрузки: " .. tostring(upload_res.error) }
            }
        end
    end

    return "Unknown path: " .. tostring(path)
end
