use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::SqlitePool;
use tauri::State;

use crate::error::AppError;
use crate::models::schedule_event::{
    CreateScheduleEventInput, ScheduleEvent, UpdateScheduleEventInput,
};
use crate::services;

#[derive(Debug, Deserialize, Type)]
pub struct ListScheduleEventsInput {
    pub class_id: String,
}

#[derive(Debug, Serialize, Deserialize, Type)]
pub struct DeleteScheduleEventInput {
    pub id: String,
}

#[derive(Debug, Serialize, Deserialize, Type)]
pub struct DeleteScheduleEventResponse {
    pub success: bool,
}

#[tauri::command]
#[specta::specta]
pub async fn list_schedule_events(
    pool: State<'_, SqlitePool>,
    input: ListScheduleEventsInput,
) -> Result<Vec<ScheduleEvent>, AppError> {
    services::schedule_event::ScheduleEventService::list_by_class(&pool, &input.class_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn get_schedule_event(
    pool: State<'_, SqlitePool>,
    id: String,
) -> Result<ScheduleEvent, AppError> {
    services::schedule_event::ScheduleEventService::get_by_id(&pool, &id).await
}

#[tauri::command]
#[specta::specta]
pub async fn create_schedule_event(
    pool: State<'_, SqlitePool>,
    input: CreateScheduleEventInput,
) -> Result<ScheduleEvent, AppError> {
    services::schedule_event::ScheduleEventService::create(&pool, input).await
}

#[tauri::command]
#[specta::specta]
pub async fn update_schedule_event(
    pool: State<'_, SqlitePool>,
    input: UpdateScheduleEventInput,
) -> Result<ScheduleEvent, AppError> {
    services::schedule_event::ScheduleEventService::update(&pool, input).await
}

#[tauri::command]
#[specta::specta]
pub async fn delete_schedule_event(
    pool: State<'_, SqlitePool>,
    input: DeleteScheduleEventInput,
) -> Result<DeleteScheduleEventResponse, AppError> {
    services::schedule_event::ScheduleEventService::delete(&pool, &input.id).await?;
    Ok(DeleteScheduleEventResponse { success: true })
}
