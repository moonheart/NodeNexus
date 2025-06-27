use sea_orm::{
    ActiveModelTrait, DbConn, DbErr, EntityTrait, Set, ModelTrait,
    ColumnTrait, QueryFilter,
};
use crate::db::entities::{command_script, prelude::CommandScript};
use crate::db::entities::command_script::ScriptLanguage;

#[derive(Debug, thiserror::Error)]
pub enum CommandScriptError {
    #[error("Database error: {0}")]
    DbErr(#[from] DbErr),
    #[error("Script not found: {0}")]
    NotFound(i32),
    #[error("Unauthorized operation")]
    Unauthorized,
    #[error("A script with the name '{0}' already exists.")]
    DuplicateName(String),
}

pub struct CommandScriptService;

impl CommandScriptService {
    pub async fn create_script(
        db: &DbConn,
        user_id: i32,
        name: String,
        description: Option<String>,
        language: ScriptLanguage,
        script_content: String,
        working_directory: String,
    ) -> Result<command_script::Model, CommandScriptError> {
        // Check for duplicate name for the same user
        if CommandScript::find()
            .filter(command_script::Column::UserId.eq(user_id))
            .filter(command_script::Column::Name.eq(&name))
            .one(db)
            .await?
            .is_some()
        {
            return Err(CommandScriptError::DuplicateName(name));
        }

        let new_script = command_script::ActiveModel {
            user_id: Set(user_id),
            name: Set(name),
            description: Set(description),
            language: Set(language),
            script_content: Set(script_content),
            working_directory: Set(working_directory),
            ..Default::default()
        };

        Ok(new_script.insert(db).await?)
    }

    pub async fn get_scripts_by_user(
        db: &DbConn,
        user_id: i32,
    ) -> Result<Vec<command_script::Model>, CommandScriptError> {
        let query = CommandScript::find().filter(command_script::Column::UserId.eq(user_id));

        Ok(query.all(db).await?)
    }

    pub async fn get_script_by_id(
        db: &DbConn,
        script_id: i32,
        user_id: i32,
    ) -> Result<command_script::Model, CommandScriptError> {
        let script = CommandScript::find_by_id(script_id)
            .filter(command_script::Column::UserId.eq(user_id))
            .one(db)
            .await?
            .ok_or(CommandScriptError::NotFound(script_id))?;
        Ok(script)
    }

    pub async fn update_script(
        db: &DbConn,
        script_id: i32,
        user_id: i32,
        name: String,
        description: Option<String>,
        language: ScriptLanguage,
        script_content: String,
        working_directory: String,
    ) -> Result<command_script::Model, CommandScriptError> {
        let script = CommandScript::find_by_id(script_id)
            .filter(command_script::Column::UserId.eq(user_id))
            .one(db)
            .await?
            .ok_or(CommandScriptError::NotFound(script_id))?;

        let mut active_script: command_script::ActiveModel = script.into();
        active_script.name = Set(name);
        active_script.description = Set(description);
        active_script.language = Set(language);
        active_script.script_content = Set(script_content);
        active_script.working_directory = Set(working_directory);
        active_script.updated_at = Set(chrono::Utc::now().into());

        Ok(active_script.update(db).await?)
    }

    pub async fn delete_script(
        db: &DbConn,
        script_id: i32,
        user_id: i32,
    ) -> Result<(), CommandScriptError> {
        let script = CommandScript::find_by_id(script_id)
            .filter(command_script::Column::UserId.eq(user_id))
            .one(db)
            .await?
            .ok_or(CommandScriptError::NotFound(script_id))?;

        script.delete(db).await?;
        Ok(())
    }
}