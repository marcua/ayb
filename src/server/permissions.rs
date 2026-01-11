use crate::ayb_db::db_interfaces::AybDb;
use crate::ayb_db::models::{
    APIToken, EntityDatabaseSharingLevel, InstantiatedDatabase, InstantiatedEntity,
    PublicSharingLevel,
};
use crate::error::AybError;
use crate::hosted_db::QueryMode;
use actix_web::web;

fn is_owner(authenticated_entity: &InstantiatedEntity, database: &InstantiatedDatabase) -> bool {
    authenticated_entity.id == database.entity_id
}

pub fn can_create_database(
    authenticated_entity: &InstantiatedEntity,
    desired_entity: &InstantiatedEntity,
) -> bool {
    authenticated_entity.id == desired_entity.id
}

pub async fn can_discover_database(
    authenticated_entity: &InstantiatedEntity,
    database: &InstantiatedDatabase,
    ayb_db: &web::Data<Box<dyn AybDb>>,
) -> Result<bool, AybError> {
    let public_sharing_level = PublicSharingLevel::try_from(database.public_sharing_level)?;
    if is_owner(authenticated_entity, database)
        || public_sharing_level == PublicSharingLevel::ReadOnly
        || public_sharing_level == PublicSharingLevel::Fork
    {
        return Ok(true);
    }

    let permission = ayb_db
        .get_entity_database_permission(authenticated_entity, database)
        .await?;
    match permission {
        Some(permission) => match EntityDatabaseSharingLevel::try_from(permission.sharing_level)? {
            EntityDatabaseSharingLevel::Manager
            | EntityDatabaseSharingLevel::ReadWrite
            | EntityDatabaseSharingLevel::ReadOnly => Ok(true),
            _ => Ok(false),
        },
        None => Ok(false),
    }
}

pub async fn can_manage_database(
    authenticated_entity: &InstantiatedEntity,
    database: &InstantiatedDatabase,
    ayb_db: &web::Data<Box<dyn AybDb>>,
) -> Result<bool, AybError> {
    if is_owner(authenticated_entity, database) {
        return Ok(true);
    }

    let permission = ayb_db
        .get_entity_database_permission(authenticated_entity, database)
        .await?;
    match permission {
        Some(permission) => match EntityDatabaseSharingLevel::try_from(permission.sharing_level)? {
            EntityDatabaseSharingLevel::Manager => Ok(true),
            _ => Ok(false),
        },
        None => Ok(false),
    }
}

/// Check if a token can access a specific database.
/// Scoped tokens can only access the database they're scoped to.
/// Unscoped tokens can access any database the user has permission for.
pub fn can_token_access_database(token: &APIToken, database: &InstantiatedDatabase) -> bool {
    match token.database_id {
        Some(token_db_id) => token_db_id == database.id,
        None => true,
    }
}

/// Apply token permission restrictions to a user's permission level.
/// Returns the more restrictive of the two permission levels.
fn apply_token_permission_cap(
    user_permission: Option<QueryMode>,
    token: Option<&APIToken>,
) -> Result<Option<QueryMode>, AybError> {
    let Some(user_perm) = user_permission else {
        return Ok(None);
    };

    let Some(token) = token else {
        return Ok(Some(user_perm));
    };

    let Some(token_perm_level) = token.query_permission_level else {
        return Ok(Some(user_perm)); // No cap on token
    };

    let token_perm = QueryMode::try_from(token_perm_level)?;

    // Return the more restrictive permission
    Ok(match (user_perm, token_perm) {
        (QueryMode::ReadOnly, _) => Some(QueryMode::ReadOnly),
        (_, QueryMode::ReadOnly) => Some(QueryMode::ReadOnly),
        (QueryMode::ReadWrite, QueryMode::ReadWrite) => Some(QueryMode::ReadWrite),
    })
}

pub async fn highest_query_access_level_with_token(
    authenticated_entity: &InstantiatedEntity,
    database: &InstantiatedDatabase,
    token: Option<&APIToken>,
    ayb_db: &web::Data<Box<dyn AybDb>>,
) -> Result<Option<QueryMode>, AybError> {
    // If token is scoped to a different database, deny access
    if let Some(token) = token {
        if !can_token_access_database(token, database) {
            return Ok(None);
        }
    }

    let user_permission = if is_owner(authenticated_entity, database) {
        Some(QueryMode::ReadWrite)
    } else {
        let permission = ayb_db
            .get_entity_database_permission(authenticated_entity, database)
            .await?;
        match permission {
            Some(permission) => {
                match EntityDatabaseSharingLevel::try_from(permission.sharing_level)? {
                    EntityDatabaseSharingLevel::Manager | EntityDatabaseSharingLevel::ReadWrite => {
                        Some(QueryMode::ReadWrite)
                    }
                    EntityDatabaseSharingLevel::ReadOnly => Some(QueryMode::ReadOnly),
                    _ => None,
                }
            }
            None => None,
        }
    };

    // If user has explicit permission, apply token cap
    if user_permission.is_some() {
        return apply_token_permission_cap(user_permission, token);
    }

    // Check public sharing level
    if PublicSharingLevel::try_from(database.public_sharing_level)? == PublicSharingLevel::ReadOnly
    {
        return apply_token_permission_cap(Some(QueryMode::ReadOnly), token);
    }

    Ok(None)
}
