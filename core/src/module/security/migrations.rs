use sea_orm_migration::prelude::*;

pub struct CreateAdminTable;

impl MigrationName for CreateAdminTable {
    fn name(&self) -> &str {
        "m20260508_000001_create_core_tables"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for CreateAdminTable {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Alias::new("core_admins"))
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Alias::new("id"))
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Alias::new("login"))
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(Alias::new("password_hash")).string().not_null())
                    .col(ColumnDef::new(Alias::new("email")).string())
                    .col(ColumnDef::new(Alias::new("permissions")).json_binary())
                    .to_owned(),
            )
            .await?;

        let password_hash = bcrypt::hash("password", 4).unwrap();
        let insert_admin = Query::insert()
            .into_table(Alias::new("core_admins"))
            .columns([Alias::new("login"), Alias::new("password_hash")])
            .values_panic(["admin".into(), password_hash.into()])
            .to_owned();

        manager.exec_stmt(insert_admin).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Alias::new("core_admins")).to_owned())
            .await?;

        Ok(())
    }
}

pub struct UpdateCoreAdmins;

impl MigrationName for UpdateCoreAdmins {
    fn name(&self) -> &str {
        "m20260508_000004_update_core_admins"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for UpdateCoreAdmins {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("core_admins"))
                    .add_column(ColumnDef::new(Alias::new("group_id")).integer().not_null().default(1))
                    .to_owned(),
            )
            .await?;
            
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("core_admins"))
                    .add_column(ColumnDef::new(Alias::new("level")).integer().not_null().default(10))
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("core_admins"))
                    .drop_column(Alias::new("group_id"))
                    .drop_column(Alias::new("level"))
                    .to_owned(),
            )
            .await?;
        Ok(())
    }
}

pub struct AddAdminGroupsAndLevels;

impl MigrationName for AddAdminGroupsAndLevels {
    fn name(&self) -> &str {
        "m20260508_000005_add_admin_groups_and_levels"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for AddAdminGroupsAndLevels {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Alias::new("core_admin_groups"))
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Alias::new("id"))
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Alias::new("name")).string().not_null())
                    .col(ColumnDef::new(Alias::new("level")).integer().not_null())
                    .to_owned(),
            )
            .await?;

        let insert_groups = Query::insert()
            .into_table(Alias::new("core_admin_groups"))
            .columns([Alias::new("name"), Alias::new("level")])
            .values_panic(["Super Administrators".into(), 100.into()])
            .values_panic(["Administrators".into(), 50.into()])
            .values_panic(["Moderators".into(), 20.into()])
            .to_owned();

        manager.exec_stmt(insert_groups).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Alias::new("core_admin_groups")).to_owned())
            .await?;
        Ok(())
    }
}
