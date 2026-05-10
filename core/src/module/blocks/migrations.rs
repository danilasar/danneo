use sea_orm_migration::prelude::*;

pub struct CreateBlockTables;

impl MigrationName for CreateBlockTables {
    fn name(&self) -> &str {
        "m20260508_000002_create_block_tables"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for CreateBlockTables {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Alias::new("core_block_posit"))
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Alias::new("id"))
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Alias::new("positcode"))
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(Alias::new("positname"))
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Alias::new("pposit"))
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
                    .table(Alias::new("core_blocks"))
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Alias::new("id"))
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Alias::new("positcode")).string().not_null())
                    .col(ColumnDef::new(Alias::new("block_name")).string().not_null())
                    .col(ColumnDef::new(Alias::new("block_file")).string().not_null())
                    .col(
                        ColumnDef::new(Alias::new("block_active"))
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(Alias::new("block_weight"))
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(ColumnDef::new(Alias::new("block_temp")).string().null())
                    .col(ColumnDef::new(Alias::new("block_mods")).json_binary().null())
                    .col(
                        ColumnDef::new(Alias::new("block_access"))
                            .string()
                            .not_null()
                            .default("all"),
                    )
                    .col(
                        ColumnDef::new(Alias::new("block_setting"))
                            .json_binary()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Alias::new("core_block_definitions"))
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Alias::new("id"))
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Alias::new("block_code"))
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(Alias::new("module_code")).string())
                    .col(ColumnDef::new(Alias::new("package_id")).integer().not_null())
                    .col(ColumnDef::new(Alias::new("version")).string().not_null())
                    .col(
                        ColumnDef::new(Alias::new("enabled"))
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(ColumnDef::new(Alias::new("manifest")).json_binary().not_null())
                    .col(ColumnDef::new(Alias::new("settings_schema")).json_binary())
                    .col(ColumnDef::new(Alias::new("template_path")).string())
                    .col(ColumnDef::new(Alias::new("renderer_type")).string().not_null())
                    .to_owned(),
            )
            .await?;

        let insert_posits = Query::insert()
            .into_table(Alias::new("core_block_posit"))
            .columns([
                Alias::new("positcode"),
                Alias::new("positname"),
                Alias::new("pposit"),
            ])
            .values_panic(["leftblock".into(), "Левая колонка".into(), 1.into()])
            .values_panic(["rightblock".into(), "Правая колонка".into(), 2.into()])
            .values_panic(["topblock".into(), "Верх центра".into(), 3.into()])
            .values_panic(["botblock".into(), "Низ центра".into(), 4.into()])
            .to_owned();

        manager.exec_stmt(insert_posits).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Alias::new("core_block_definitions")).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Alias::new("core_blocks")).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Alias::new("core_block_posit")).to_owned())
            .await?;

        Ok(())
    }
}
