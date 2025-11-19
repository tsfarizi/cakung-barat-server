use actix_multipart::Multipart;
use actix_web::HttpResponse;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use log::{error};
use sanitize_filename::sanitize;

use crate::{ErrorResponse, posting::models::CreatePostingRequest};

#[derive(Debug, Serialize, Deserialize)]
pub struct ParsedMultipartData {
    pub title: String,
    pub category: String,
    pub excerpt: String,
    pub files_data: Vec<(Vec<u8>, String)>,
}

#[derive(Debug, thiserror::Error)]
pub enum MultipartParseError {
    #[error("Multipart field error: {0}")]
    FieldError(String),
    #[error("Invalid metadata: {0}")]
    MetadataError(String),
    #[error("IO error: {0}")]
    IoError(String),
    #[error("Invalid UTF-8 data: {0}")]
    Utf8Error(String),
    #[error("Serialization error: {0}")]
    SerializationError(String),
}

impl From<MultipartParseError> for HttpResponse {
    fn from(error: MultipartParseError) -> Self {
        match error {
            MultipartParseError::MetadataError(_) | 
            MultipartParseError::Utf8Error(_) | 
            MultipartParseError::SerializationError(_) => {
                HttpResponse::BadRequest()
                    .json(ErrorResponse::bad_request(&format!("{}", error)))
            },
            _ => HttpResponse::InternalServerError()
                .json(ErrorResponse::internal_error(&format!("{}", error))),
        }
    }
}

pub struct MultipartParser;

impl MultipartParser {
    pub async fn parse_posting_multipart(
        mut multipart: Multipart,
    ) -> Result<ParsedMultipartData, MultipartParseError> {
        let mut title = String::new();
        let mut category = String::new();
        let mut excerpt = String::new();
        let mut files_data: Vec<(Vec<u8>, String)> = Vec::new();

        while let Some(item) = multipart.next().await {
            let mut field = item.map_err(|e| MultipartParseError::FieldError(e.to_string()))?;
            let content_disposition = field.content_disposition()
                .ok_or_else(|| MultipartParseError::FieldError("Content disposition not found".to_string()))?;
            let name = content_disposition.get_name()
                .ok_or_else(|| MultipartParseError::FieldError("Field name not found".to_string()))?;

            let maybe_filename = content_disposition.get_filename().map(|s| s.to_string());

            if name == "metadata" {
                let mut buffer = Vec::new();
                while let Some(chunk) = field.next().await {
                    let data_chunk = chunk.map_err(|e| MultipartParseError::IoError(e.to_string()))?;
                    buffer.extend_from_slice(&data_chunk);
                }
                
                let metadata_str = String::from_utf8(buffer)
                    .map_err(|e| MultipartParseError::Utf8Error(e.to_string()))?;
                
                let metadata: CreatePostingRequest = serde_json::from_str(&metadata_str)
                    .map_err(|e| MultipartParseError::SerializationError(e.to_string()))?;
                
                title = metadata.title;
                category = metadata.category;
                excerpt = metadata.excerpt;
            } else if name.starts_with("file") {
  
                let mut file_buffer = Vec::new();
                while let Some(chunk) = field.next().await {
                    let data_chunk = chunk.map_err(|e| MultipartParseError::IoError(e.to_string()))?;
                    file_buffer.extend_from_slice(&data_chunk);
                }

                let original_filename = match maybe_filename {
                    Some(fname) => fname,
                    None => format!("file_{}.dat", files_data.len()),
                };

                files_data.push((file_buffer, original_filename));
            }
        }

        Ok(ParsedMultipartData {
            title,
            category,
            excerpt,
            files_data,
        })
    }

    pub async fn parse_asset_multipart(
        mut multipart: Multipart,
    ) -> Result<(Vec<u8>, String, Option<String>, Option<Uuid>, Vec<String>), MultipartParseError> {
        let mut file_data = Vec::new();
        let mut original_filename = String::new();
        let mut asset_name: Option<String> = None;
        let mut posting_id: Option<Uuid> = None;
        let mut folder_names: Vec<String> = Vec::new();

        while let Some(item) = multipart.next().await {
            let mut field = item.map_err(|e| MultipartParseError::FieldError(e.to_string()))?;
            let content_disposition = field.content_disposition()
                .ok_or_else(|| MultipartParseError::FieldError("Content disposition not found".to_string()))?;
            let field_name = content_disposition.get_name()
                .ok_or_else(|| MultipartParseError::FieldError("Field name not found".to_string()))?;

            match field_name {
                "file" => {
                    let filename = content_disposition.get_filename()
                        .ok_or_else(|| MultipartParseError::FieldError("No filename in file field".to_string()))?;
                    
                    original_filename = sanitize(&filename).to_string();

                    while let Some(chunk) = field.next().await {
                        let chunk_data = chunk.map_err(|e| MultipartParseError::IoError(e.to_string()))?;
                        file_data.extend_from_slice(&chunk_data);
                    }
                },
                "posting_id" => {
                    let mut bytes = Vec::new();
                    while let Some(chunk) = field.next().await {
                        let chunk_data = chunk.map_err(|e| MultipartParseError::IoError(e.to_string()))?;
                        bytes.extend_from_slice(&chunk_data);
                    }
                    let value = String::from_utf8(bytes)
                        .map_err(|e| MultipartParseError::Utf8Error(e.to_string()))?;
                    posting_id = Uuid::parse_str(&value)
                        .map_err(|_| MultipartParseError::FieldError("Invalid posting ID format".to_string())).ok();
                },
                "folders" => {
                    let mut bytes = Vec::new();
                    while let Some(chunk) = field.next().await {
                        let chunk_data = chunk.map_err(|e| MultipartParseError::IoError(e.to_string()))?;
                        bytes.extend_from_slice(&chunk_data);
                    }
                    let value = String::from_utf8(bytes)
                        .map_err(|e| MultipartParseError::Utf8Error(e.to_string()))?;

                    folder_names = value
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                },
                "name" => {
                    let mut bytes = Vec::new();
                    while let Some(chunk) = field.next().await {
                        let chunk_data = chunk.map_err(|e| MultipartParseError::IoError(e.to_string()))?;
                        bytes.extend_from_slice(&chunk_data);
                    }
                    let value = String::from_utf8(bytes)
                        .map_err(|e| MultipartParseError::Utf8Error(e.to_string()))?;
                    asset_name = Some(value);
                },
                _ => {
                    continue;
                }
            }
        }

        if file_data.is_empty() {
            return Err(MultipartParseError::FieldError("No file data found in multipart payload".to_string()));
        }

        Ok((file_data, original_filename, asset_name, posting_id, folder_names))
    }
}