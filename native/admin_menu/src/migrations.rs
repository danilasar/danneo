use sea_orm_migration::prelude::*;

pub struct CreateMenuTables;

impl MigrationName for CreateMenuTables {
    fn name(&self) -> &str {
        "m20260508_000003_create_menu_tables"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for CreateMenuTables {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Alias::new("core_menu_groups"))
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Alias::new("code"))
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Alias::new("parent_code")).string())
                    .col(ColumnDef::new(Alias::new("label")).string().not_null())
                    .col(ColumnDef::new(Alias::new("icon")).string())
                    .col(
                        ColumnDef::new(Alias::new("weight"))
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Alias::new("core_menu_items"))
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Alias::new("id"))
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Alias::new("module_code"))
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Alias::new("item_code")).string().not_null())
                    .col(ColumnDef::new(Alias::new("category")).string().not_null())
                    .col(ColumnDef::new(Alias::new("label")).string().not_null())
                    .col(ColumnDef::new(Alias::new("link")).string().not_null())
                    .col(
                        ColumnDef::new(Alias::new("weight"))
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .index(
                        Index::create()
                            .name("idx_menu_item")
                            .unique()
                            .col(Alias::new("module_code"))
                            .col(Alias::new("item_code")),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(Alias::new("core_menu_items"))
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(Alias::new("core_menu_groups"))
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}
