use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260508_000004_update_core_admins"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Добавляем колонки email и permissions
        manager
            .alter_table(
                Table::alter()
                    .table(CoreAdmins::Table)
                    .add_column(ColumnDef::new(CoreAdmins::Email).string().null())
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(CoreAdmins::Table)
                    .add_column(
                        ColumnDef::new(CoreAdmins::Permissions)
                            .json_binary()
                            .not_null()
                            .default(Expr::value(serde_json::json!([]))),
                    )
                    .to_owned(),
            )
            .await?;

        // Обновляем права для первого админа (admin) - даем ему всё
        // В Danneo 2.0 Rust мы можем использовать ["all"] как спец-флаг
        let update_admin = Query::update()
            .table(CoreAdmins::Table)
            .values([(
                CoreAdmins::Permissions,
                Expr::value(serde_json::json!(["all"])),
            )])
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
                    .drop_column(CoreAdmins::Email)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(CoreAdmins::Table)
                    .drop_column(CoreAdmins::Permissions)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum CoreAdmins {
    Table,
    Id,
    Email,
    Permissions,
}
