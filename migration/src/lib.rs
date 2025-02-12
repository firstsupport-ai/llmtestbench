pub use sea_orm_migration::prelude::*;

mod util;
mod m20250211_101302_init;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20250211_101302_init::Migration),
        ]
    }
}
