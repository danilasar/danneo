use sea_orm_migration::prelude::*;

pub struct ModuleMigrationRegistration {
    pub migration: &'static dyn MigrationTrait,
}

inventory::collect!(ModuleMigrationRegistration);
