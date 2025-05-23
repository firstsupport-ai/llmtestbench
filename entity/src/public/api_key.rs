//! `SeaORM` Entity, @generated by sea-orm-codegen 1.1.0

use sea_orm::entity::prelude::*;
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize)]
#[sea_orm(schema_name = "public", table_name = "api_key")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub created_at: DateTimeWithTimeZone,
    pub user_id: Uuid,
    #[sea_orm(column_type = "VarBinary(StringLen::None)")]
    pub key: Vec<u8>,
    pub active_plan_id: Option<Uuid>,
    pub active_plan_from: Option<DateTimeWithTimeZone>,
    pub active_plan_to: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::plan::Entity",
        from = "Column::ActivePlanId",
        to = "super::plan::Column::Id",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    Plan,
    #[sea_orm(has_many = "super::session::Entity")]
    Session,
    #[sea_orm(
        belongs_to = "crate::auth::users::Entity",
        from = "Column::UserId",
        to = "crate::auth::users::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Users,
}

impl Related<super::plan::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Plan.def()
    }
}

impl Related<super::session::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Session.def()
    }
}

impl Related<crate::auth::users::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Users.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
