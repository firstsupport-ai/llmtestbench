use sea_orm_migration::prelude::*;

use crate::util::default_table_statement;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(default_table_statement()
                .table(Session::Table)
                .col(ColumnDef::new(Session::FinishedAt).timestamp_with_time_zone())
                .col(ColumnDef::new(Session::Progress).integer().not_null().default(0))
                .take()
        ).await.unwrap();

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                TableDropStatement::new()
                    .table(Session::Table)
                    .take()
            ).await.unwrap();

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Session {
    Table,
    FinishedAt,
    Progress,
}
