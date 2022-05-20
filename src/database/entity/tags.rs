//! SeaORM Entity. Generated by sea-orm-codegen 0.8.0

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "tags")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub name: String,
    #[sea_orm(column_type = "Text", nullable)]
    pub description: Option<String>,
    pub created_by: String,
    pub date_created: DateTimeWithTimeZone,
    pub modified_at: Option<DateTimeWithTimeZone>,
    pub deleted_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::tagged_links::Entity")]
    TaggedLinks,
}

impl Related<super::tagged_links::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TaggedLinks.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}