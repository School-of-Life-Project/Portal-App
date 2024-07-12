#![allow(
    clippy::type_complexity,
    clippy::used_underscore_binding,
    clippy::module_name_repetitions
)]

use std::{
    ffi::{OsStr, OsString},
    path::PathBuf,
    sync::Arc,
};

use futures_util::join;
use serde::{Deserialize, Serialize};
use tauri::{FsScope, Manager};
use tokio::{fs, sync::OnceCell};
use uuid::Uuid;

use crate::data;

use super::{
    state::State, Course, CourseCompletion, CourseMap, CourseProgress, OverallProgress, Settings,
};

#[derive(Serialize, Deserialize, Debug)]
pub struct ErrorWrapper {
    pub(super) message: String,
    pub(super) cause: String,
}

impl ErrorWrapper {
    pub(super) fn new<T>(message: String, inner: &T) -> Self
    where
        T: std::error::Error,
    {
        Self {
            message,
            cause: format!("{inner}"),
        }
    }
}

pub struct StateWrapper {
    inner: Arc<OnceCell<State>>,
}

impl StateWrapper {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(OnceCell::new()),
        }
    }
    pub(super) async fn state(&self) -> Result<&State, ErrorWrapper> {
        self.inner.get_or_try_init(State::new).await.map_err(|e| {
            ErrorWrapper::new("Unable to open application directories".to_string(), &e)
        })
    }
}

#[tauri::command]
pub async fn open_data_dir(state: tauri::State<'_, StateWrapper>) -> Result<(), ErrorWrapper> {
    let state = state.state().await?;

    open::that_detached(&state.data_dir).map_err(|e| {
        ErrorWrapper::new(
            format!("Unable to launch OS opener for {:?}", &state.data_dir),
            &e,
        )
    })?;

    Ok(())
}

#[tauri::command]
pub async fn open_project_issue_tracker(
    state: tauri::State<'_, StateWrapper>,
    data: bool,
) -> Result<(), ErrorWrapper> {
    let state = state.state().await?;

    #[allow(clippy::match_bool)]
    let url = match data {
        true => crate::PROJECT_ISSUE_TRACKER_NEW,
        false => crate::PROJECT_ISSUE_TRACKER,
    };
    open::that_detached(url).map_err(|e| {
        ErrorWrapper::new(
            format!("Unable to launch OS opener for {:?}", &state.data_dir),
            &e,
        )
    })?;

    Ok(())
}

#[tauri::command]
pub async fn open_project_repo(state: tauri::State<'_, StateWrapper>) -> Result<(), ErrorWrapper> {
    let state = state.state().await?;

    open::that_detached(crate::PROJECT_SOURCE_REPO).map_err(|e| {
        ErrorWrapper::new(
            format!("Unable to launch OS opener for {:?}", &state.data_dir),
            &e,
        )
    })?;

    Ok(())
}

#[tauri::command]
pub async fn get_course_maps(
    state: tauri::State<'_, StateWrapper>,
) -> Result<Vec<Result<CourseMap, ErrorWrapper>>, ErrorWrapper> {
    let state = state.state().await?;

    state
        .get_course_maps()
        .await
        .map_err(|e| ErrorWrapper::new("Unable to get CourseMap list".to_string(), &e))
}

#[tauri::command]
pub async fn get_courses(
    state: tauri::State<'_, StateWrapper>,
) -> Result<Vec<Result<(Course, CourseProgress), ErrorWrapper>>, ErrorWrapper> {
    let state = state.state().await?;

    state
        .get_courses()
        .await
        .map_err(|e| ErrorWrapper::new("Unable to get list of Courses".to_string(), &e))
}

#[tauri::command]
pub async fn get_courses_active(
    state: tauri::State<'_, StateWrapper>,
) -> Result<Vec<Result<(Course, CourseProgress), ErrorWrapper>>, ErrorWrapper> {
    let state = state.state().await?;

    state
        .get_courses_active()
        .await
        .map_err(|e| ErrorWrapper::new("Unable to get list of active Courses".to_string(), &e))
}

