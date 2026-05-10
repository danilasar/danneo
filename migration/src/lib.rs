use sea_orm_migration::prelude::*;

pub mod m20260508_000006_create_module_registry;
pub mod m20260508_000007_create_system_state;
pub mod m20260510_000010_create_lua_migrations_table;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        let mut migrations: Vec<Box<dyn MigrationTrait>> = vec![
            Box::new(m20260508_000006_create_module_registry::Migration),
            Box::new(m20260508_000007_create_system_state::Migration),
            Box::new(m20260510_000010_create_lua_migrations_table::Migration),
        ];

        // Collect all migrations from other crates (modules)
        for reg in inventory::iter::<ModuleMigrationRegistration> {
            migrations.push(Box::new(MigrationWrapper { inner: reg.migration }));
        }

        // Keep them sorted by name for consistency
        migrations.sort_by(|a, b| a.name().cmp(b.name()));
        migrations
    }
}

pub struct ModuleMigrationRegistration {
    pub migration: &'static dyn MigrationTrait,
}

inventory::collect!(ModuleMigrationRegistration);

struct MigrationWrapper {
    inner: &'static dyn MigrationTrait,
}

impl MigrationName for MigrationWrapper {
    fn name(&self) -> &str {
        self.inner.name()
    }
}

#[async_trait::async_trait]
impl MigrationTrait for MigrationWrapper {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        self.inner.up(manager).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        self.inner.down(manager).await
    }
}
