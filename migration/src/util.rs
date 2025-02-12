use sea_orm_migration::prelude::*;

pub(crate) fn default_table_statement() -> TableCreateStatement {
    TableCreateStatement::new()
        .if_not_exists()
        .col(ColumnDef::new(DefaultColumn::Id)
            .uuid()
            .primary_key()
            .default(Expr::cust("GEN_RANDOM_UUID()"))
            .take())
        .col(ColumnDef::new(DefaultColumn::CreatedAt)
            .timestamp_with_time_zone()
            .default(Expr::current_timestamp())
            .not_null()
            .take())
        .take()
}

#[derive(DeriveIden)]
pub(crate) enum DefaultColumn {
    Id,
    CreatedAt,
}
