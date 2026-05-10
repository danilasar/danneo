use sea_orm_migration::prelude::*;

pub struct CreateImageTable;

impl MigrationName for CreateImageTable {
    fn name(&self) -> &str {
        "m20260510_000008_create_image_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for CreateImageTable {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Alias::new("core_images"))
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Alias::new("id"))
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Alias::new("original_path")).string().not_null())
                    .col(ColumnDef::new(Alias::new("access_type")).string().not_null())
                    .col(ColumnDef::new(Alias::new("content_type")).string().not_null())
                    .col(ColumnDef::new(Alias::new("size")).big_integer().not_null())
                    .col(ColumnDef::new(Alias::new("owner_id")).integer())
                    .col(
                        ColumnDef::new(Alias::new("created_at"))
                            .date_time()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Alias::new("core_images")).to_owned())
            .await?;

        Ok(())
    }
}

pub struct UpgradeImageTable;

impl MigrationName for UpgradeImageTable {
    fn name(&self) -> &str {
        "m20260510_000009_upgrade_image_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for UpgradeImageTable {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("core_images"))
                    .add_column(ColumnDef::new(Alias::new("thumbnails")).json_binary().not_null().default("{}"))
                    .to_owned(),
            )
            .await?;
        
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("core_images"))
                    .drop_column(Alias::new("thumbnails"))
                    .to_owned(),
            )
            .await?;
        Ok(())
    }
}