fn allow_path(scope: &FsScope, path: &mut PathBuf, is_dir: bool) -> Result<(), ErrorWrapper> {
    if is_dir {
        path.push("");
        scope.allow_directory(&path, true)
    } else {
        scope.allow_file(&path)
    }
    .map_err(|e| {
        ErrorWrapper::new(
            format!("Unable to update renderer permissions for path {path:?}"),
            &e,
        )
    })
}

#[tauri::command]
pub async fn get_course(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, StateWrapper>,
    id: Uuid,
) -> Result<(Course, CourseCompletion), ErrorWrapper> {
    let state: &State = state.state().await?;

    let (mut course, progress) = state
        .get_course(id)
        .await
        .map_err(|e| ErrorWrapper::new(format!("Unable to get Course {id}"), &e))?;

    let scope = app_handle.asset_protocol_scope();

    let epub_extension = Some(OsString::from("epub"));
    let epub_extracted_extension = OsString::from("decompressed.epub");
    let tmp_extension = OsString::from("temp");

    for book in &mut course.books {
        let extracted_path = book.file.with_extension(&epub_extracted_extension);

        let (metadata_extract, metadata_full) =
            join!(fs::metadata(&extracted_path), fs::metadata(&book.file));

        if let Ok(metadata) = metadata_full {
            if metadata.is_dir() {
                allow_path(&scope, &mut book.file, true)?;
            } else if book.file.extension().map(OsStr::to_ascii_lowercase) == epub_extension {
                let tmp_path = book.file.with_extension(&tmp_extension);

                data::unzip(book.file.clone(), tmp_path, extracted_path.clone())
                    .await
                    .map_err(|e| {
                        ErrorWrapper::new(format!("Unable to extract {:?}", &book.file), &e)
                    })?;

                book.file = extracted_path;

                allow_path(&scope, &mut book.file, true)?;
            } else {
                allow_path(&scope, &mut book.file, false)?;
            }
        } else if let Ok(metadata) = metadata_extract {
            if metadata.is_dir() {
                book.file = extracted_path;
                allow_path(&scope, &mut book.file, true)?;
            }
        }
    }

    Ok((course, progress))
}

#[tauri::command]
pub async fn set_course_completion(
    state: tauri::State<'_, StateWrapper>,
    id: Uuid,
    data: CourseCompletion,
) -> Result<(), ErrorWrapper> {
    let state = state.state().await?;

    state
        .set_course_completion(id, data)
        .await
        .map_err(|e| ErrorWrapper::new(format!("Unable to update completion for Course {id}"), &e))
}

#[tauri::command]
pub async fn set_course_active_status(
    state: tauri::State<'_, StateWrapper>,
    id: Uuid,
    data: bool,
) -> Result<(), ErrorWrapper> {
    let state = state.state().await?;

    state
        .set_course_active_status(id, data)
        .await
        .map_err(|e| ErrorWrapper::new("Unable to update active Courses".to_string(), &e))
}

#[tauri::command]
pub async fn get_overall_progress(
    state: tauri::State<'_, StateWrapper>,
) -> Result<OverallProgress, ErrorWrapper> {
    let state = state.state().await?;

    state
        .get_overall_progress()
        .await
        .map_err(|e| ErrorWrapper::new("Unable to get total progress data".to_string(), &e))
}

#[tauri::command]
pub async fn get_settings(state: tauri::State<'_, StateWrapper>) -> Result<Settings, ErrorWrapper> {
    let state = state.state().await?;

    state
        .get_settings()
        .await
        .map_err(|e| ErrorWrapper::new("Unable to get Settings".to_string(), &e))
}

#[tauri::command]
pub async fn set_settings(
    state: tauri::State<'_, StateWrapper>,
    data: Settings,
) -> Result<(), ErrorWrapper> {
    let state = state.state().await?;

    state
        .set_settings(&data)
        .await
        .map_err(|e| ErrorWrapper::new("Unable to update Settings".to_string(), &e))
}
