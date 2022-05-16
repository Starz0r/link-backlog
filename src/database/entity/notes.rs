//! SeaORM Entity. Generated by sea-orm-codegen 0.8.0

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "notes")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub link_id: String,
    #[sea_orm(column_type = "Text", nullable)]
    pub body: Option<String>,
    pub private: bool,
    pub created_by: String,
    pub date_created: DateTimeWithTimeZone,
    pub modified_at: Option<DateTimeWithTimeZone>,
    pub deleted_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::links::Entity",
        from = "Column::LinkId",
        to = "super::links::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    Links,
}

impl Related<super::links::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Links.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
