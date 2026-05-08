use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260508_000003_create_menu_tables"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Таблица групп меню
        manager
            .create_table(
                Table::create()
                    .table(CoreMenuGroups::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(CoreMenuGroups::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(CoreMenuGroups::Code)
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(CoreMenuGroups::Title).string().not_null())
                    .to_owned(),
            )
            .await?;

        // Таблица пунктов меню
        manager
            .create_table(
                Table::create()
                    .table(CoreMenuItems::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(CoreMenuItems::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(CoreMenuItems::GroupId).integer().not_null())
                    .col(
                        ColumnDef::new(CoreMenuItems::ParentId)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(ColumnDef::new(CoreMenuItems::Title).string().not_null())
                    .col(ColumnDef::new(CoreMenuItems::Link).string().not_null())
                    .col(
                        ColumnDef::new(CoreMenuItems::Target)
                            .string()
                            .not_null()
                            .default("_self"),
                    )
                    .col(ColumnDef::new(CoreMenuItems::Css).string().null())
                    .col(ColumnDef::new(CoreMenuItems::Before).string().null())
                    .col(ColumnDef::new(CoreMenuItems::After).string().null())
                    .col(
                        ColumnDef::new(CoreMenuItems::Posit)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(CoreMenuItems::Acc)
                            .string()
                            .not_null()
                            .default("all"),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-menu-items-group")
                            .from(CoreMenuItems::Table, CoreMenuItems::GroupId)
                            .to(CoreMenuGroups::Table, CoreMenuGroups::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Дефолтная группа меню
        let insert_group = Query::insert()
            .into_table(CoreMenuGroups::Table)
            .columns([CoreMenuGroups::Code, CoreMenuGroups::Title])
            .values_panic(["top_menu".into(), "Верхнее меню".into()])
            .to_owned();

        manager.exec_stmt(insert_group).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(CoreMenuItems::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(CoreMenuGroups::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum CoreMenuGroups {
    Table,
    Id,
    Code,
    Title,
}

#[derive(DeriveIden)]
enum CoreMenuItems {
    Table,
    Id,
    GroupId,
    ParentId,
    Title,
    Link,
    Target,
    Css,
    Before,
    After,
    Posit,
    Acc,
}
