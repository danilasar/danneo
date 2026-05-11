return {
    name = "001_init_news",
    up = function()
        danneo.log.info("Creating news tables via Lua migration")
        
        danneo.db.create_table({
            table_name = "categories",
            fields = {
                { name = "id", field_type = "integer", primary_key = true, auto_increment = true },
                { name = "title", field_type = "string" }
            }
        })

        danneo.db.create_table({
            table_name = "news",
            fields = {
                { name = "id", field_type = "integer", primary_key = true, auto_increment = true },
                { name = "cat_id", field_type = "integer" },
                { name = "title", field_type = "string" },
                { name = "content", field_type = "text" },
                { name = "created_at", field_type = "datetime" }
            }
        })
        
        danneo.db.insert("categories", { title = "Общие" })
    end,
    down = function()
        danneo.log.info("Dropping news tables")
        danneo.db.drop_table("news")
        danneo.db.drop_table("categories")
    end
}
