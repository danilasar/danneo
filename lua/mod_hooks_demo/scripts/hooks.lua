function on_install(arg)
    print("Hooks Demo module installed!")
end

function on_enable(arg)
    print("Hooks Demo module enabled!")
end

function before_save(arg)
    print("Saving entity: " .. tostring(arg.entity))
end
