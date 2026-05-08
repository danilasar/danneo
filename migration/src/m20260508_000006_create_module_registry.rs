use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260508_000006_create_module_registry"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.create_table(
            Table::create()
                .table(CorePackages::Table)
                .if_not_exists()
                .col(ColumnDef::new(CorePackages::Id).integer().not_null().auto_increment().primary_key())
                .col(ColumnDef::new(CorePackages::PackageId).string().not_null())
                .col(ColumnDef::new(CorePackages::PackageType).string().not_null())
                .col(ColumnDef::new(CorePackages::RuntimeType).string().not_null())
                .col(ColumnDef::new(CorePackages::Version).string().not_null())
                .col(ColumnDef::new(CorePackages::Path).string().not_null())
                .col(ColumnDef::new(CorePackages::Hash).string().not_null())
                .col(ColumnDef::new(CorePackages::Signature).string())
                .col(ColumnDef::new(CorePackages::Status).string().not_null())
                .col(ColumnDef::new(CorePackages::Manifest).json().not_null())
                .col(ColumnDef::new(CorePackages::UploadedAt).timestamp_with_time_zone().not_null())
                .col(ColumnDef::new(CorePackages::InstalledAt).timestamp_with_time_zone())
                .to_owned()
        ).await?;

        manager.create_table(
            Table::create()
                .table(CoreModules::Table)
                .if_not_exists()
                .col(ColumnDef::new(CoreModules::Id).integer().not_null().auto_increment().primary_key())
                .col(ColumnDef::new(CoreModules::Code).string().not_null().unique_key())
                .col(ColumnDef::new(CoreModules::Name).string().not_null())
                .col(ColumnDef::new(CoreModules::Version).string().not_null())
                .col(ColumnDef::new(CoreModules::PackageId).integer().not_null())
                .col(ColumnDef::new(CoreModules::PackagePath).string().not_null())
                .col(ColumnDef::new(CoreModules::PackageHash).string().not_null())
                .col(ColumnDef::new(CoreModules::PackageSignature).string())
                .col(ColumnDef::new(CoreModules::RuntimeType).string().not_null())
                .col(ColumnDef::new(CoreModules::Enabled).boolean().not_null().default(false))
                .col(ColumnDef::new(CoreModules::Installed).boolean().not_null().default(false))
                .col(ColumnDef::new(CoreModules::Position).integer().not_null().default(0))
                .col(ColumnDef::new(CoreModules::AdminEnabled).boolean().not_null().default(false))
                .col(ColumnDef::new(CoreModules::SitemapEnabled).boolean().not_null().default(false))
                .col(ColumnDef::new(CoreModules::Theme).string())
                .col(ColumnDef::new(CoreModules::Manifest).json().not_null())
                .col(ColumnDef::new(CoreModules::Settings).json())
                .col(ColumnDef::new(CoreModules::Capabilities).json())
                .col(ColumnDef::new(CoreModules::InstalledAt).timestamp_with_time_zone().not_null())
                .col(ColumnDef::new(CoreModules::UpdatedAt).timestamp_with_time_zone().not_null())
                .to_owned()
        ).await?;

        manager.create_table(
            Table::create()
                .table(CoreModuleMigrations::Table)
                .if_not_exists()
                .col(ColumnDef::new(CoreModuleMigrations::ModuleCode).string().not_null())
                .col(ColumnDef::new(CoreModuleMigrations::MigrationName).string().not_null())
                .col(ColumnDef::new(CoreModuleMigrations::Checksum).string().not_null())
                .col(ColumnDef::new(CoreModuleMigrations::AppliedAt).timestamp_with_time_zone().not_null())
                .primary_key(Index::create().col(CoreModuleMigrations::ModuleCode).col(CoreModuleMigrations::MigrationName))
                .to_owned()
        ).await?;

        manager.create_table(
            Table::create()
                .table(CoreModuleRoutes::Table)
                .if_not_exists()
                .col(ColumnDef::new(CoreModuleRoutes::Id).integer().not_null().auto_increment().primary_key())
                .col(ColumnDef::new(CoreModuleRoutes::ModuleCode).string().not_null())
                .col(ColumnDef::new(CoreModuleRoutes::RouteName).string().not_null())
                .col(ColumnDef::new(CoreModuleRoutes::Method).string().not_null())
                .col(ColumnDef::new(CoreModuleRoutes::Path).string().not_null())
                .col(ColumnDef::new(CoreModuleRoutes::Handler).string().not_null())
                .col(ColumnDef::new(CoreModuleRoutes::Permission).string())
                .col(ColumnDef::new(CoreModuleRoutes::Descriptor).json().not_null())
                .to_owned()
        ).await?;

        manager.create_table(
            Table::create()
                .table(CoreModuleEntities::Table)
                .if_not_exists()
                .col(ColumnDef::new(CoreModuleEntities::Id).integer().not_null().auto_increment().primary_key())
                .col(ColumnDef::new(CoreModuleEntities::ModuleCode).string().not_null())
                .col(ColumnDef::new(CoreModuleEntities::EntityName).string().not_null())
                .col(ColumnDef::new(CoreModuleEntities::TableName).string().not_null())
                .col(ColumnDef::new(CoreModuleEntities::Schema).json().not_null())
                .to_owned()
        ).await?;

        manager.create_table(
            Table::create()
                .table(CoreModuleSettings::Table)
                .if_not_exists()
                .col(ColumnDef::new(CoreModuleSettings::ModuleCode).string().not_null())
                .col(ColumnDef::new(CoreModuleSettings::Key).string().not_null())
                .col(ColumnDef::new(CoreModuleSettings::Value).json().not_null())
                .col(ColumnDef::new(CoreModuleSettings::UpdatedAt).timestamp_with_time_zone().not_null())
                .primary_key(Index::create().col(CoreModuleSettings::ModuleCode).col(CoreModuleSettings::Key))
                .to_owned()
        ).await?;

        manager.create_table(
            Table::create()
                .table(CoreBlockDefinitions::Table)
                .if_not_exists()
                .col(ColumnDef::new(CoreBlockDefinitions::Id).integer().not_null().auto_increment().primary_key())
                .col(ColumnDef::new(CoreBlockDefinitions::BlockCode).string().not_null().unique_key())
                .col(ColumnDef::new(CoreBlockDefinitions::ModuleCode).string())
                .col(ColumnDef::new(CoreBlockDefinitions::PackageId).integer().not_null())
                .col(ColumnDef::new(CoreBlockDefinitions::Version).string().not_null())
                .col(ColumnDef::new(CoreBlockDefinitions::Enabled).boolean().not_null().default(true))
                .col(ColumnDef::new(CoreBlockDefinitions::Manifest).json().not_null())
                .col(ColumnDef::new(CoreBlockDefinitions::SettingsSchema).json())
                .col(ColumnDef::new(CoreBlockDefinitions::TemplatePath).string())
                .col(ColumnDef::new(CoreBlockDefinitions::RendererType).string().not_null())
                .to_owned()
        ).await?;

        manager.create_table(
            Table::create()
                .table(CoreBlockTargets::Table)
                .if_not_exists()
                .col(ColumnDef::new(CoreBlockTargets::Id).integer().not_null().auto_increment().primary_key())
                .col(ColumnDef::new(CoreBlockTargets::BlockInstanceId).integer().not_null())
                .col(ColumnDef::new(CoreBlockTargets::ModuleCode).string())
                .col(ColumnDef::new(CoreBlockTargets::RouteName).string())
                .col(ColumnDef::new(CoreBlockTargets::PageKey).string())
                .col(ColumnDef::new(CoreBlockTargets::Mode).string().not_null())
                .to_owned()
        ).await?;

        manager.create_table(
            Table::create()
                .table(CoreBlockAccessGroups::Table)
                .if_not_exists()
                .col(ColumnDef::new(CoreBlockAccessGroups::BlockInstanceId).integer().not_null())
                .col(ColumnDef::new(CoreBlockAccessGroups::GroupId).string().not_null())
                .primary_key(Index::create().col(CoreBlockAccessGroups::BlockInstanceId).col(CoreBlockAccessGroups::GroupId))
                .to_owned()
        ).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(CoreBlockAccessGroups::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(CoreBlockTargets::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(CoreBlockDefinitions::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(CoreModuleSettings::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(CoreModuleEntities::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(CoreModuleRoutes::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(CoreModuleMigrations::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(CoreModules::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(CorePackages::Table).to_owned()).await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum CorePackages {
    Table, Id, PackageId, PackageType, RuntimeType, Version, Path, Hash, Signature, Status, Manifest, UploadedAt, InstalledAt,
}

#[derive(DeriveIden)]
enum CoreModules {
    Table, Id, Code, Name, Version, PackageId, PackagePath, PackageHash, PackageSignature, RuntimeType, Enabled, Installed, Position, AdminEnabled, SitemapEnabled, Theme, Manifest, Settings, Capabilities, InstalledAt, UpdatedAt,
}

#[derive(DeriveIden)]
enum CoreModuleMigrations {
    Table, ModuleCode, MigrationName, Checksum, AppliedAt,
}

#[derive(DeriveIden)]
enum CoreModuleRoutes {
    Table, Id, ModuleCode, RouteName, Method, Path, Handler, Permission, Descriptor,
}

#[derive(DeriveIden)]
enum CoreModuleEntities {
    Table, Id, ModuleCode, EntityName, TableName, Schema,
}

#[derive(DeriveIden)]
enum CoreModuleSettings {
    Table, ModuleCode, Key, Value, UpdatedAt,
}

#[derive(DeriveIden)]
enum CoreBlockDefinitions {
    Table, Id, BlockCode, ModuleCode, PackageId, Version, Enabled, Manifest, SettingsSchema, TemplatePath, RendererType,
}

#[derive(DeriveIden)]
enum CoreBlockTargets {
    Table, Id, BlockInstanceId, ModuleCode, RouteName, PageKey, Mode,
}

#[derive(DeriveIden)]
enum CoreBlockAccessGroups {
    Table, BlockInstanceId, GroupId,
}
