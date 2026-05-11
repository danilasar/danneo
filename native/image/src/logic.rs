use super::ThumbnailConfig;
use base64::{Engine as _, engine::general_purpose};
use danneo_sdk::{models::core_images, rpc::RpcContext, state::AppState};
use image::io::Reader as ImageReader;
use sea_orm::{ActiveModelTrait, Set};
use serde_json::{Value, json};
use std::io::Cursor;
use std::sync::Arc;
use uuid::Uuid;

pub async fn process_image(
    db: Arc<sea_orm::DatabaseConnection>,
    original_data: Vec<u8>,
    access_type: &str,
    custom_presets: Option<Vec<ThumbnailConfig>>,
    state: Arc<AppState>,
    get_presets: impl Fn() -> Vec<ThumbnailConfig>,
) -> Result<Value, String> {
    let id = Uuid::new_v4().to_string();
    let format = image::guess_format(&original_data).map_err(|e| e.to_string())?;

    let img = ImageReader::new(Cursor::new(&original_data))
        .with_guessed_format()
        .map_err(|e| e.to_string())?
        .decode()
        .map_err(|e| e.to_string())?;

    let mut thumbnails_map = serde_json::Map::new();
    let rpc_ctx = RpcContext::default();

    let presets = custom_presets.unwrap_or_else(get_presets);

    for preset in presets {
        let thumb = super::ImageModule::apply_strategy(img.clone(), &preset);
        let mut thumb_buf = Cursor::new(Vec::new());
        thumb
            .write_to(&mut thumb_buf, image::ImageFormat::WebP)
            .map_err(|e| e.to_string())?;
        let thumb_data = thumb_buf.into_inner();

        let thumb_path = format!("media/images/{}/{}.webp", id, preset.name);

        state
            .rpc_registry
            .call(
                "storage",
                "upload",
                json!({
                    "path": thumb_path,
                    "content": general_purpose::STANDARD.encode(&thumb_data)
                }),
                rpc_ctx.clone(),
                state.clone(),
            )
            .await
            .map_err(|e| format!("Thumbnail upload failed for {}: {}", preset.name, e))?;

        thumbnails_map.insert(preset.name, json!(thumb_path));
    }

    let extension = match format {
        image::ImageFormat::Jpeg => "jpg",
        image::ImageFormat::Png => "png",
        image::ImageFormat::WebP => "webp",
        _ => "bin",
    };
    let original_path = format!("media/images/{}/original.{}", id, extension);

    state
        .rpc_registry
        .call(
            "storage",
            "upload",
            json!({
                "path": original_path,
                "content": general_purpose::STANDARD.encode(&original_data)
            }),
            rpc_ctx,
            state.clone(),
        )
        .await
        .map_err(|e| format!("Original upload failed: {}", e))?;

    let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
    let model = core_images::ActiveModel {
        id: Set(id.clone()),
        original_path: Set(original_path),
        access_type: Set(access_type.to_string()),
        content_type: Set(format!("{:?}", format)),
        size: Set(original_data.len() as i64),
        thumbnails: Set(Value::Object(thumbnails_map)),
        created_at: Set(now),
        ..Default::default()
    };

    model.insert(db.as_ref()).await.map_err(|e| e.to_string())?;

    Ok(json!({ "id": id, "status": "processed" }))
}
