use sea_orm_migration::prelude::*;

use crate::util::{default_table_statement, DefaultColumn};

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
                .col(ColumnDef::new(Session::RecordCount).integer().not_null())
                .col(ColumnDef::new(Session::ApiKeyId).uuid().not_null())
                .take()
            ).await.unwrap();
        
        manager
            .create_table(default_table_statement()
                .table(ApiKey::Table)
                .col(ColumnDef::new(ApiKey::UserId).uuid().not_null())
                .col(ColumnDef::new(ApiKey::Key).binary_len(16).not_null())
                .col(ColumnDef::new(ApiKey::ActivePlanId).uuid())
                .col(ColumnDef::new(ApiKey::ActivePlanFrom).timestamp_with_time_zone())
                .col(ColumnDef::new(ApiKey::ActivePlanTo).timestamp_with_time_zone())
                .take()
            ).await.unwrap();
        
        manager
            .create_table(default_table_statement()
                .table(Plan::Table)
                .col(ColumnDef::new(Plan::Name).string().not_null())
                .col(ColumnDef::new(Plan::ProductId).string().not_null())
                .col(ColumnDef::new(Plan::PurchaseUrl).string().not_null())
                .col(ColumnDef::new(Plan::Quota).integer().not_null())
                .take()
            ).await.unwrap();
        
        manager
            .create_foreign_key(ForeignKeyCreateStatement::new()
                .from(ApiKey::Table, ApiKey::ActivePlanId)
                .to(Plan::Table, DefaultColumn::Id)
                .on_delete(ForeignKeyAction::SetNull)
                .on_update(ForeignKeyAction::Cascade)
                .take()
            ).await.unwrap();

        manager
            .create_foreign_key(ForeignKeyCreateStatement::new()
                .from(Session::Table, Session::ApiKeyId)
                .to(ApiKey::Table, DefaultColumn::Id)
                .on_delete(ForeignKeyAction::Cascade)
                .on_update(ForeignKeyAction::Cascade)
                .take()
            ).await.unwrap();
        
        manager
            .create_foreign_key(ForeignKeyCreateStatement::new()
                .from(ApiKey::Table, ApiKey::UserId)
                .to((Users::Schema, Users::Table), Users::Id)
                .on_delete(ForeignKeyAction::Cascade)
                .on_update(ForeignKeyAction::Cascade)
                .take()
            ).await.unwrap();
        
        manager
            .exec_stmt(Query::insert()
                .into_table(Plan::Table)
                .columns([Plan::Name, Plan::ProductId, Plan::PurchaseUrl, Plan::Quota])
                .values_panic(["Starter Plan".into(), "466184".into(), "https://terrydjony.lemonsqueezy.com/buy/86f66fca-3804-4e0c-888c-dbac52b2025d".into(), 10_000.into()])
                .to_owned()
            ).await.unwrap();

        manager
            .exec_stmt(Query::insert()
                .into_table(Plan::Table)
                .columns([Plan::Name, Plan::ProductId, Plan::PurchaseUrl, Plan::Quota])
                .values_panic(["Growth Plan".into(), "466326".into(), "https://terrydjony.lemonsqueezy.com/buy/1707a8a2-36c8-4322-ad0d-1193a72b2cd8".into(), 35_000.into()])
                .to_owned()
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
        
        manager
            .drop_table(
                TableDropStatement::new()
                    .table(ApiKey::Table)
                    .take()
            ).await.unwrap();
        
        manager
            .drop_table(
                TableDropStatement::new()
                    .table(Plan::Table)
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
    RecordCount,
    ApiKeyId,
}

#[derive(DeriveIden)]
enum ApiKey {
    Table,
    UserId,
    Key,
    ActivePlanId,
    ActivePlanFrom,
    ActivePlanTo,
}

#[derive(DeriveIden)]
enum Users {
    #[sea_orm(iden = "auth")]
    Schema,
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Plan {
    Table,
    Name,
    ProductId,
    PurchaseUrl,
    Quota,
}
