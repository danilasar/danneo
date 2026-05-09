function on_install(arg)
    print("Installing Hello module...")
    
    rpc.call("admin_menu", "register_items", {
        module = "mod_hello",
        items = {
            {
                code = "greetings",
                category = "content",
                label = "Приветствия",
                link = "/admin/crud/mod_hello/mod_hello_greetings/list",
                weight = 100
            }
        }
    })
end

function on_uninstall(arg)
    print("Uninstalling Hello module...")
    rpc.call("admin_menu", "unregister_module", {
        module = "mod_hello",
        mode = "remove"
    })
end

function frontend_dispatch(arg)
    return "<h1>Hello from Lua module!</h1>"
end

function render_block(arg)
    local block_code = arg.block_code
    if block_code == "b-Hello" then
        return {
            template = "blocks/b-Hello/block.html",
            context = {
                message = "Это сообщение из Lua-блока модуля Hello!"
            }
        }
    end
    return "Unknown block"
end
