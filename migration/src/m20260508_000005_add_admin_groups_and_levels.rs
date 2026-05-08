use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260508_000005_add_admin_groups_and_levels"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 1. Создаем таблицу групп администраторов
        manager
            .create_table(
                Table::create()
                    .table(CoreAdminGroups::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(CoreAdminGroups::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(CoreAdminGroups::Name).string().not_null().unique_key())
                    .col(ColumnDef::new(CoreAdminGroups::Level).integer().not_null().default(1))
                    .to_owned(),
            )
            .await?;

        // 2. Добавляем колонки в core_admins
        manager
            .alter_table(
                Table::alter()
                    .table(CoreAdmins::Table)
                    .add_column(ColumnDef::new(CoreAdmins::GroupId).integer().null())
                    .add_column(ColumnDef::new(CoreAdmins::Level).integer().not_null().default(1))
                    .to_owned(),
            )
            .await?;

        // 3. Создаем дефолтную группу SuperAdmins
        let insert_group = Query::insert()
            .into_table(CoreAdminGroups::Table)
            .columns([CoreAdminGroups::Name, CoreAdminGroups::Level])
            .values_panic(["SuperAdmins".into(), 100.into()])
            .to_owned();
        manager.exec_stmt(insert_group).await?;

        // 4. Привязываем первого админа к SuperAdmins и ставим ему уровень 100
        let update_admin = Query::update()
            .table(CoreAdmins::Table)
            .values([
                (CoreAdmins::GroupId, 1.into()),
                (CoreAdmins::Level, 100.into()),
            ])
            .and_where(Expr::col(CoreAdmins::Id).eq(1))
            .to_owned();
        manager.exec_stmt(update_admin).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(CoreAdmins::Table)
                    .drop_column(CoreAdmins::GroupId)
                    .drop_column(CoreAdmins::Level)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(Table::drop().table(CoreAdminGroups::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum CoreAdminGroups {
    Table,
    Id,
    Name,
    Level,
}

#[derive(DeriveIden)]
enum CoreAdmins {
    Table,
    Id,
    GroupId,
    Level,
}
