use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260508_000002_create_block_tables"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Таблица позиций блоков (core_block_posit)
        manager
            .create_table(
                Table::create()
                    .table(CoreBlockPosit::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(CoreBlockPosit::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(CoreBlockPosit::Positcode).string().not_null().unique_key())
                    .col(ColumnDef::new(CoreBlockPosit::Positname).string().not_null())
                    .col(ColumnDef::new(CoreBlockPosit::Pposit).integer().not_null().default(0))
                    .to_owned(),
            )
            .await?;

        // Таблица самих блоков (core_blocks)
        manager
            .create_table(
                Table::create()
                    .table(CoreBlocks::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(CoreBlocks::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(CoreBlocks::Positcode).string().not_null())
                    .col(ColumnDef::new(CoreBlocks::BlockName).string().not_null())
                    .col(ColumnDef::new(CoreBlocks::BlockFile).string().not_null())
                    .col(ColumnDef::new(CoreBlocks::BlockActive).boolean().not_null().default(true))
                    .col(ColumnDef::new(CoreBlocks::BlockWeight).integer().not_null().default(0))
                    .col(ColumnDef::new(CoreBlocks::BlockTemp).string().null())
                    .col(ColumnDef::new(CoreBlocks::BlockMods).json().null())
                    .col(ColumnDef::new(CoreBlocks::BlockAccess).string().not_null().default("all"))
                    .col(ColumnDef::new(CoreBlocks::BlockSetting).json().null())
                    .to_owned(),
            )
            .await?;

        // Дефолтные позиции для темы Soft/Lite
        let insert_posits = Query::insert()
            .into_table(CoreBlockPosit::Table)
            .columns([CoreBlockPosit::Positcode, CoreBlockPosit::Positname, CoreBlockPosit::Pposit])
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
            .drop_table(Table::drop().table(CoreBlocks::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(CoreBlockPosit::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum CoreBlockPosit {
    Table,
    Id,
    Positcode,
    Positname,
    Pposit,
}

#[derive(DeriveIden)]
enum CoreBlocks {
    Table,
    Id,
    Positcode,
    BlockName,
    BlockFile,
    BlockActive,
    BlockWeight,
    BlockTemp,
    BlockMods,
    BlockAccess,
    BlockSetting,
}
