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

/// Check if a token is scoped to a specific database
pub fn is_token_scoped(token: &APIToken) -> bool {
    token.database_id.is_some()
}

/// Check if a scoped token can access a specific database
pub fn can_scoped_token_access_database(token: &APIToken, database: &InstantiatedDatabase) -> bool {
    match token.database_id {
        Some(token_db_id) => token_db_id == database.id,
        None => true, // Unscoped tokens can access any database the user has permission for
    }
}

/// Apply token permission restrictions to a user's permission level.
/// Returns the more restrictive of the two permission levels.
fn apply_token_permission_cap(
    user_permission: Option<QueryMode>,
    token: Option<&APIToken>,
) -> Option<QueryMode> {
    let user_perm = user_permission?;

    let Some(token) = token else {
        return Some(user_perm);
    };

    let Some(token_perm_level) = token.query_permission_level else {
        return Some(user_perm); // No cap on token
    };

    // Token permission level uses same values as QueryMode
    let token_perm = match QueryMode::try_from(token_perm_level) {
        Ok(perm) => perm,
        Err(_) => return Some(user_perm), // Invalid permission level, use user permission
    };

    // Return the more restrictive permission
    match (user_perm, token_perm) {
        (QueryMode::ReadOnly, _) => Some(QueryMode::ReadOnly),
        (_, QueryMode::ReadOnly) => Some(QueryMode::ReadOnly),
        (QueryMode::ReadWrite, QueryMode::ReadWrite) => Some(QueryMode::ReadWrite),
    }
}

pub async fn highest_query_access_level(
    authenticated_entity: &InstantiatedEntity,
    database: &InstantiatedDatabase,
    ayb_db: &web::Data<Box<dyn AybDb>>,
) -> Result<Option<QueryMode>, AybError> {
    highest_query_access_level_with_token(authenticated_entity, database, None, ayb_db).await
}

pub async fn highest_query_access_level_with_token(
    authenticated_entity: &InstantiatedEntity,
    database: &InstantiatedDatabase,
    token: Option<&APIToken>,
    ayb_db: &web::Data<Box<dyn AybDb>>,
) -> Result<Option<QueryMode>, AybError> {
    // If token is scoped to a different database, deny access
    if let Some(token) = token {
        if !can_scoped_token_access_database(token, database) {
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
        return Ok(apply_token_permission_cap(user_permission, token));
    }

    // Check public sharing level
    if PublicSharingLevel::try_from(database.public_sharing_level)? == PublicSharingLevel::ReadOnly
    {
        return Ok(apply_token_permission_cap(Some(QueryMode::ReadOnly), token));
    }

    Ok(None)
}
