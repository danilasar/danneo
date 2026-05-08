use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260508_000001_create_core_tables"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Создаем таблицу администраторов (core_admins)
        manager
            .create_table(
                Table::create()
                    .table(CoreAdmins::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(CoreAdmins::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(CoreAdmins::Login).string().not_null().unique_key())
                    .col(ColumnDef::new(CoreAdmins::PasswordHash).string().not_null())
                    .to_owned(),
            )
            .await?;

        // Создаем таблицу глобальных настроек (core_settings)
        manager
            .create_table(
                Table::create()
                    .table(CoreSettings::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(CoreSettings::Key)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(CoreSettings::Value).json().not_null())
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(CoreSettings::Table).to_owned())
            .await?;
        
        manager
            .drop_table(Table::drop().table(CoreAdmins::Table).to_owned())
            .await?;

        // Вставляем дефолтного админа: admin / password
        let password_hash = bcrypt::hash("password", 4).unwrap();
        let insert_admin = Query::insert()
            .into_table(CoreAdmins::Table)
            .columns([CoreAdmins::Login, CoreAdmins::PasswordHash])
            .values_panic(["admin".into(), password_hash.into()])
            .to_owned();

        manager.exec_stmt(insert_admin).await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum CoreAdmins {
    Table,
    Id,
    Login,
    PasswordHash,
}

#[derive(DeriveIden)]
enum CoreSettings {
    Table,
    Key,
    Value,
}
